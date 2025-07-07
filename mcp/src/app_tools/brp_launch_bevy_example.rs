use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;
use tracing::debug;

use super::constants::{
    DEFAULT_PROFILE, PARAM_EXAMPLE_NAME, PARAM_PORT, PARAM_PROFILE, PROFILE_RELEASE,
};
use super::support::{cargo_detector, launch_common, process, scanning};
use crate::support::params;
use crate::{BrpMcpService, service};

pub async fn handle(
    service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    service::handle_launch_binary(service, request, context, |req, search_paths| async move {
        // Get parameters
        let example_name = params::extract_required_string(&req, PARAM_EXAMPLE_NAME)?;
        let profile = params::extract_optional_string(&req, PARAM_PROFILE, DEFAULT_PROFILE);
        let path = params::extract_optional_path(&req);
        let port = params::extract_optional_u16_from_request(&req, PARAM_PORT)?;

        // Launch the example
        launch_bevy_example(example_name, profile, path.as_deref(), port, &search_paths)
    })
    .await
}

pub fn launch_bevy_example(
    example_name: &str,
    profile: &str,
    path: Option<&str>,
    port: Option<u16>,
    search_paths: &[PathBuf],
) -> Result<CallToolResult, McpError> {
    let launch_start = Instant::now();

    // Find and validate the example
    let (example, manifest_dir_buf, find_duration) =
        find_and_validate_example(example_name, path, search_paths)?;
    let manifest_dir = manifest_dir_buf.as_path();

    // Setup launch environment
    let cargo_command = build_cargo_command_string(example_name, profile);
    collect_debug_info_if_enabled(example_name, &cargo_command, profile, manifest_dir);

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
    let launch_result = execute_launch_process(launch_params)?;

    // Build and return response
    build_launch_response(example_name, &example, launch_result, launch_start, profile)
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
) -> Result<(cargo_detector::ExampleInfo, PathBuf, Duration), McpError> {
    let find_start = Instant::now();
    let example = scanning::find_required_example_with_path(example_name, path, search_paths)?;
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
) {
    debug!(
        "Launching example {} from {}",
        example_name,
        manifest_dir.display()
    );
    debug!("Working directory: {}", manifest_dir.display());
    debug!("CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    debug!("Profile: {}", profile);
    debug!("Command: {}", cargo_command);
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

fn execute_launch_process(params: LaunchProcessParams<'_>) -> Result<LaunchResult, McpError> {
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

    // Collect complete debug info
    let launch_duration_ms = launch_end.duration_since(params.launch_start).as_millis();

    debug!("Launch duration: {}ms", launch_duration_ms);
    debug!("Package: {}", params.package_name);
    debug!(
        "TIMING - Find example: {}ms",
        params.find_duration.as_millis()
    );
    debug!("TIMING - Log setup: {}ms", log_setup_duration.as_millis());
    debug!(
        "TIMING - Command setup: {}ms",
        cmd_setup_duration.as_millis()
    );
    debug!("TIMING - Spawn process: {}ms", spawn_duration.as_millis());

    if let Some(port) = params.port {
        debug!("Environment variable: BRP_PORT={}", port);
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

    Ok(launch_common::build_launch_success_response(
        response_params,
    ))
}
