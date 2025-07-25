use std::fs::File;
use std::path::Path;
use std::process::Stdio;

use error_stack::{Report, ResultExt};

use crate::error::{Error, Result};

/// Launch a detached process with proper setup
pub fn launch_detached_process(
    cmd: &std::process::Command,
    working_dir: &Path,
    log_file: File,
    process_name: &str,
    operation: &str,
) -> Result<u32> {
    // Clone the log file handle for stderr
    let log_file_for_stderr = log_file
        .try_clone()
        .change_context(Error::ProcessManagement(
            "Failed to clone log file handle".to_string(),
        ))
        .attach_printable(format!("Process: {process_name}, Operation: {operation}"))?;

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

    // Spawn the process
    tracing::debug!("Preparing to spawn process: {process_name}");
    tracing::debug!("Command: {:?}", new_cmd);
    tracing::debug!("Working directory: {}", working_dir.display());

    match new_cmd.spawn() {
        Ok(child) => {
            // Get the PID
            let pid = child.id();

            tracing::debug!("Process spawned successfully: {process_name} (PID: {pid})");

            Ok(pid)
        }
        Err(e) => {
            tracing::error!("Failed to spawn process {process_name}: {e}");
            Err(Report::new(e)
                .change_context(Error::ProcessManagement(
                    "Failed to spawn process".to_string(),
                ))
                .attach_printable(format!("Process: {process_name}"))
                .attach_printable(format!("Operation: {operation}"))
                .attach_printable(format!("Working directory: {}", working_dir.display())))?
        }
    }
}
