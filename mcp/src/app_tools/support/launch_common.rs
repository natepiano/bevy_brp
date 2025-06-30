use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

use crate::error::{Error, report_to_mcp_error};
use crate::support::response;
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;

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

/// Collects common debug information for launch operations
pub fn collect_launch_debug_info(
    name: &str,
    name_type: &str, // "app" or "example"
    manifest_dir: &Path,
    binary_or_command: &str,
    profile: &str,
    debug_info: &mut Vec<String>,
) {
    debug_info.push(format!(
        "Launching {name_type} {name} from {}",
        manifest_dir.display()
    ));
    debug_info.push(format!("Working directory: {}", manifest_dir.display()));
    debug_info.push(format!("CARGO_MANIFEST_DIR: {}", manifest_dir.display()));
    debug_info.push(format!("Profile: {profile}"));
    debug_info.push(format!(
        "{}: {binary_or_command}",
        if name_type == "app" {
            "Binary path"
        } else {
            "Command"
        }
    ));
}

/// Creates a success response with common fields and workspace info
pub fn build_launch_success_response(params: LaunchResponseParams) -> CallToolResult {
    let launch_duration_ms = params
        .launch_end
        .duration_since(params.launch_start)
        .as_millis() as u64;
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

    let response = ResponseBuilder::success()
        .message(format!(
            "Successfully launched '{}' (PID: {})",
            params.name, params.pid
        ))
        .data(response_data)
        .map_or_else(
            |_| {
                ResponseBuilder::error()
                    .message("Failed to serialize response data")
                    .build()
            },
            ResponseBuilder::build,
        );

    json_response_to_result(&response)
}

/// Sets BRP-related environment variables on a command
///
/// Currently sets:
/// - `BRP_PORT`: When a port is provided, sets this environment variable for `bevy_brp_extras` to
///   read
pub fn set_brp_env_vars(cmd: &mut Command, port: Option<u16>) {
    if let Some(port) = port {
        cmd.env("BRP_PORT", port.to_string());
    }
}

/// Collects enhanced debug information for launch operations including timing details
pub fn collect_enhanced_launch_debug_info(
    name: &str,
    name_type: &str, // "app" or "example"
    manifest_dir: &Path,
    binary_or_command: &str,
    profile: &str,
    launch_start: Instant,
    launch_end: Instant,
    env_vars: &[(&str, &str)],
    debug_info: &mut Vec<String>,
) {
    let launch_duration_ms = launch_end.duration_since(launch_start).as_millis();

    debug_info.push(format!(
        "Launching {name_type} {name} from {}",
        manifest_dir.display()
    ));
    debug_info.push(format!("Working directory: {}", manifest_dir.display()));
    debug_info.push(format!("CARGO_MANIFEST_DIR: {}", manifest_dir.display()));
    debug_info.push(format!("Profile: {profile}"));
    debug_info.push(format!(
        "{}: {binary_or_command}",
        if name_type == "app" {
            "Binary path"
        } else {
            "Command"
        }
    ));
    debug_info.push(format!("Launch duration: {launch_duration_ms}ms"));

    if !env_vars.is_empty() {
        debug_info.push("Environment variables:".to_string());
        for (key, value) in env_vars {
            debug_info.push(format!("  {key}={value}"));
        }
    }
}
