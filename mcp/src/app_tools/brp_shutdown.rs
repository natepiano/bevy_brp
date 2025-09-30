use bevy_brp_mcp_macros::{ParamStruct, ResultStruct, ToolFn};
use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, get_sockets_info};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sysinfo::{Signal, System};
use tracing::debug;

use crate::brp_tools::{BrpClient, JSON_RPC_ERROR_METHOD_NOT_FOUND, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ShutdownParams {
    /// Name of the Bevy app to shutdown
    pub app_name: String,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:     Port,
}

/// Result from shutting down a Bevy app
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ShutdownResult {
    /// App name that was shut down
    #[to_metadata]
    app_name:         String,
    /// Process ID
    #[to_metadata]
    pid:              u32,
    /// Shutdown method used
    #[to_metadata]
    shutdown_method:  String,
    /// Port where shutdown was attempted
    #[to_metadata]
    port:             u16,
    /// Warning for degraded success (process kill)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    warning:          Option<String>,
    /// Message template for formatting responses
    #[to_message]
    message_template: Option<String>,
}

/// Result of a shutdown operation
enum ShutdownOutcome {
    /// Graceful shutdown via `bevy_brp_extras` succeeded
    CleanShutdown { pid: u32 },
    /// Process was killed using system signal - typically when extras plugin is not available
    ProcessKilled { pid: u32 },
    /// Process was not running
    NotRunning,
    /// An error occurred during shutdown
    Error { message: String },
}

#[derive(ToolFn)]
#[tool_fn(params = "ShutdownParams", output = "ShutdownResult")]
pub struct Shutdown;

/// Attempt to shutdown a Bevy app, first trying graceful shutdown then falling back to kill
async fn shutdown_app(app_name: &str, port: Port) -> ShutdownOutcome {
    debug!("Starting shutdown process for app '{app_name}' on port {port}");
    // First, check if the process is actually running
    if !is_process_running(app_name) {
        debug!("Process '{app_name}' not found in system process list");
        return ShutdownOutcome::NotRunning;
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
            ShutdownOutcome::CleanShutdown { pid }
        }
        Ok(None) => {
            debug!("Graceful shutdown failed, falling back to process kill");
            // BRP responded but bevy_brp_extras not available - fall back to kill
            handle_kill_process_fallback(app_name, port, None)
        }
        Err(e) => {
            debug!("BRP communication error, falling back to process kill: {e}");
            // BRP not responsive - fall back to kill
            handle_kill_process_fallback(app_name, port, Some(e.to_string()))
        }
    }
}

/// Handle the fallback to kill process when graceful shutdown fails
fn handle_kill_process_fallback(
    app_name: &str,
    port: Port,
    brp_error: Option<String>,
) -> ShutdownOutcome {
    match kill_process(app_name, port) {
        Ok(Some(pid)) => {
            debug!("Successfully killed process {app_name} with PID {pid}");
            ShutdownOutcome::ProcessKilled { pid }
        }
        Ok(None) => {
            if brp_error.is_some() {
                debug!("Process '{app_name}' not found when attempting to kill after BRP failure");
            } else {
                debug!("Process '{app_name}' not found when attempting to kill");
            }
            ShutdownOutcome::NotRunning
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
            ShutdownOutcome::Error {
                message: error_message,
            }
        }
    }
}

async fn handle_impl(params: ShutdownParams) -> Result<ShutdownResult> {
    // Shutdown the app
    let result = shutdown_app(&params.app_name, params.port).await;

    // Build and return typed response
    match result {
        ShutdownOutcome::CleanShutdown { pid } => Ok(ShutdownResult::new(
            params.app_name.clone(),
            pid,
            "clean_shutdown".to_string(),
            params.port.0,
            None,
        )
        .with_message_template(format!(
            "Successfully initiated graceful shutdown for '{}' (PID: {pid}) via bevy_brp_extras",
            params.app_name
        ))),
        ShutdownOutcome::ProcessKilled { pid } => Ok(ShutdownResult::new(
            params.app_name.clone(),
            pid,
            "process_kill".to_string(),
            params.port.0,
            Some("Consider adding bevy_brp_extras for clean shutdown".to_string()),
        )
        .with_message_template(format!(
            "Terminated process '{}' (PID: {pid}) using kill",
            params.app_name
        ))),
        ShutdownOutcome::NotRunning => Err(Error::Structured {
            result: Box::new(ProcessNotRunningError::new(params.app_name.clone())),
        })?,
        ShutdownOutcome::Error { message } => Err(Error::Structured {
            result: Box::new(ShutdownFailedError::new(params.app_name, message)),
        })?,
    }
}

/// Try to gracefully shutdown via `bevy_brp_extras`
async fn try_graceful_shutdown(port: Port) -> Result<Option<serde_json::Value>> {
    debug!("Starting graceful shutdown attempt on port {port}");
    let client = BrpClient::new(BrpMethod::BrpShutdown, port, None);
    match client.execute_raw().await {
        Ok(ResponseStatus::Success(result)) => {
            // Graceful shutdown succeeded
            debug!("BRP extras shutdown successful: {result:?}");
            Ok(result)
        }
        Ok(ResponseStatus::Error(brp_error)) => {
            // Check if this is a method not found error (bevy_brp_extras not available)
            if brp_error.get_code() == JSON_RPC_ERROR_METHOD_NOT_FOUND {
                debug!(
                    "BRP extras method not found (code {}): {}",
                    brp_error.get_code(),
                    brp_error.get_message()
                );
            } else {
                // Other BRP errors also indicate graceful shutdown failed
                debug!(
                    "BRP extras returned error (code {}): {}",
                    brp_error.get_code(),
                    brp_error.get_message()
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
            .attach("BRP not responsive")
            .attach(format!("Port: {port}")))
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

/// Get the PID for a process listening on the specified port
fn get_pid_for_port(port: Port) -> Option<u32> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP;

    get_sockets_info(af_flags, proto_flags)
        .ok()?
        .into_iter()
        .find_map(|si| {
            if let ProtocolSocketInfo::Tcp(tcp_si) = si.protocol_socket_info {
                if tcp_si.local_port == *port {
                    return si.associated_pids.first().copied();
                }
            }
            None
        })
}

/// Kill the process using the system signal
fn kill_process(app_name: &str, port: Port) -> Result<Option<u32>> {
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // First try: Get PID from port for more reliable process identification
    let target_pid = if let Some(pid) = get_pid_for_port(port) {
        debug!("Found PID {pid} listening on port {port}");
        // Verify the process name matches to ensure we're killing the right process
        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            let process_name = process.name().to_string_lossy();
            if process_name == app_name
                || process_name == format!("{app_name}.exe")
                || process_name.strip_suffix(".exe").unwrap_or(&process_name) == app_name
            {
                debug!("Verified process name matches: {process_name}");
                Some(pid)
            } else {
                debug!(
                    "Process name mismatch: expected '{app_name}', found '{process_name}' for PID \
                     {pid}"
                );
                None
            }
        } else {
            debug!("PID {pid} not found in process list");
            None
        }
    } else {
        debug!("No process found listening on port {port}, falling back to name-only lookup");
        None
    };

    // If port-based lookup succeeded, kill that specific PID
    if let Some(pid) = target_pid {
        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            if process.kill_with(Signal::Term).unwrap_or(false) {
                debug!("Successfully killed process {app_name} (PID {pid}) via port lookup");
                return Ok(Some(pid));
            }
            return Err(error_stack::Report::new(Error::ProcessManagement(
                "Failed to terminate process".to_string(),
            ))
            .attach(format!("Process name: {app_name}"))
            .attach(format!("PID: {pid}"))
            .attach(format!("Port: {port}"))
            .attach("Failed to send SIGTERM signal"));
        }
    }

    // Fallback: Use name-only lookup (original behavior)
    debug!("Falling back to name-only process lookup for '{app_name}'");
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
            debug!("Successfully killed process {app_name} (PID {pid}) via name-only lookup");
            Ok(Some(pid))
        } else {
            Err(error_stack::Report::new(Error::ProcessManagement(
                "Failed to terminate process".to_string(),
            ))
            .attach(format!("Process name: {app_name}"))
            .attach(format!("PID: {pid}"))
            .attach("Failed to send SIGTERM signal"))
        }
    })
}

/// Error when process is not running
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ProcessNotRunningError {
    #[to_error_info]
    app_name: String,

    #[to_message(message_template = "Process '{app_name}' is not currently running")]
    message_template: String,
}

/// Error when shutdown fails
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ShutdownFailedError {
    #[to_error_info]
    app_name: String,

    #[to_error_info]
    error_details: String,

    #[to_message(message_template = "Failed to shutdown '{app_name}': {error_details}")]
    message_template: String,
}
