use std::path::PathBuf;
use std::time::Instant;

use chrono;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use tracing::debug;

use super::constants::{DEFAULT_PROFILE, PARAM_APP_NAME, PARAM_PORT, PARAM_PROFILE};
use super::support::cargo_detector::TargetType;
use super::support::{launch_common, process, scanning};
use crate::app_tools::constants::PROFILE_RELEASE;
use crate::error::{Error, report_to_mcp_error};
use crate::extractors::McpCallExtractor;
use crate::response::BevyAppLaunchResult;
use crate::{BrpMcpService, service};

pub async fn handle(
    service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<BevyAppLaunchResult, McpError> {
    // Get parameters
    let extractor = McpCallExtractor::from_request(&request);
    let app_name = extractor.get_required_string(PARAM_APP_NAME, "app name")?;
    let profile = extractor.get_optional_string(PARAM_PROFILE, DEFAULT_PROFILE);
    let path = extractor.get_optional_path();
    let port = extractor.get_optional_u16(PARAM_PORT)?;

    // Get search paths
    let search_paths = service::fetch_roots_and_get_paths(service, context).await?;

    // Launch the app
    launch_bevy_app(app_name, profile, path.as_deref(), port, &search_paths)
}

pub fn launch_bevy_app(
    app_name: &str,
    profile: &str,
    path: Option<&str>,
    port: Option<u16>,
    search_paths: &[PathBuf],
) -> Result<BevyAppLaunchResult, McpError> {
    let launch_start = Instant::now();

    // Find the app
    let app = match scanning::find_required_target_with_path(
        app_name,
        TargetType::App,
        path,
        search_paths,
    ) {
        Ok(app) => app,
        Err(mcp_error) => {
            // Check if this is a path disambiguation error
            let error_msg = &mcp_error.message;
            if error_msg.contains("Found multiple") || error_msg.contains("not found at path") {
                // Parse duplicate paths from error message
                let duplicate_paths = if error_msg.contains("Found multiple") {
                    // Extract paths from error message like "Found multiple app named 'hana' at:\n-
                    // hana\n- hana-brp-extras-2-1"
                    let lines: Vec<&str> = error_msg.lines().collect();
                    let mut paths = Vec::new();
                    for line in &lines[1..] {
                        // Skip first line
                        if let Some(path) = line.strip_prefix("- ") {
                            paths.push(path.to_string());
                        }
                    }
                    if paths.is_empty() { None } else { Some(paths) }
                } else {
                    None
                };

                // Return structured error response with minimal fields
                let error_response = super::support::create_disambiguation_error(
                    "app",
                    app_name,
                    duplicate_paths.unwrap_or_default(),
                );

                // Convert the JSON value to our typed result
                // We only populate the fields that are in the error response
                return Ok(BevyAppLaunchResult {
                    status:             error_response["status"]
                        .as_str()
                        .unwrap_or("error")
                        .to_string(),
                    message:            error_response["message"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    app_name:           error_response["app_name"].as_str().map(|s| s.to_string()),
                    pid:                None,
                    working_directory:  None,
                    profile:            None,
                    log_file:           None,
                    binary_path:        None,
                    launch_duration_ms: None,
                    launch_timestamp:   None,
                    workspace:          None,
                    duplicate_paths:    error_response["duplicate_paths"].as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    }),
                });
            }
            return Err(mcp_error);
        }
    };

    // Build the binary path
    let binary_path = app.get_binary_path(profile);

    // Check if the binary exists
    if !binary_path.exists() {
        return Err(report_to_mcp_error(
            &error_stack::Report::new(Error::Configuration("Missing binary file".to_string()))
                .attach_printable(format!("Binary path: {}", binary_path.display()))
                .attach_printable(format!(
                    "Please build the app with 'cargo build{}' first",
                    if profile == PROFILE_RELEASE {
                        " --release"
                    } else {
                        ""
                    }
                )),
        ));
    }

    // Get the manifest directory (parent of Cargo.toml)
    let manifest_dir = launch_common::validate_manifest_directory(&app.manifest_path)?;

    debug!("Launching app {} from {}", app_name, manifest_dir.display());
    debug!("Working directory: {}", manifest_dir.display());
    debug!("CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    debug!("Profile: {}", profile);
    debug!("Binary path: {}", binary_path.display());

    // Setup logging
    let (log_file_path, log_file_for_redirect) = launch_common::setup_launch_logging(
        app_name,
        "App",
        profile,
        &binary_path,
        manifest_dir,
        port,
        None,
    )?;

    // Build app command
    let cmd = launch_common::build_app_command(&binary_path, port);

    let pid = process::launch_detached_process(
        &cmd,
        manifest_dir,
        log_file_for_redirect,
        app_name,
        "launch",
    )?;
    let launch_end = Instant::now();

    // Collect enhanced debug info
    let launch_duration_ms = launch_end.duration_since(launch_start).as_millis();

    debug!("Launch duration: {}ms", launch_duration_ms);

    if let Some(port) = port {
        debug!("Environment variable: BRP_PORT={}", port);
    }

    // Collect enhanced debug info
    let launch_duration_ms = launch_end.duration_since(launch_start).as_millis() as u64;
    let launch_timestamp = chrono::Utc::now().to_rfc3339();

    // Extract workspace name
    let workspace = app
        .workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string());

    Ok(BevyAppLaunchResult {
        status: "success".to_string(),
        message: format!("Successfully launched '{}' (PID: {})", app_name, pid),
        app_name: Some(app_name.to_string()),
        pid: Some(pid),
        working_directory: Some(manifest_dir.display().to_string()),
        profile: Some(profile.to_string()),
        log_file: Some(log_file_path.display().to_string()),
        binary_path: Some(binary_path.display().to_string()),
        launch_duration_ms: Some(launch_duration_ms),
        launch_timestamp: Some(launch_timestamp),
        workspace,
        duplicate_paths: None, // No duplicates for successful launches
    })
}
