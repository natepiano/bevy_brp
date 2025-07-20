use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::brp_tools::support::brp_client::{BrpResult, execute_brp_method};
use crate::tool::{
    BRP_METHOD_LIST, HandlerContext, HandlerResponse, HandlerResult, HasPort, LocalToolFnWithPort,
    NoMethod, ParameterName,
};

/// Result from checking status of a Bevy app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    /// Status of the check operation
    pub status:         String,
    /// App name that was checked
    pub app_name:       String,
    /// Whether the app process is running
    pub app_running:    bool,
    /// Whether BRP is responsive
    pub brp_responsive: bool,
    /// Process ID if running
    pub pid:            Option<u32>,
    /// Status message
    pub message:        String,
}

impl HandlerResult for StatusResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct Status;

impl LocalToolFnWithPort for Status {
    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<'_> {
        let app_name = match ctx.extract_required(ParameterName::AppName) {
            Ok(value) => match value.into_string() {
                Ok(s) => s,
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let port = ctx.port();
        Box::pin(async move {
            handle_impl(&app_name, port)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(app_name: &str, port: u16) -> std::result::Result<StatusResult, McpError> {
    check_brp_for_app(app_name, port).await
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

async fn check_brp_for_app(
    app_name: &str,
    port: u16,
) -> std::result::Result<StatusResult, McpError> {
    // Check if a process with this name is running using sysinfo
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let running_process = system.processes().values().find(|process| {
        // Filter out defunct/zombie processes
        !matches!(process.status(), sysinfo::ProcessStatus::Zombie)
            && process_matches_app(process, app_name)
    });

    // Check BRP connectivity
    let brp_responsive = check_brp_on_port(port).await?;

    // Build response based on findings
    let (status, message, app_running, pid) = match (running_process, brp_responsive) {
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

    Ok(StatusResult {
        status: status.to_string(),
        app_name: app_name.to_string(),
        app_running,
        brp_responsive,
        pid,
        message,
    })
}

/// Check if BRP is responding on the given port
async fn check_brp_on_port(port: u16) -> std::result::Result<bool, McpError> {
    // Try a simple BRP request to check connectivity using bevy/list
    match execute_brp_method(BRP_METHOD_LIST, None, port).await {
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
