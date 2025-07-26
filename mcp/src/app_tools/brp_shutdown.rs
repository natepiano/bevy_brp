use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sysinfo::{Signal, System};
use tracing::debug;

use crate::brp_tools::{
    BrpResult, JSON_RPC_ERROR_METHOD_NOT_FOUND, default_port, deserialize_port, execute_brp_method,
};
use crate::error::{Error, Result};
use crate::tool::{
    BrpMethod, HandlerContext, HandlerResult, LocalWithPortCallInfo, ToolFn, ToolResult,
};

#[derive(Deserialize, JsonSchema, ParamStruct)]
pub struct ShutdownParams {
    /// Name of the Bevy app to shutdown
    #[to_metadata]
    pub app_name: String,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port:     u16,
}

/// Result from shutting down a Bevy app
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ShutdownResultData {
    /// Status of the shutdown operation
    #[to_metadata]
    status:           String,
    /// Shutdown method used
    #[to_metadata]
    shutdown_method:  String,
    /// App name that was shut down
    #[to_metadata]
    app_name:         String,
    /// Process ID if terminated via kill
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pid:              Option<u32>,
    /// Message template for formatting responses
    #[to_message]
    message_template: Option<String>,
}

/// Result of a shutdown operation
enum ShutdownResult {
    /// Graceful shutdown via `bevy_brp_extras` succeeded
    CleanShutdown { pid: u32 },
    /// Process was killed using system signal - typically when extras plugin is not available
    ProcessKilled { pid: u32 },
    /// Process was not running
    NotRunning,
    /// An error occurred during shutdown
    Error { message: String },
}

pub struct Shutdown;

impl ToolFn for Shutdown {
    type Output = ShutdownResultData;
    type CallInfoData = LocalWithPortCallInfo;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            let params: ShutdownParams = ctx.extract_parameter_values()?;
            let port = params.port;

            let result = handle_impl(&params.app_name, port).await;
            Ok(ToolResult::from_result(
                result,
                LocalWithPortCallInfo { port },
            ))
        })
    }
}

/// Attempt to shutdown a Bevy app, first trying graceful shutdown then falling back to kill
async fn shutdown_app(app_name: &str, port: u16) -> ShutdownResult {
    debug!("Starting shutdown process for app '{app_name}' on port {port}");
    // First, check if the process is actually running
    if !is_process_running(app_name) {
        debug!("Process '{app_name}' not found in system process list");
        return ShutdownResult::NotRunning;
    }

    debug!("Process '{app_name}' found, attempting graceful shutdown");

    // Process is running, try graceful shutdown via bevy_brp_extras
    // Extraction shouldn't return 0 with the udpated data extras but it's possible we could be
    // running against an older version
    match try_graceful_shutdown(port).await {
        Ok(Some(result)) => {
            debug!("Graceful shutdown succeeded");
            // Extract PID from the BRP response
            let pid = result
                .get("pid")
                .and_then(serde_json::Value::as_u64)
                .and_then(|p| u32::try_from(p).ok())
                .unwrap_or_else(|| {
                    debug!("Warning: PID not found in BRP extras shutdown response");
                    0
                });
            ShutdownResult::CleanShutdown { pid }
        }
        Ok(None) => {
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
fn handle_kill_process_fallback(app_name: &str, brp_error: Option<String>) -> ShutdownResult {
    match kill_process(app_name) {
        Ok(Some(pid)) => {
            debug!("Successfully killed process {app_name} with PID {pid}");
            ShutdownResult::ProcessKilled { pid }
        }
        Ok(None) => {
            if brp_error.is_some() {
                debug!("Process '{app_name}' not found when attempting to kill after BRP failure");
            } else {
                debug!("Process '{app_name}' not found when attempting to kill");
            }
            ShutdownResult::NotRunning
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
            ShutdownResult::Error {
                message: error_message,
            }
        }
    }
}

async fn handle_impl(app_name: &str, port: u16) -> Result<ShutdownResultData> {
    // Shutdown the app
    let result = shutdown_app(app_name, port).await;

    // Build and return typed response
    let shutdown_result = match result {
        ShutdownResult::CleanShutdown { pid } => {
            let message = format!(
                "Successfully initiated graceful shutdown for '{app_name}' (PID: {pid}) via bevy_brp_extras"
            );
            ShutdownResultData::new(
                "success".to_string(),
                "clean_shutdown".to_string(),
                app_name.to_string(),
                Some(pid),
            )
            .with_message_template(message)
        }
        ShutdownResult::ProcessKilled { pid } => {
            let message = format!(
                "Terminated process '{app_name}' (PID: {pid}) using kill. Consider adding bevy_brp_extras for clean shutdown."
            );
            ShutdownResultData::new(
                "success".to_string(),
                "process_kill".to_string(),
                app_name.to_string(),
                Some(pid),
            )
            .with_message_template(message)
        }
        ShutdownResult::NotRunning => {
            let message = format!("Process '{app_name}' is not currently running");
            ShutdownResultData::new(
                "error".to_string(),
                "none".to_string(),
                app_name.to_string(),
                None,
            )
            .with_message_template(message)
        }
        ShutdownResult::Error { message } => ShutdownResultData::new(
            "error".to_string(),
            "process_kill_failed".to_string(),
            app_name.to_string(),
            None,
        )
        .with_message_template(message),
    };

    Ok(shutdown_result)
}

/// Try to gracefully shutdown via `bevy_brp_extras`
async fn try_graceful_shutdown(port: u16) -> Result<Option<serde_json::Value>> {
    debug!("Starting graceful shutdown attempt on port {port}");
    match execute_brp_method(BrpMethod::BrpShutdown, None, port).await {
        Ok(BrpResult::Success(result)) => {
            // Graceful shutdown succeeded
            debug!("BRP extras shutdown successful: {result:?}");
            Ok(result)
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
            Ok(None)
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
