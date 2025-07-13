use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use sysinfo::{Signal, System};
use tracing::debug;

use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};
use crate::constants::{DEFAULT_BRP_PORT, JSON_RPC_ERROR_METHOD_NOT_FOUND, PARAM_APP_NAME};
use crate::error::{Error, Result, report_to_mcp_error};
use crate::service::{HandlerContext, LocalContext};
use crate::tool::{BRP_METHOD_EXTRAS_SHUTDOWN, HandlerResponse, HandlerResult, LocalToolFunction};

/// Result from shutting down a Bevy app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownResultData {
    /// Status of the shutdown operation
    pub status:           String,
    /// Shutdown method used
    pub shutdown_method:  String,
    /// App name that was shut down
    pub app_name:         String,
    /// Port that was checked
    pub port:             u16,
    /// Process ID if terminated via kill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid:              Option<u32>,
    /// Detailed shutdown message for display
    pub shutdown_message: String,
}

impl HandlerResult for ShutdownResultData {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

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
    debug!("Starting shutdown process for app '{app_name}' on port {port}");
    // First, check if the process is actually running
    if !is_process_running(app_name) {
        debug!("Process '{app_name}' not found in system process list");
        return (ShutdownResult::AlreadyShutdown, Vec::new());
    }

    debug!("Process '{app_name}' found, attempting graceful shutdown");

    // Process is running, try graceful shutdown via bevy_brp_extras
    match try_graceful_shutdown(port).await {
        Ok((true, _)) => {
            debug!("Graceful shutdown succeeded");
            (ShutdownResult::CleanShutdown, Vec::new())
        }
        Ok((false, _)) => {
            debug!("Graceful shutdown failed, falling back to process kill");
            // BRP responded but bevy_brp_extras not available - fall back to kill
            handle_kill_process_fallback(app_name, None)
        }
        Err(e) => {
            debug!("BRP communication error, falling back to process kill: {e}");
            // BRP not responsive - fall back to kill
            handle_kill_process_fallback(app_name, Some(e.to_string()))
        }
    }
}

/// Handle the fallback to kill process when graceful shutdown fails
fn handle_kill_process_fallback(
    app_name: &str,
    brp_error: Option<String>,
) -> (ShutdownResult, Vec<String>) {
    match kill_process(app_name) {
        Ok(Some(pid)) => {
            debug!("Successfully killed process {app_name} with PID {pid}");
            (ShutdownResult::ProcessKilled { pid }, Vec::new())
        }
        Ok(None) => {
            if brp_error.is_some() {
                debug!("Process '{app_name}' not found when attempting to kill after BRP failure");
            } else {
                debug!("Process '{app_name}' not found when attempting to kill");
            }
            (ShutdownResult::NotRunning, Vec::new())
        }
        Err(kill_err) => {
            if brp_error.is_some() {
                debug!("Failed to kill process '{app_name}' after BRP failure: {kill_err:?}");
            } else {
                debug!("Failed to kill process '{app_name}': {kill_err:?}");
            }
            let error_message = brp_error.map_or_else(
                || format!("{kill_err:?}"),
                |brp_err| format!("BRP failed: {brp_err}, Kill failed: {kill_err:?}"),
            );
            (
                ShutdownResult::Error {
                    message: error_message,
                },
                Vec::new(),
            )
        }
    }
}

pub struct Shutdown;

impl LocalToolFunction for Shutdown {
    fn call(&self, ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_> {
        let app_name = match ctx.extract_required_string(PARAM_APP_NAME, "app name") {
            Ok(name) => name.to_string(),
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let port = ctx.extract_optional_number("port", u64::from(DEFAULT_BRP_PORT));
        let Ok(port) = u16::try_from(port) else {
            return Box::pin(async move {
                Err(report_to_mcp_error(
                    &error_stack::Report::new(Error::InvalidArgument(
                        "Invalid port parameter".to_string(),
                    ))
                    .attach_printable("Port must be a valid u16")
                    .attach_printable(format!("Provided value: {port}")),
                ))
            });
        };

        Box::pin(async move {
            handle_impl(&app_name, port)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    app_name: &str,
    port: u16,
) -> std::result::Result<ShutdownResultData, McpError> {
    // Shutdown the app
    let (result, _debug_info) = shutdown_app(app_name, port).await;

    // Build and return typed response
    let shutdown_result = match result {
        ShutdownResult::CleanShutdown => {
            let message = format!(
                "Successfully initiated graceful shutdown for '{app_name}' via bevy_brp_extras on port {port}"
            );
            ShutdownResultData {
                status: "success".to_string(),
                shutdown_method: "clean_shutdown".to_string(),
                app_name: app_name.to_string(),
                port,
                pid: None,
                shutdown_message: message,
            }
        }
        ShutdownResult::ProcessKilled { pid } => {
            let message = format!(
                "Terminated process '{app_name}' (PID: {pid}) using kill. Consider adding bevy_brp_extras for clean shutdown."
            );
            ShutdownResultData {
                status: "success".to_string(),
                shutdown_method: "process_kill".to_string(),
                app_name: app_name.to_string(),
                port,
                pid: Some(pid),
                shutdown_message: message,
            }
        }
        ShutdownResult::AlreadyShutdown => {
            let message = format!(
                "Process '{app_name}' is not running - may have already shutdown or crashed. No action needed."
            );
            ShutdownResultData {
                status: "error".to_string(),
                shutdown_method: "already_shutdown".to_string(),
                app_name: app_name.to_string(),
                port,
                pid: None,
                shutdown_message: message,
            }
        }
        ShutdownResult::NotRunning => {
            let message = format!("Process '{app_name}' is not currently running");
            ShutdownResultData {
                status: "error".to_string(),
                shutdown_method: "none".to_string(),
                app_name: app_name.to_string(),
                port,
                pid: None,
                shutdown_message: message,
            }
        }
        ShutdownResult::Error { message } => ShutdownResultData {
            status: "error".to_string(),
            shutdown_method: "process_kill_failed".to_string(),
            app_name: app_name.to_string(),
            port,
            pid: None,
            shutdown_message: message,
        },
    };

    Ok(shutdown_result)
}

/// Try to gracefully shutdown via `bevy_brp_extras`
async fn try_graceful_shutdown(port: u16) -> Result<(bool, Vec<String>)> {
    debug!("Starting graceful shutdown attempt on port {port}");
    match execute_brp_method(BRP_METHOD_EXTRAS_SHUTDOWN, None, Some(port)).await {
        Ok(BrpResult::Success(result)) => {
            // Graceful shutdown succeeded
            debug!("BRP extras shutdown successful: {result:?}");
            Ok((true, Vec::new()))
        }
        Ok(BrpResult::Error(brp_error)) => {
            // Check if this is a method not found error (bevy_brp_extras not available)
            if brp_error.code == JSON_RPC_ERROR_METHOD_NOT_FOUND {
                debug!(
                    "BRP extras method not found (code {}): {}",
                    brp_error.code, brp_error.message
                );
            } else {
                // Other BRP errors also indicate graceful shutdown failed
                debug!(
                    "BRP extras returned error (code {}): {}",
                    brp_error.code, brp_error.message
                );
            }
            Ok((false, Vec::new()))
        }
        Err(e) => {
            // BRP communication failed entirely
            debug!("BRP communication failed: {e}");
            Err(error_stack::Report::new(Error::BrpCommunication(
                "BRP communication failed".to_string(),
            ))
            .attach_printable("BRP not responsive")
            .attach_printable(format!("Port: {port}")))
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
