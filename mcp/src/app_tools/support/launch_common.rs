use std::path::{Path, PathBuf};
use std::process::Command;

use rmcp::Error as McpError;

use crate::brp_tools::constants::BRP_PORT_ENV_VAR;
use crate::error::{Error, report_to_mcp_error};

/// Validates and extracts the manifest directory from a manifest path
pub fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path, McpError> {
    manifest_path.parent().ok_or_else(|| -> McpError {
        report_to_mcp_error(
            &error_stack::Report::new(Error::Configuration("Invalid manifest path".to_string()))
                .attach_printable("No parent directory found")
                .attach_printable(format!("Path: {}", manifest_path.display())),
        )
    })
}

/// Sets BRP-related environment variables on a command
///
/// Currently sets:
/// - `BRP_PORT`: When a port is provided, sets this environment variable for `bevy_brp_extras` to
///   read
pub fn set_brp_env_vars(cmd: &mut Command, port: Option<u16>) {
    if let Some(port) = port {
        cmd.env(BRP_PORT_ENV_VAR, port.to_string());
    }
}

/// Setup logging for launch operations and return log file handles
pub fn setup_launch_logging(
    name: &str,
    name_type: &str, // "App" or "Example"
    profile: &str,
    command_or_binary: &Path,
    manifest_dir: &Path,
    port: Option<u16>,
    extra_log_info: Option<&str>,
) -> Result<(PathBuf, std::fs::File), McpError> {
    use super::logging;

    // Create log file
    let (log_file_path, _) = logging::create_log_file(
        name,
        name_type,
        profile,
        command_or_binary,
        manifest_dir,
        port,
    )?;

    // Add extra info to log file if provided
    if let Some(extra_info) = extra_log_info {
        logging::append_to_log_file(&log_file_path, &format!("{extra_info}\n"))?;
    }

    // Open log file for stdout/stderr redirection
    let log_file_for_redirect = logging::open_log_file_for_redirect(&log_file_path)?;

    Ok((log_file_path, log_file_for_redirect))
}

/// Build cargo command for running examples
pub fn build_cargo_example_command(
    example_name: &str,
    profile: &str,
    port: Option<u16>,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example").arg(example_name);

    // Add profile flag if release
    if profile == "release" {
        cmd.arg("--release");
    }

    // Set BRP-related environment variables
    set_brp_env_vars(&mut cmd, port);

    cmd
}

/// Build command for running app binaries
pub fn build_app_command(binary_path: &Path, port: Option<u16>) -> Command {
    let mut cmd = Command::new(binary_path);
    set_brp_env_vars(&mut cmd, port);
    cmd
}
