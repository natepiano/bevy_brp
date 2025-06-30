use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

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
    let launch_start = Instant::now();
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
    let launch_end = Instant::now();

    // Collect enhanced debug info if enabled
    if is_debug_enabled() {
        let mut env_vars = Vec::new();
        if let Some(port) = port {
            env_vars.push(("BRP_PORT", port.to_string()));
        }

        launch_common::collect_enhanced_launch_debug_info(
            example_name,
            "example",
            manifest_dir,
            &cargo_command,
            profile,
            launch_start,
            launch_end,
            &env_vars
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
            &mut debug_info,
        );
        debug_info.push(format!("Package: {}", example.package_name));
    }

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
        launch_start,
        launch_end,
    };

    let base_response = launch_common::build_launch_success_response(response_params);

    // Extract the inner JSON response and inject debug info using standard approach
    if let Ok(json_str) = serde_json::to_string(&base_response.content) {
        if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(&json_str) {
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
                    |builder| {
                        builder
                            .auto_inject_debug_info(
                                if debug_info.is_empty() {
                                    None
                                } else {
                                    Some(debug_info)
                                },
                                None::<Vec<String>>,
                            )
                            .build()
                    },
                );

            return Ok(json_response_to_result(&response));
        }
    }

    Ok(base_response)
}
