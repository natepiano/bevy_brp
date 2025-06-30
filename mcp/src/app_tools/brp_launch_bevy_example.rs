use std::path::PathBuf;
use std::process::Command;

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::{launch_common, logging, process, scanning};
use crate::BrpMcpService;
use crate::brp_tools::brp_set_debug_mode::is_debug_enabled;
use crate::constants::{
    DEFAULT_PROFILE, PARAM_EXAMPLE_NAME, PARAM_PORT, PARAM_PROFILE, PROFILE_RELEASE,
};
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::{params, service};

pub async fn handle(
    service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    // Get parameters
    let example_name = params::extract_required_string(&request, PARAM_EXAMPLE_NAME)?;
    let profile = params::extract_optional_string(&request, PARAM_PROFILE, DEFAULT_PROFILE);
    let path = params::extract_optional_path(&request);
    let port = params::extract_optional_u16_from_request(&request, PARAM_PORT)?;

    // Fetch current roots
    let search_paths = service::fetch_roots_and_get_paths(service, context).await?;

    // Launch the example
    launch_bevy_example(example_name, profile, path.as_deref(), port, &search_paths)
}

pub fn launch_bevy_example(
    example_name: &str,
    profile: &str,
    path: Option<&str>,
    port: Option<u16>,
    search_paths: &[PathBuf],
) -> Result<CallToolResult, McpError> {
    let mut debug_info = Vec::new();

    // Find the example
    let example = scanning::find_required_example_with_path(
        example_name,
        path,
        search_paths,
        &mut debug_info,
    )?;

    // Get the manifest directory (parent of Cargo.toml)
    let manifest_dir = launch_common::validate_manifest_directory(&example.manifest_path)?;

    // Build cargo command string for debug output
    let cargo_command = format!(
        "cargo run --example {example_name} {}",
        if profile == PROFILE_RELEASE {
            "--release"
        } else {
            ""
        }
    )
    .trim()
    .to_string();

    if is_debug_enabled() {
        launch_common::collect_launch_debug_info(
            example_name,
            "example",
            manifest_dir,
            &cargo_command,
            profile,
            &mut debug_info,
        );
        debug_info.push(format!("Package: {}", example.package_name));
    }

    // Create log file for example output (examples use cargo run, so we pass the command string)

    let (log_file_path, _) = logging::create_log_file(
        example_name,
        "Example",
        profile,
        &PathBuf::from(&cargo_command),
        manifest_dir,
        port,
    )?;

    // Add extra info to log file
    logging::append_to_log_file(
        &log_file_path,
        &format!("Package: {}\n", example.package_name),
    )?;

    // Open log file for stdout/stderr redirection
    let log_file_for_redirect = logging::open_log_file_for_redirect(&log_file_path)?;

    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example").arg(example_name);

    // Add profile flag if release
    if profile == PROFILE_RELEASE {
        cmd.arg("--release");
    }

    // Set BRP-related environment variables
    launch_common::set_brp_env_vars(&mut cmd, port);

    // Launch the process
    let pid = process::launch_detached_process(
        &cmd,
        manifest_dir,
        log_file_for_redirect,
        example_name,
        "spawn",
    )?;

    // Create additional example-specific data
    let additional_data = json!({
        "package_name": example.package_name,
        "note": "Cargo will build the example if needed before running"
    });

    // Get workspace info
    let workspace_root =
        super::support::scanning::get_workspace_root_from_manifest(&example.manifest_path);

    let response_params = launch_common::LaunchResponseParams {
        name: example_name,
        name_field: "example_name",
        pid,
        manifest_dir,
        profile,
        log_file_path: &log_file_path,
        additional_data: Some(additional_data),
        workspace_root: workspace_root.as_ref(),
    };

    let base_response = launch_common::build_launch_success_response(response_params);

    // Extract the inner JSON response and inject debug info
    if let Ok(json_str) = serde_json::to_string(&base_response.content) {
        if let Ok(mut json_response) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if is_debug_enabled() && !debug_info.is_empty() {
                if let Some(obj) = json_response.as_object_mut() {
                    obj.insert("brp_mcp_debug_info".to_string(), json!(debug_info));
                }
            }

            let response = ResponseBuilder::success()
                .message(format!(
                    "Successfully launched '{example_name}' (PID: {pid})"
                ))
                .data(json_response)
                .map_or_else(
                    |_| {
                        ResponseBuilder::error()
                            .message("Failed to serialize response data")
                            .build()
                    },
                    ResponseBuilder::build,
                );

            return Ok(json_response_to_result(&response));
        }
    }

    Ok(base_response)
}
