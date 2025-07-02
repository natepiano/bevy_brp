use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::{cargo_detector, launch_common, process, scanning};
use crate::BrpMcpService;
use crate::brp_tools::brp_set_tracing_level::get_current_level;
use crate::constants::{
    DEFAULT_PROFILE, PARAM_EXAMPLE_NAME, PARAM_PORT, PARAM_PROFILE, PROFILE_RELEASE,
};
use crate::support::tracing::TracingLevel;
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

    // Find and validate the example
    let (example, manifest_dir_buf, find_duration) =
        find_and_validate_example(example_name, path, search_paths, &mut debug_info)?;
    let manifest_dir = manifest_dir_buf.as_path();

    // Setup launch environment
    let cargo_command = build_cargo_command_string(example_name, profile);
    collect_debug_info_if_enabled(
        example_name,
        &cargo_command,
        profile,
        manifest_dir,
        &mut debug_info,
    );

    // Execute the launch process
    let launch_params = LaunchProcessParams {
        example_name,
        profile,
        port,
        manifest_dir,
        package_name: &example.package_name,
        cargo_command: &cargo_command,
        launch_start,
        find_duration,
    };
    let launch_result = execute_launch_process(launch_params, &mut debug_info)?;

    // Build and return response
    build_launch_response(
        example_name,
        &example,
        launch_result,
        launch_start,
        profile,
        debug_info,
    )
}

struct LaunchResult {
    pid:           u32,
    log_file_path: PathBuf,
    launch_end:    Instant,
}

fn find_and_validate_example(
    example_name: &str,
    path: Option<&str>,
    search_paths: &[PathBuf],
    debug_info: &mut Vec<String>,
) -> Result<(cargo_detector::ExampleInfo, PathBuf, Duration), McpError> {
    let find_start = Instant::now();
    let example =
        scanning::find_required_example_with_path(example_name, path, search_paths, debug_info)?;
    let find_duration = find_start.elapsed();

    let manifest_path = example.manifest_path.clone();
    let manifest_dir = launch_common::validate_manifest_directory(&manifest_path)?;

    Ok((example, manifest_dir.to_path_buf(), find_duration))
}

fn build_cargo_command_string(example_name: &str, profile: &str) -> String {
    format!(
        "cargo run --example {example_name} {}",
        if profile == PROFILE_RELEASE {
            "--release"
        } else {
            ""
        }
    )
    .trim()
    .to_string()
}

fn collect_debug_info_if_enabled(
    example_name: &str,
    cargo_command: &str,
    profile: &str,
    manifest_dir: &Path,
    debug_info: &mut Vec<String>,
) {
    if matches!(
        get_current_level(),
        TracingLevel::Debug | TracingLevel::Trace
    ) {
        launch_common::collect_launch_debug_info(
            example_name,
            "example",
            manifest_dir,
            cargo_command,
            profile,
            debug_info,
        );
    }
}

/// Parameters for executing the launch process
struct LaunchProcessParams<'a> {
    example_name:  &'a str,
    profile:       &'a str,
    port:          Option<u16>,
    manifest_dir:  &'a Path,
    package_name:  &'a str,
    cargo_command: &'a str,
    launch_start:  Instant,
    find_duration: Duration,
}

fn execute_launch_process(
    params: LaunchProcessParams<'_>,
    debug_info: &mut Vec<String>,
) -> Result<LaunchResult, McpError> {
    // Setup logging
    let log_setup_start = Instant::now();
    let (log_file_path, log_file_for_redirect) = launch_common::setup_launch_logging(
        params.example_name,
        "Example",
        params.profile,
        &PathBuf::from(params.cargo_command),
        params.manifest_dir,
        params.port,
        Some(&format!("Package: {}", params.package_name)),
    )?;
    let log_setup_duration = log_setup_start.elapsed();

    // Build cargo command
    let cmd_setup_start = Instant::now();
    let cmd = launch_common::build_cargo_example_command(
        params.example_name,
        params.profile,
        params.port,
    );
    let cmd_setup_duration = cmd_setup_start.elapsed();

    // Launch the process
    let spawn_start = Instant::now();
    let pid = process::launch_detached_process(
        &cmd,
        params.manifest_dir,
        log_file_for_redirect,
        params.example_name,
        "spawn",
    )?;
    let spawn_duration = spawn_start.elapsed();
    let launch_end = Instant::now();

    // Collect complete debug info if enabled
    if matches!(
        get_current_level(),
        TracingLevel::Debug | TracingLevel::Trace
    ) {
        launch_common::collect_complete_launch_debug_info(
            launch_common::LaunchDebugParams {
                name: params.example_name,
                name_type: "example",
                manifest_dir: params.manifest_dir,
                binary_or_command: params.cargo_command,
                profile: params.profile,
                launch_start: params.launch_start,
                launch_end,
                port: params.port,
                package_name: Some(params.package_name),
                find_duration: Some(params.find_duration),
                log_setup_duration: Some(log_setup_duration),
                cmd_setup_duration: Some(cmd_setup_duration),
                spawn_duration: Some(spawn_duration),
            },
            debug_info,
        );
    }

    Ok(LaunchResult {
        pid,
        log_file_path,
        launch_end,
    })
}

fn build_launch_response(
    example_name: &str,
    example: &cargo_detector::ExampleInfo,
    launch_result: LaunchResult,
    launch_start: Instant,
    profile: &str,
    debug_info: Vec<String>,
) -> Result<CallToolResult, McpError> {
    let additional_data = json!({
        "package_name": example.package_name,
        "note": "Cargo will build the example if needed before running"
    });

    let workspace_root =
        super::support::scanning::get_workspace_root_from_manifest(&example.manifest_path);
    let manifest_dir = launch_common::validate_manifest_directory(&example.manifest_path)?;

    let response_params = launch_common::LaunchResponseParams {
        name: example_name,
        name_field: "example_name",
        pid: launch_result.pid,
        manifest_dir,
        profile,
        log_file_path: &launch_result.log_file_path,
        additional_data: Some(additional_data),
        workspace_root: workspace_root.as_ref(),
        launch_start,
        launch_end: launch_result.launch_end,
    };

    let base_response = launch_common::build_launch_success_response(response_params);

    Ok(launch_common::build_final_launch_response(
        base_response,
        debug_info,
        format!(
            "Successfully launched '{example_name}' (PID: {})",
            launch_result.pid
        ),
    ))
}
