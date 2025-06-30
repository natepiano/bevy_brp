use std::path::PathBuf;
use std::time::Instant;

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::{launch_common, process, scanning};
use crate::BrpMcpService;
use crate::brp_tools::brp_set_debug_mode::is_debug_enabled;
use crate::constants::{
    DEFAULT_PROFILE, PARAM_EXAMPLE_NAME, PARAM_PORT, PARAM_PROFILE, PROFILE_RELEASE,
};
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
    let find_start = Instant::now();
    let example = scanning::find_required_example_with_path(
        example_name,
        path,
        search_paths,
        &mut debug_info,
    )?;
    let find_duration = find_start.elapsed();

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

    // Setup logging with package info
    let log_setup_start = Instant::now();
    let (log_file_path, log_file_for_redirect) = launch_common::setup_launch_logging(
        example_name,
        "Example",
        profile,
        &PathBuf::from(&cargo_command),
        manifest_dir,
        port,
        Some(&format!("Package: {}", example.package_name)),
    )?;
    let log_setup_duration = log_setup_start.elapsed();

    // Build cargo command
    let cmd_setup_start = Instant::now();
    let cmd = launch_common::build_cargo_example_command(example_name, profile, port);
    let cmd_setup_duration = cmd_setup_start.elapsed();

    // Launch the process
    let spawn_start = Instant::now();
    let pid = process::launch_detached_process(
        &cmd,
        manifest_dir,
        log_file_for_redirect,
        example_name,
        "spawn",
    )?;
    let spawn_duration = spawn_start.elapsed();
    let launch_end = Instant::now();

    // Collect enhanced debug info if enabled
    if is_debug_enabled() {
        launch_common::collect_complete_launch_debug_info(
            launch_common::LaunchDebugParams {
                name: example_name,
                name_type: "example",
                manifest_dir,
                binary_or_command: &cargo_command,
                profile,
                launch_start,
                launch_end,
                port,
                package_name: Some(&example.package_name),
                find_duration: Some(find_duration),
                log_setup_duration: Some(log_setup_duration),
                cmd_setup_duration: Some(cmd_setup_duration),
                spawn_duration: Some(spawn_duration),
            },
            &mut debug_info,
        );
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

    Ok(launch_common::build_final_launch_response(
        base_response,
        debug_info,
        format!("Successfully launched '{example_name}' (PID: {pid})"),
    ))
}
