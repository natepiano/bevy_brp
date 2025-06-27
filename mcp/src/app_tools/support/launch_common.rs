use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Prints common debug information for launch operations
pub fn print_launch_debug_info(
    name: &str,
    name_type: &str, // "app" or "example"
    manifest_dir: &Path,
    binary_or_command: &str,
    profile: &str,
) {
    eprintln!(
        "Launching {name_type} {name} from {}",
        manifest_dir.display()
    );
    eprintln!("Working directory: {}", manifest_dir.display());
    eprintln!("CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    eprintln!("Profile: {profile}");
    eprintln!(
        "{}: {binary_or_command}",
        if name_type == "app" {
            "Binary path"
        } else {
            "Command"
        }
    );
}

/// Creates a success response with common fields and workspace info
pub fn build_launch_success_response(params: LaunchResponseParams) -> CallToolResult {
    let mut response_data = json!({
        params.name_field: params.name,
        "pid": params.pid,
        "working_directory": params.manifest_dir.display().to_string(),
        "profile": params.profile,
        "log_file": params.log_file_path.display().to_string(),
        "status": "running_in_background"
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
