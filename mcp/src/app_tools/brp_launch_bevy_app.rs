use std::path::PathBuf;
use std::time::Instant;

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;
use tracing::debug;

use super::constants::{DEFAULT_PROFILE, PARAM_APP_NAME, PARAM_PORT, PARAM_PROFILE};
use super::support::cargo_detector::TargetType;
use super::support::{launch_common, process, scanning};
use crate::app_tools::constants::PROFILE_RELEASE;
use crate::error::{Error, report_to_mcp_error};
use crate::extractors::McpCallExtractor;
use crate::support::response::ResponseBuilder;
use crate::{BrpMcpService, service};

pub async fn handle(
    service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    service::handle_launch_binary(service, request, context, |req, search_paths| async move {
        // Get parameters
        let extractor = McpCallExtractor::from_request(&req);
        let app_name = extractor.get_required_string(PARAM_APP_NAME, "app name")?;
        let profile = extractor.get_optional_string(PARAM_PROFILE, DEFAULT_PROFILE);
        let path = extractor.get_optional_path();
        let port = extractor.get_optional_u16(PARAM_PORT)?;

        // Launch the app
        launch_bevy_app(app_name, profile, path.as_deref(), port, &search_paths)
    })
    .await
}

pub fn launch_bevy_app(
    app_name: &str,
    profile: &str,
    path: Option<&str>,
    port: Option<u16>,
    search_paths: &[PathBuf],
) -> Result<CallToolResult, McpError> {
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
                // Convert to proper tool response
                return Ok(ResponseBuilder::error()
                    .message(error_msg.to_string())
                    .build()
                    .to_call_tool_result());
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

    // Create additional app-specific data
    let additional_data = json!({
        "binary_path": binary_path.display().to_string()
    });

    let response_params = launch_common::LaunchResponseParams {
        name: app_name,
        name_field: "app_name",
        pid,
        manifest_dir,
        profile,
        log_file_path: &log_file_path,
        additional_data: Some(additional_data),
        workspace_root: Some(&app.workspace_root),
        launch_start,
        launch_end,
    };

    Ok(launch_common::build_launch_success_response(
        response_params,
    ))
}
