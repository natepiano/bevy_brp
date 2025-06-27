use std::path::PathBuf;
use std::process::Command;

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::{launch_common, logging, process, scanning};
use crate::BrpMcpService;
use crate::constants::{
    DEFAULT_PROFILE, PARAM_APP_NAME, PARAM_PORT, PARAM_PROFILE, PROFILE_RELEASE,
};
use crate::error::{Error, report_to_mcp_error};
use crate::support::{params, service};

pub async fn handle(
    service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    service::handle_with_request_and_paths(
        service,
        request,
        context,
        |req, search_paths| async move {
            // Get parameters
            let app_name = params::extract_required_string(&req, PARAM_APP_NAME)?;
            let profile = params::extract_optional_string(&req, PARAM_PROFILE, DEFAULT_PROFILE);
            let workspace = params::extract_optional_workspace(&req);
            let port = params::extract_optional_u16_from_request(&req, PARAM_PORT)?;

            // Launch the app
            launch_bevy_app(app_name, profile, workspace.as_deref(), port, &search_paths)
        },
    )
    .await
}

pub fn launch_bevy_app(
    app_name: &str,
    profile: &str,
    workspace: Option<&str>,
    port: Option<u16>,
    search_paths: &[PathBuf],
) -> Result<CallToolResult, McpError> {
    // Find the app
    let app = scanning::find_required_app_with_workspace(app_name, workspace, search_paths)?;

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

    launch_common::print_launch_debug_info(
        app_name,
        "app",
        manifest_dir,
        &binary_path.display().to_string(),
        profile,
    );

    // Create log file
    let (log_file_path, _) =
        logging::create_log_file(app_name, "App", profile, &binary_path, manifest_dir, port)?;

    // Open log file for stdout/stderr redirection
    let log_file_for_redirect = logging::open_log_file_for_redirect(&log_file_path)?;

    // Launch the binary
    let mut cmd = Command::new(&binary_path);

    // Set BRP-related environment variables
    launch_common::set_brp_env_vars(&mut cmd, port);

    let pid = process::launch_detached_process(
        &cmd,
        manifest_dir,
        log_file_for_redirect,
        app_name,
        "launch",
    )?;

    // Create additional app-specific data
    let additional_data = json!({
        "binary_path": binary_path.display().to_string()
    });

    Ok(launch_common::build_launch_success_response(
        launch_common::LaunchResponseParams {
            name: app_name,
            name_field: "app_name",
            pid,
            manifest_dir,
            profile,
            log_file_path: &log_file_path,
            additional_data: Some(additional_data),
            workspace_root: Some(&app.workspace_root),
        },
    ))
}
