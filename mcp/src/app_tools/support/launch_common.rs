use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use tracing::debug;

use crate::brp_tools::constants::BRP_PORT_ENV_VAR;
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
    debug!(
        "Launching {name_type} {name} from {}",
        manifest_dir.display()
    );
    debug!("Working directory: {}", manifest_dir.display());
    debug!("CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    debug!("Profile: {profile}");
    debug!(
        "{}: {binary_or_command}",
        if name_type == "app" {
            "Binary path"
        } else {
            "Command"
        }
    );

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
        cmd.env(BRP_PORT_ENV_VAR, port.to_string());
    }
}

/// Parameters for enhanced debug info collection
pub struct EnhancedDebugParams<'a> {
    pub name:              &'a str,
    pub name_type:         &'a str, // "app" or "example"
    pub manifest_dir:      &'a Path,
    pub binary_or_command: &'a str,
    pub profile:           &'a str,
    pub launch_start:      Instant,
    pub launch_end:        Instant,
    pub env_vars:          &'a [(&'a str, &'a str)],
}

/// Parameters for complete launch debug info collection with timing details
pub struct LaunchDebugParams<'a> {
    pub name:               &'a str,
    pub name_type:          &'a str, // "app" or "example"
    pub manifest_dir:       &'a Path,
    pub binary_or_command:  &'a str,
    pub profile:            &'a str,
    pub launch_start:       Instant,
    pub launch_end:         Instant,
    pub port:               Option<u16>,
    pub package_name:       Option<&'a str>, // For examples
    pub find_duration:      Option<std::time::Duration>,
    pub log_setup_duration: Option<std::time::Duration>,
    pub cmd_setup_duration: Option<std::time::Duration>,
    pub spawn_duration:     Option<std::time::Duration>,
}

/// Collects enhanced debug information for launch operations including timing details
pub fn collect_enhanced_launch_debug_info(
    params: EnhancedDebugParams,
    debug_info: &mut Vec<String>,
) {
    let launch_duration_ms = params
        .launch_end
        .duration_since(params.launch_start)
        .as_millis();

    debug!(
        "Launching {} {} from {}",
        params.name_type,
        params.name,
        params.manifest_dir.display()
    );
    debug!("Working directory: {}", params.manifest_dir.display());
    debug!("CARGO_MANIFEST_DIR: {}", params.manifest_dir.display());
    debug!("Profile: {}", params.profile);
    debug!(
        "{}: {}",
        if params.name_type == "app" {
            "Binary path"
        } else {
            "Command"
        },
        params.binary_or_command
    );
    debug!("Launch duration: {launch_duration_ms}ms");

    if !params.env_vars.is_empty() {
        debug!("Environment variables:");
        for (key, value) in params.env_vars {
            debug!("  {key}={value}");
        }
    }

    debug_info.push(format!(
        "Launching {} {} from {}",
        params.name_type,
        params.name,
        params.manifest_dir.display()
    ));
    debug_info.push(format!(
        "Working directory: {}",
        params.manifest_dir.display()
    ));
    debug_info.push(format!(
        "CARGO_MANIFEST_DIR: {}",
        params.manifest_dir.display()
    ));
    debug_info.push(format!("Profile: {}", params.profile));
    debug_info.push(format!(
        "{}: {}",
        if params.name_type == "app" {
            "Binary path"
        } else {
            "Command"
        },
        params.binary_or_command
    ));
    debug_info.push(format!("Launch duration: {launch_duration_ms}ms"));

    if !params.env_vars.is_empty() {
        debug_info.push("Environment variables:".to_string());
        for (key, value) in params.env_vars {
            debug_info.push(format!("  {key}={value}"));
        }
    }
}

/// Collects complete launch debug information including timing breakdowns
pub fn collect_complete_launch_debug_info(params: LaunchDebugParams, debug_info: &mut Vec<String>) {
    let mut env_vars = Vec::new();
    if let Some(port) = params.port {
        env_vars.push((BRP_PORT_ENV_VAR, port.to_string()));
    }

    // Collect base enhanced debug info
    collect_enhanced_launch_debug_info(
        EnhancedDebugParams {
            name:              params.name,
            name_type:         params.name_type,
            manifest_dir:      params.manifest_dir,
            binary_or_command: params.binary_or_command,
            profile:           params.profile,
            launch_start:      params.launch_start,
            launch_end:        params.launch_end,
            env_vars:          &env_vars
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
        },
        debug_info,
    );

    // Add package name if provided (for examples)
    if let Some(package_name) = params.package_name {
        debug!("Package: {package_name}");
        debug_info.push(format!("Package: {package_name}"));
    }

    // Add timing information if provided
    if let Some(find_duration) = params.find_duration {
        debug!(
            "TIMING - Find {}: {}ms",
            params.name_type,
            find_duration.as_millis()
        );
        debug_info.push(format!(
            "TIMING - Find {}: {}ms",
            params.name_type,
            find_duration.as_millis()
        ));
    }
    if let Some(log_setup_duration) = params.log_setup_duration {
        debug!("TIMING - Log setup: {}ms", log_setup_duration.as_millis());
        debug_info.push(format!(
            "TIMING - Log setup: {}ms",
            log_setup_duration.as_millis()
        ));
    }
    if let Some(cmd_setup_duration) = params.cmd_setup_duration {
        debug!(
            "TIMING - Command setup: {}ms",
            cmd_setup_duration.as_millis()
        );
        debug_info.push(format!(
            "TIMING - Command setup: {}ms",
            cmd_setup_duration.as_millis()
        ));
    }
    if let Some(spawn_duration) = params.spawn_duration {
        debug!("TIMING - Spawn process: {}ms", spawn_duration.as_millis());
        debug_info.push(format!(
            "TIMING - Spawn process: {}ms",
            spawn_duration.as_millis()
        ));
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

/// Build final response with debug info injection
pub fn build_final_launch_response(
    base_response: CallToolResult,
    debug_info: Vec<String>,
    success_message: String,
) -> CallToolResult {
    use crate::support::response::ResponseBuilder;
    use crate::support::serialization::json_response_to_result;

    // Extract the inner JSON response and inject debug info using standard approach
    if let Ok(json_str) = serde_json::to_string(&base_response.content) {
        if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(&json_str) {
            let response = ResponseBuilder::success()
                .message(success_message)
                .data(json_response)
                .map_or_else(
                    |_| {
                        ResponseBuilder::error()
                            .message("Failed to serialize response data")
                            .build()
                    },
                    |builder| {
                        builder
                            .auto_inject_debug_info(if debug_info.is_empty() {
                                None
                            } else {
                                Some(debug_info)
                            })
                            .build()
                    },
                );

            return json_response_to_result(&response);
        }
    }

    base_response
}
