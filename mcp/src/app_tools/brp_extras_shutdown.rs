use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;
use sysinfo::{Signal, System};

use crate::BrpMcpService;
use crate::brp_tools::brp_set_debug_mode::is_debug_enabled;
use crate::brp_tools::constants::DEFAULT_BRP_PORT;
use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};
use crate::error::{Error, Result, report_to_mcp_error};
use crate::support::params;
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;

/// Helper function to build shutdown response with debug info
fn build_shutdown_response(
    message: &str,
    response_data: serde_json::Value,
    debug_info: &[String],
) -> CallToolResult {
    let response = ResponseBuilder::success()
        .message(message)
        .data(response_data)
        .map_or_else(
            |_| {
                ResponseBuilder::error()
                    .message("Failed to serialize response data")
                    .auto_inject_debug_info(Some(debug_info), None::<&serde_json::Value>)
                    .build()
            },
            |builder| {
                builder
                    .auto_inject_debug_info(Some(debug_info), None::<&serde_json::Value>)
                    .build()
            },
        );

    json_response_to_result(&response)
}
use crate::tools::BRP_METHOD_EXTRAS_SHUTDOWN;

/// Result of a shutdown operation
enum ShutdownResult {
    /// Graceful shutdown via `bevy_brp_extras` succeeded
    CleanShutdown,
    /// Process was killed using system signal
    ProcessKilled { pid: u32 },
    /// Process was not running when shutdown was attempted (may have crashed)
    AlreadyShutdown,
    /// Process was not running
    NotRunning,
    /// An error occurred during shutdown
    Error { message: String },
}

/// Attempt to shutdown a Bevy app, first trying graceful shutdown then falling back to kill
async fn shutdown_app(app_name: &str, port: u16) -> (ShutdownResult, Vec<String>) {
    let mut debug_info = Vec::new();

    if is_debug_enabled() {
        debug_info.push(format!(
            "Starting shutdown process for app '{app_name}' on port {port}"
        ));
    }
    // First, check if the process is actually running
    if !is_process_running(app_name) {
        if is_debug_enabled() {
            debug_info.push(format!(
                "Process '{app_name}' not found in system process list"
            ));
        }
        return (ShutdownResult::AlreadyShutdown, debug_info);
    }

    if is_debug_enabled() {
        debug_info.push(format!(
            "Process '{app_name}' found, attempting graceful shutdown"
        ));
    }

    // Process is running, try graceful shutdown via bevy_brp_extras
    match try_graceful_shutdown(port).await {
        Ok((true, mut graceful_debug)) => {
            debug_info.append(&mut graceful_debug);
            if is_debug_enabled() {
                debug_info.push("Graceful shutdown succeeded".to_string());
            }
            (ShutdownResult::CleanShutdown, debug_info)
        }
        Ok((false, mut graceful_debug)) => {
            debug_info.append(&mut graceful_debug);
            if is_debug_enabled() {
                debug_info
                    .push("Graceful shutdown failed, falling back to process kill".to_string());
            }
            // BRP responded but bevy_brp_extras not available - fall back to kill
            handle_kill_process_fallback(app_name, &mut debug_info, None)
        }
        Err(e) => {
            if is_debug_enabled() {
                debug_info.push(format!(
                    "BRP communication error, falling back to process kill: {e}"
                ));
            }
            // BRP not responsive - fall back to kill
            handle_kill_process_fallback(app_name, &mut debug_info, Some(e.to_string()))
        }
    }
}

/// Handle the fallback to kill process when graceful shutdown fails
fn handle_kill_process_fallback(
    app_name: &str,
    debug_info: &mut Vec<String>,
    brp_error: Option<String>,
) -> (ShutdownResult, Vec<String>) {
    match kill_process(app_name) {
        Ok(Some(pid)) => {
            if is_debug_enabled() {
                debug_info.push(format!(
                    "Successfully killed process {app_name} with PID {pid}"
                ));
            }
            (ShutdownResult::ProcessKilled { pid }, debug_info.clone())
        }
        Ok(None) => {
            if is_debug_enabled() {
                let msg = if brp_error.is_some() {
                    format!(
                        "Process '{app_name}' not found when attempting to kill after BRP failure"
                    )
                } else {
                    format!("Process '{app_name}' not found when attempting to kill")
                };
                debug_info.push(msg);
            }
            (ShutdownResult::NotRunning, debug_info.clone())
        }
        Err(kill_err) => {
            if is_debug_enabled() {
                let msg = if brp_error.is_some() {
                    format!("Failed to kill process '{app_name}' after BRP failure: {kill_err:?}")
                } else {
                    format!("Failed to kill process '{app_name}': {kill_err:?}")
                };
                debug_info.push(msg);
            }
            let error_message = brp_error.map_or_else(
                || format!("{kill_err:?}"),
                |brp_err| format!("BRP failed: {brp_err}, Kill failed: {kill_err:?}"),
            );
            (
                ShutdownResult::Error {
                    message: error_message,
                },
                debug_info.clone(),
            )
        }
    }
}

pub async fn handle(
    _service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> std::result::Result<CallToolResult, McpError> {
    // Get parameters
    let app_name = params::extract_required_string(&request, "app_name")?;
    let port = params::extract_optional_number(&request, "port", u64::from(DEFAULT_BRP_PORT))?;

    let port = u16::try_from(port).map_err(|_| -> McpError {
        report_to_mcp_error(
            &error_stack::Report::new(Error::ParameterExtraction(
                "Invalid port parameter".to_string(),
            ))
            .attach_printable("Port must be a valid u16")
            .attach_printable(format!("Provided value: {port}")),
        )
    })?;

    // Shutdown the app
    let (result, debug_info) = shutdown_app(app_name, port).await;

    // Build and return standard response
    match result {
        ShutdownResult::CleanShutdown => {
            let message = format!(
                "Successfully initiated graceful shutdown for '{app_name}' via bevy_brp_extras on port {port}"
            );
            let response_data = json!({
                "status": "success",
                "method": "clean_shutdown",
                "app_name": app_name,
                "port": port,
                "message": message
            });

            Ok(build_shutdown_response(
                &message,
                response_data,
                &debug_info,
            ))
        }
        ShutdownResult::ProcessKilled { pid } => {
            let message = format!(
                "Terminated process '{app_name}' (PID: {pid}) using kill. Consider adding bevy_brp_extras for clean shutdown."
            );
            let response_data = json!({
                "status": "success",
                "method": "process_kill",
                "app_name": app_name,
                "port": port,
                "pid": pid,
                "message": message
            });

            Ok(build_shutdown_response(
                &message,
                response_data,
                &debug_info,
            ))
        }
        ShutdownResult::AlreadyShutdown => {
            let message = format!(
                "Process '{app_name}' is not running - may have already shutdown or crashed. No action needed."
            );
            let response_data = json!({
                "status": "error",
                "method": "already_shutdown",
                "app_name": app_name,
                "port": port,
                "message": message
            });

            Ok(build_shutdown_response(
                &message,
                response_data,
                &debug_info,
            ))
        }
        ShutdownResult::NotRunning => {
            let message = format!("Process '{app_name}' is not currently running");
            let response_data = json!({
                "status": "error",
                "method": "none",
                "app_name": app_name,
                "port": port,
                "message": message
            });

            Ok(build_shutdown_response(
                &message,
                response_data,
                &debug_info,
            ))
        }
        ShutdownResult::Error { message } => {
            let response_data = json!({
                "status": "error",
                "method": "process_kill_failed",
                "app_name": app_name,
                "port": port,
                "message": message
            });

            Ok(build_shutdown_response(
                &message,
                response_data,
                &debug_info,
            ))
        }
    }
}

/// Try to gracefully shutdown via `bevy_brp_extras`
async fn try_graceful_shutdown(port: u16) -> Result<(bool, Vec<String>)> {
    let mut debug_info = Vec::new();

    if is_debug_enabled() {
        debug_info.push(format!("Starting graceful shutdown attempt on port {port}"));
    }
    match execute_brp_method(BRP_METHOD_EXTRAS_SHUTDOWN, None, Some(port)).await {
        Ok(BrpResult::Success(result)) => {
            // Graceful shutdown succeeded
            if is_debug_enabled() {
                debug_info.push(format!("BRP extras shutdown successful: {result:?}"));
            }
            Ok((true, debug_info))
        }
        Ok(BrpResult::Error(brp_error)) => {
            // Check if this is a method not found error (bevy_brp_extras not available)
            if brp_error.code == -32601 {
                if is_debug_enabled() {
                    debug_info.push(format!(
                        "BRP extras method not found (code {}): {}",
                        brp_error.code, brp_error.message
                    ));
                }
            } else {
                // Other BRP errors also indicate graceful shutdown failed
                if is_debug_enabled() {
                    debug_info.push(format!(
                        "BRP extras returned error (code {}): {}",
                        brp_error.code, brp_error.message
                    ));
                }
            }
            Ok((false, debug_info))
        }
        Err(e) => {
            // BRP communication failed entirely
            if is_debug_enabled() {
                debug_info.push(format!("BRP communication failed: {e}"));
            }
            Err(error_stack::Report::new(Error::BrpCommunication(
                "BRP communication failed".to_string(),
            ))
            .attach_printable("BRP not responsive")
            .attach_printable(format!("Port: {port}"))
            .attach_printable(format!("Debug info: {debug_info:?}")))
        }
    }
}

/// Check if a process with the given name is currently running
fn is_process_running(app_name: &str) -> bool {
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    system.processes().values().any(|process| {
        let process_name = process.name().to_string_lossy();
        // Match exact name or with common variations (.exe suffix, etc.)
        process_name == app_name
            || process_name == format!("{app_name}.exe")
            || process_name.strip_suffix(".exe").unwrap_or(&process_name) == app_name
    })
}

/// Kill the process using the system signal
fn kill_process(app_name: &str) -> Result<Option<u32>> {
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let running_process = system.processes().values().find(|process| {
        let process_name = process.name().to_string_lossy();
        // Match exact name or with common variations (.exe suffix, etc.)
        process_name == app_name
            || process_name == format!("{app_name}.exe")
            || process_name.strip_suffix(".exe").unwrap_or(&process_name) == app_name
    });

    running_process.map_or(Ok(None), |process| {
        let pid = process.pid().as_u32();

        // Try to kill the process
        if process.kill_with(Signal::Term).unwrap_or(false) {
            Ok(Some(pid))
        } else {
            Err(error_stack::Report::new(Error::ProcessManagement(
                "Failed to terminate process".to_string(),
            ))
            .attach_printable(format!("Process name: {app_name}"))
            .attach_printable(format!("PID: {pid}"))
            .attach_printable("Failed to send SIGTERM signal"))
        }
    })
}
