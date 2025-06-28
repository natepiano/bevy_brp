use rmcp::model::{CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;
use sysinfo::System;

use super::constants::{DEFAULT_BRP_PORT, JSON_FIELD_PORT, JSON_FIELD_STATUS};
use super::support::brp_client::{BrpResult, execute_brp_method};
use crate::BrpMcpService;
use crate::brp_tools::brp_set_debug_mode::is_debug_enabled;
use crate::constants::{PARAM_APP_NAME, PARAM_PORT};
use crate::error::{Error, report_to_mcp_error};
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::{params, schema};
use crate::tools::{
    BRP_METHOD_EXTRAS_SET_DEBUG_MODE, BRP_METHOD_LIST, DESC_BRP_STATUS, TOOL_BRP_STATUS,
};

pub fn register_tool() -> Tool {
    Tool {
        name:         TOOL_BRP_STATUS.into(),
        description:  DESC_BRP_STATUS.into(),
        input_schema: schema::SchemaBuilder::new()
            .add_string_property(PARAM_APP_NAME, "Name of the process to check for", true)
            .add_number_property(
                PARAM_PORT,
                &format!("Port to check for BRP (default: {DEFAULT_BRP_PORT})"),
                false,
            )
            .build(),
    }
}

pub async fn handle(
    _service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    // Get parameters
    let app_name = params::extract_required_string(&request, PARAM_APP_NAME)?;
    let port = params::extract_optional_number(&request, PARAM_PORT, u64::from(DEFAULT_BRP_PORT))?;

    // Check the app
    check_brp_for_app(
        app_name,
        u16::try_from(port).map_err(|_| -> McpError {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(
                    "Invalid port value".to_string(),
                ))
                .attach_printable("Port must be a valid u16")
                .attach_printable(format!("Provided value: {port}")),
            )
        })?,
    )
    .await
}

/// Normalize process name for robust matching
fn normalize_process_name(name: &str) -> String {
    // Convert to lowercase and remove common path separators and extensions
    let name = name.to_lowercase();

    // Remove path components - get just the base name
    let base_name = name.split(['/', '\\']).next_back().unwrap_or(&name);

    // Remove common executable extensions
    base_name
        .strip_suffix(".exe")
        .or_else(|| base_name.strip_suffix(".app"))
        .or_else(|| base_name.strip_suffix(".bin"))
        .unwrap_or(base_name)
        .to_string()
}

/// Check if process matches the target app name
fn process_matches_app(process: &sysinfo::Process, target_app: &str) -> bool {
    let normalized_target = normalize_process_name(target_app);

    // Check process name
    let process_name = process.name().to_string_lossy();
    let normalized_process_name = normalize_process_name(&process_name);

    if normalized_process_name == normalized_target {
        return true;
    }

    // Check command line arguments for additional matching
    // This helps catch cases where the process name is different from the binary name
    if let Some(cmd) = process.cmd().first() {
        let cmd_normalized = normalize_process_name(&cmd.to_string_lossy());
        if cmd_normalized.contains(&normalized_target)
            || normalized_target.contains(&cmd_normalized)
        {
            return true;
        }
    }

    // Check all command line arguments for potential matches
    for arg in process.cmd() {
        let arg_str = arg.to_string_lossy();
        let arg_normalized = normalize_process_name(&arg_str);

        // Check if this argument contains our target name
        if arg_normalized.contains(&normalized_target) {
            return true;
        }
    }

    false
}

async fn check_brp_for_app(app_name: &str, port: u16) -> Result<CallToolResult, McpError> {
    // Check if a process with this name is running using sysinfo
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let running_process = system
        .processes()
        .values()
        .find(|process| process_matches_app(process, app_name));

    // Check BRP connectivity
    let brp_responsive = check_brp_on_port(port).await?;

    // Check extras debug responsiveness
    let extras_debug_responsive = check_extras_debug_responsive(port).await;

    // Build response based on findings
    let (status, message, app_running, app_pid) = match (running_process, brp_responsive) {
        (Some(process), true) => {
            let pid = process.pid().as_u32();
            (
                "running_with_brp",
                format!(
                    "Process '{app_name}' (PID: {pid}) is running with BRP enabled on port {port}"
                ),
                true,
                Some(pid),
            )
        }
        (Some(process), false) => {
            let pid = process.pid().as_u32();
            (
                "running_no_brp",
                format!(
                    "Process '{app_name}' (PID: {pid}) is running but not responding to BRP on port {port}. Make sure RemotePlugin is added to your Bevy app."
                ),
                true,
                Some(pid),
            )
        }
        (None, true) => {
            // BRP is responding but our specific process isn't found
            (
                "brp_found_process_not_detected",
                format!(
                    "BRP is responding on port {port} but process '{app_name}' not detected. Another process may be using BRP."
                ),
                false,
                None,
            )
        }
        (None, false) => (
            "not_running",
            format!("Process '{app_name}' is not currently running"),
            false,
            None,
        ),
    };

    let response = ResponseBuilder::success()
        .message(message)
        .data(json!({
            JSON_FIELD_STATUS: status,
            "app_name": app_name,
            JSON_FIELD_PORT: port,
            "app_running": app_running,
            "brp_responsive": brp_responsive,
            "app_pid": app_pid,
            "mcp_debug_enabled": is_debug_enabled(),
            "extras_debug_responsive": extras_debug_responsive
        }))
        .map_or_else(
            |_| {
                ResponseBuilder::error()
                    .message("Failed to serialize response data")
                    .build()
            },
            ResponseBuilder::build,
        );

    Ok(json_response_to_result(&response))
}

/// Check if BRP is responding on the given port
async fn check_brp_on_port(port: u16) -> Result<bool, McpError> {
    // Try a simple BRP request to check connectivity using bevy/list
    match execute_brp_method(BRP_METHOD_LIST, None, Some(port)).await {
        Ok(BrpResult::Success(_)) => {
            // BRP is responding and working
            Ok(true)
        }
        Ok(BrpResult::Error(_)) | Err(_) => {
            // BRP not responding or returned an error
            Ok(false)
        }
    }
}

/// Check if extras debug commands are responsive on the given port
async fn check_extras_debug_responsive(port: u16) -> bool {
    // Try a simple extras debug command to check if extras plugin is available
    let params = serde_json::json!({
        "enabled": false  // Just check if the command works, don't change state
    });

    match execute_brp_method(BRP_METHOD_EXTRAS_SET_DEBUG_MODE, Some(params), Some(port)).await {
        Ok(BrpResult::Success(_)) => true,
        Ok(BrpResult::Error(_)) | Err(_) => false,
    }
}
