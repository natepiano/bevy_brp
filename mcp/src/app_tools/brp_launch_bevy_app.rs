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
    DEFAULT_PROFILE, PARAM_APP_NAME, PARAM_PORT, PARAM_PROFILE, PROFILE_RELEASE,
};
use crate::error::{Error, report_to_mcp_error};
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
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
            let path = params::extract_optional_path(&req);
            let port = params::extract_optional_u16_from_request(&req, PARAM_PORT)?;

            // Launch the app
            launch_bevy_app(app_name, profile, path.as_deref(), port, &search_paths)
        },
    )
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
    let mut debug_info = Vec::new();

    // Find the app
    let app = scanning::find_required_app_with_path(app_name, path, search_paths, &mut debug_info)?;

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

    if is_debug_enabled() {
        launch_common::collect_launch_debug_info(
            app_name,
            "app",
            manifest_dir,
            &binary_path.display().to_string(),
            profile,
            &mut debug_info,
        );
    }

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
    let launch_end = Instant::now();

    // Collect enhanced debug info if enabled
    if is_debug_enabled() {
        let mut env_vars = Vec::new();
        if let Some(port) = port {
            env_vars.push(("BRP_PORT", port.to_string()));
        }

        launch_common::collect_enhanced_launch_debug_info(
            app_name,
            "app",
            manifest_dir,
            &binary_path.display().to_string(),
            profile,
            launch_start,
            launch_end,
            &env_vars
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
            &mut debug_info,
        );
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

    let base_response = launch_common::build_launch_success_response(response_params);

    // Extract the inner JSON response and inject debug info using standard approach
    if let Ok(json_str) = serde_json::to_string(&base_response.content) {
        if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(&json_str) {
            let response = ResponseBuilder::success()
                .message(format!("Successfully launched '{app_name}' (PID: {pid})"))
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
