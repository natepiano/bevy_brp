use std::fs::File;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use error_stack::Report;
use error_stack::ResultExt;
use netstat2::AddressFamilyFlags;
use netstat2::ProtocolFlags;
use netstat2::ProtocolSocketInfo;
use sysinfo::Process;

use super::constants::APP_EXTENSION_SUFFIX;
use super::constants::BIN_EXTENSION_SUFFIX;
use super::constants::EXE_EXTENSION_SUFFIX;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;

/// Launch a detached process with proper setup
pub(super) fn launch_detached_process(
    command: &Command,
    working_dir: &Path,
    log_file: File,
    process_name: &str,
) -> Result<u32> {
    // Clone the log file handle for stderr
    let log_file_for_stderr = log_file
        .try_clone()
        .change_context(Error::ProcessManagement(
            "Failed to clone log file handle".to_string(),
        ))
        .attach(format!("Process: {process_name}, Operation: launch"))?;

    // Create a new command from the provided one
    let mut new_command = std::process::Command::new(command.get_program());

    // Copy args
    for arg in command.get_args() {
        new_command.arg(arg);
    }

    // Set working directory and CARGO_MANIFEST_DIR
    new_command
        .current_dir(working_dir)
        .env("CARGO_MANIFEST_DIR", working_dir);

    // Copy other environment variables
    for (key, value) in command.get_envs() {
        if let Some(value) = value {
            new_command.env(key, value);
        }
    }

    // Set stdio
    new_command
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_for_stderr));

    // Create new process group for true detachment (Unix only)
    #[cfg(unix)]
    new_command.process_group(0);

    // Spawn the process
    tracing::debug!("Preparing to spawn process: {process_name}");
    tracing::debug!("Command: {:?}", new_command);
    tracing::debug!("Working directory: {}", working_dir.display());

    match new_command.spawn() {
        Ok(mut child) => {
            // Get the PID
            let process_id = child.id();

            tracing::debug!("Process spawned successfully: {process_name} (PID: {process_id})");

            // Spawn a background thread to reap the child when it exits
            // This prevents zombie processes
            std::thread::spawn(move || match child.wait() {
                Ok(status) => {
                    tracing::debug!("Child process {process_id} exited with status: {status:?}");
                },
                Err(e) => {
                    tracing::warn!("Failed to wait for child process {process_id}: {e}");
                },
            });

            Ok(process_id)
        },
        Err(e) => {
            tracing::error!("Failed to spawn process {process_name}: {e}");
            Err(Report::new(e)
                .change_context(Error::ProcessManagement(
                    "Failed to spawn process".to_string(),
                ))
                .attach(format!("Process: {process_name}"))
                .attach(format!("Working directory: {}", working_dir.display())))?
        },
    }
}

/// Normalize a process name or binary path for robust matching.
///
/// Strips directory paths, removes common executable extensions (.exe, .app, .bin),
/// and lowercases the result to enable case-insensitive comparison.
pub fn normalize_process_name(name: &str) -> String {
    let name = name.to_lowercase();

    // Remove path components - get just the base name
    let base_name = name.split(['/', '\\']).next_back().unwrap_or(&name);

    // Remove common executable extensions
    base_name
        .strip_suffix(EXE_EXTENSION_SUFFIX)
        .or_else(|| base_name.strip_suffix(APP_EXTENSION_SUFFIX))
        .or_else(|| base_name.strip_suffix(BIN_EXTENSION_SUFFIX))
        .unwrap_or(base_name)
        .to_string()
}

/// Check if a process exactly matches a target app name.
///
/// Checks the command path first since it is not subject to
/// OS-level process name truncation (macOS truncates to 16 chars, Linux to 15).
/// Falls back to the kernel-reported process name when the command path is unavailable.
pub fn process_matches_name_exact(process: &Process, target: &str) -> bool {
    let normalized_target = normalize_process_name(target);

    // Prefer the full binary path because it is not subject to kernel truncation.
    if let Some(command_path) = process.cmd().first()
        && normalize_process_name(&command_path.to_string_lossy()) == normalized_target
    {
        return true;
    }

    // Fall back to process name (may be truncated on macOS/Linux)
    let process_name = process.name().to_string_lossy();
    normalize_process_name(&process_name) == normalized_target
}

/// Get the PID for a process listening on the specified port
pub fn get_pid_for_port(port: Port) -> Option<u32> {
    let address_family_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let protocol_flags = ProtocolFlags::TCP;

    netstat2::get_sockets_info(address_family_flags, protocol_flags)
        .ok()?
        .into_iter()
        .find_map(|socket_info| {
            if let ProtocolSocketInfo::Tcp(tcp_socket_info) = socket_info.protocol_socket_info
                && tcp_socket_info.local_port == *port
            {
                return socket_info.associated_pids.first().copied();
            }
            None
        })
}
