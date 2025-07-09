use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

use crate::brp_tools::constants::BRP_PORT_ENV_VAR;
use crate::error::{Error, report_to_mcp_error};
use crate::support::response;
use crate::support::response::ResponseBuilder;

/// Parameters for building a launch success response
pub struct LaunchResponseParams<'a> {
    pub name:            &'a str,
    pub name_field:      &'a str, // "app_name" or "example_name"
    pub pid:             u32,
    pub manifest_dir:    &'a Path,
    pub profile:         &'a str,
    pub log_file_path:   &'a Path,
    pub additional_data: Option<Value>,
    pub workspace_root:  Option<&'a PathBuf>,
    pub launch_start:    Instant,
    pub launch_end:      Instant,
}

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

/// Creates a success response with common fields and workspace info
///
/// This function never fails - it always returns a `CallToolResult`. However, the returned
/// result may represent either a successful or error response depending on internal serialization.
/// If response data serialization fails, it returns an error response instead of panicking.
///
/// Callers should wrap the result in `Ok()` when returning from functions that return
/// `Result<CallToolResult, McpError>`.
pub fn build_launch_success_response(params: LaunchResponseParams) -> CallToolResult {
    let launch_duration_ms = u64::try_from(
        params
            .launch_end
            .duration_since(params.launch_start)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    let launch_timestamp = chrono::Utc::now().to_rfc3339();

    let mut response_data = json!({
        params.name_field: params.name,
        "pid": params.pid,
        "working_directory": params.manifest_dir.display().to_string(),
        "profile": params.profile,
        "log_file": params.log_file_path.display().to_string(),
        "status": "running_in_background",
        "launch_duration_ms": launch_duration_ms,
        "launch_timestamp": launch_timestamp
    });

    // Add any additional data specific to the launch type
    if let Some(Value::Object(additional_map)) = params.additional_data {
        if let Value::Object(ref mut response_map) = response_data {
            response_map.extend(additional_map);
        }
    }

    // Add workspace info
    response::add_workspace_info_to_response(&mut response_data, params.workspace_root);

    let mut builder = ResponseBuilder::success().message(format!(
        "Successfully launched '{}' (PID: {})",
        params.name, params.pid
    ));

    // Add all fields from response_data
    if let Value::Object(map) = response_data {
        let mut current_builder = builder;
        for (key, value) in map {
            current_builder = match current_builder.add_field(&key, value) {
                Ok(new_builder) => new_builder,
                Err(e) => {
                    // Log error but continue with remaining fields
                    tracing::warn!("Failed to add field '{}': {}", key, e);
                    // Create a new builder with the current message and continue
                    ResponseBuilder::success().message(format!(
                        "Successfully launched '{}' (PID: {})",
                        params.name, params.pid
                    ))
                }
            };
        }
        builder = current_builder;
    }

    builder.build().to_call_tool_result()
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
