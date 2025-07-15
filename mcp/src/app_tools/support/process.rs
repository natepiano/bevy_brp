use std::fs::File;
// use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Stdio;

use rmcp::Error as McpError;

use crate::error::{Error, report_to_mcp_error};

/// Launch a detached process with proper setup
pub fn launch_detached_process(
    cmd: &std::process::Command,
    working_dir: &Path,
    log_file: File,
    process_name: &str,
    operation: &str,
) -> Result<u32, McpError> {
    // Clone the log file handle for stderr
    let log_file_for_stderr = log_file.try_clone().map_err(|e| {
        let error_report = error_stack::Report::new(e)
            .change_context(Error::ProcessManagement(
                "Failed to clone log file handle".to_string(),
            ))
            .attach_printable(format!("Process: {process_name}, Operation: {operation}"));
        report_to_mcp_error(&error_report)
    })?;

    // Set FD_CLOEXEC on log file descriptors to prevent interference between concurrent spawns
    // let log_fd = log_file.as_raw_fd();
    // let stderr_fd = log_file_for_stderr.as_raw_fd();

    // tracing::debug!(
    //     "Setting up file descriptors for {process_name}: stdout_fd={log_fd},
    // stderr_fd={stderr_fd}" );

    // unsafe {
    //     if libc::fcntl(log_fd, libc::F_SETFD, libc::FD_CLOEXEC) == -1 {
    //         tracing::warn!(
    //             "Failed to set FD_CLOEXEC on stdout log file descriptor for {process_name}"
    //         );
    //     }
    //     if libc::fcntl(stderr_fd, libc::F_SETFD, libc::FD_CLOEXEC) == -1 {
    //         tracing::warn!(
    //             "Failed to set FD_CLOEXEC on stderr log file descriptor for {process_name}"
    //         );
    //     }
    // }

    // tracing::debug!("FD_CLOEXEC flags set successfully for {process_name}");

    // Create a new command from the provided one
    let mut new_cmd = std::process::Command::new(cmd.get_program());

    // Copy args
    for arg in cmd.get_args() {
        new_cmd.arg(arg);
    }

    // Set working directory and CARGO_MANIFEST_DIR
    new_cmd
        .current_dir(working_dir)
        .env("CARGO_MANIFEST_DIR", working_dir);

    // Copy other environment variables
    for (key, value) in cmd.get_envs() {
        if let Some(value) = value {
            new_cmd.env(key, value);
        }
    }

    // Set stdio
    new_cmd
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_for_stderr));

    // UNIX-specific: Create a new process group to detach from parent
    #[cfg(unix)]
    unsafe {
        new_cmd.pre_exec(|| {
            // Create new session for full process detachment
            libc::setsid();
            Ok(())
        });
    }

    // Spawn the process
    tracing::debug!("Preparing to spawn process: {process_name}");
    tracing::debug!("Command: {:?}", new_cmd);
    tracing::debug!("Working directory: {}", working_dir.display());

    match new_cmd.spawn() {
        Ok(child) => {
            // Get the PID
            let pid = child.id();

            tracing::debug!("Process spawned successfully: {process_name} (PID: {pid})");

            // The process is now detached and will continue running
            // independently even after this program exits
            Ok(pid)
        }
        Err(e) => {
            tracing::error!("Failed to spawn process {process_name}: {e}");
            let error_report = error_stack::Report::new(e)
                .change_context(Error::ProcessManagement(
                    "Failed to spawn process".to_string(),
                ))
                .attach_printable(format!("Process: {process_name}"))
                .attach_printable(format!("Operation: {operation}"))
                .attach_printable(format!("Working directory: {}", working_dir.display()));
            Err(report_to_mcp_error(&error_report))
        }
    }
}
