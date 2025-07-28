use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::brp_tools::{self, BrpResult, Port};
use crate::error::Result;
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct StatusParams {
    /// Name of the process to check for
    pub app_name: String,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:     Port,
}

/// Result from checking status of a Bevy app
///
/// Note: This struct has private fields and can only be constructed via `StatusResult::new()`
/// due to the `#[to_message]` attribute. This ensures the message template is always set.
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct StatusResult {
    /// Status of the check - "success" only if app found and BRP responding
    #[to_metadata]
    status:             String,
    /// App name that was requested
    #[to_metadata]
    app_name_requested: String,
    /// Whether the app process was found
    #[to_metadata]
    app_found:          bool,
    /// Whether BRP is responding on the port
    #[to_metadata]
    responding_on_port: bool,
    /// Process ID if running
    #[to_metadata(skip_if_none)]
    pid:                Option<u32>,
    /// Similar app name if no exact match found
    #[to_metadata(skip_if_none)]
    similar_app_name:   Option<String>,
    /// Message template for formatting responses
    #[to_message]
    message_template:   Option<String>,
}

pub struct Status;

impl ToolFn for Status {
    type Output = StatusResult;
    type Params = StatusParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            let params: StatusParams = ctx.extract_parameter_values()?;
            let port = params.port;
            let result = handle_impl(&params.app_name, port).await;
            Ok(ToolResult {
                result,
                params: Some(params),
            })
        })
    }
}

async fn handle_impl(app_name: &str, port: Port) -> Result<StatusResult> {
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

/// Check if process matches the target app name with exact match
fn process_matches_app_exact(process: &sysinfo::Process, target_app: &str) -> bool {
    let normalized_target = normalize_process_name(target_app);

    // Check process name
    let process_name = process.name().to_string_lossy();
    let normalized_process_name = normalize_process_name(&process_name);

    if normalized_process_name == normalized_target {
        return true;
    }

    // Check exact match on binary name from command
    if let Some(cmd) = process.cmd().first() {
        let cmd_normalized = normalize_process_name(&cmd.to_string_lossy());
        if cmd_normalized == normalized_target {
            return true;
        }
    }

    false
}

/// Check if process matches the target app name with substring match
fn process_matches_app_substring(process: &sysinfo::Process, target_app: &str) -> bool {
    let normalized_target = normalize_process_name(target_app);

    // Check process name
    let process_name = process.name().to_string_lossy();
    let normalized_process_name = normalize_process_name(&process_name);

    if normalized_process_name.contains(&normalized_target) {
        return true;
    }

    // Check first command argument (usually the binary path) for substring matches
    // but skip generic process names that wouldn't be helpful
    if let Some(cmd) = process.cmd().first() {
        let cmd_str = cmd.to_string_lossy();
        let cmd_normalized = normalize_process_name(&cmd_str);

        // Skip if it's a generic utility that happens to have the target in its args
        let generic_utils = ["tail", "grep", "cat", "less", "more", "head", "sed", "awk"];
        if !generic_utils.contains(&normalized_process_name.as_str())
            && cmd_normalized.contains(&normalized_target)
        {
            return true;
        }
    }

    false
}

/// Check if process is `bevy_brp_mcp` (the MCP tool itself)
fn is_bevy_brp_mcp(process: &sysinfo::Process) -> bool {
    let process_name = process.name().to_string_lossy();
    process_name == "bevy_brp_mcp"
}

/// Extract clean app name from process for suggestions
fn extract_app_name(process: &sysinfo::Process) -> String {
    let process_name = process.name().to_string_lossy();

    // Check if it's running through cargo
    if process_name == "cargo" {
        // Look for "run" and then the binary name in args
        let args: Vec<String> = process
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        if let Some(_run_pos) = args.iter().position(|arg| arg == "run") {
            // Check for --bin argument
            if let Some(bin_pos) = args.iter().position(|arg| arg == "--bin") {
                if let Some(bin_name) = args.get(bin_pos + 1) {
                    return bin_name.clone();
                }
            }
            // Check for --example argument
            if let Some(ex_pos) = args.iter().position(|arg| arg == "--example") {
                if let Some(ex_name) = args.get(ex_pos + 1) {
                    return ex_name.clone();
                }
            }
        }
    }

    // If the process name looks like a path to a binary, extract just the binary name
    if process_name.contains("target/debug") || process_name.contains("target/release") {
        return normalize_process_name(&process_name);
    }

    // For processes run directly, check the first command argument
    if let Some(cmd) = process.cmd().first() {
        let cmd_str = cmd.to_string_lossy();
        if cmd_str.contains("target/debug")
            || cmd_str.contains("target/release")
            || cmd_str.contains("/examples/")
        {
            return normalize_process_name(&cmd_str);
        }
    }

    normalize_process_name(&process_name)
}

async fn check_brp_for_app(app_name: &str, port: Port) -> Result<StatusResult> {
    // Check if a process with this name is running using sysinfo
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // First try exact match
    let exact_match = system.processes().values().find(|process| {
        // Filter out defunct/zombie processes
        !matches!(process.status(), sysinfo::ProcessStatus::Zombie)
            && process_matches_app_exact(process, app_name)
    });

    // Check BRP connectivity
    let brp_responsive = check_brp_on_port(port).await?;

    exact_match.map_or_else(|| {
        // No exact match found, look for suggestions
        let suggestions: Vec<String> = system
            .processes()
            .values()
            .filter(|process| {
                !matches!(process.status(), sysinfo::ProcessStatus::Zombie)
                    && process_matches_app_substring(process, app_name)
                    && !is_bevy_brp_mcp(process)
            })
            .map(extract_app_name)
            .collect();

        // Pick the first suggestion if available
        let similar_app = suggestions.first().cloned();

        let message = match (similar_app.as_ref(), brp_responsive) {
            (Some(suggestion), true) => {
                format!(
                    "Process '{app_name}' not found. Did you mean: {suggestion}? (BRP is responding on port {port})"
                )
            }
            (Some(suggestion), false) => {
                format!("Process '{app_name}' not found. Did you mean: {suggestion}?")
            }
            (None, true) => {
                format!(
                    "Process '{app_name}' not found. BRP is responding on port {port} - another process may be using it."
                )
            }
            (None, false) => {
                format!("Process '{app_name}' not found and BRP is not responding on port {port}.")
            }
        };

        Ok(StatusResult::new(
            "error".to_string(),
            app_name.to_string(),
            false,
            brp_responsive,
            None,
            similar_app,
        ).with_message_template(message))
    }, |process| {
        // Found exact match
        let pid = process.pid().as_u32();

        if brp_responsive {
            // SUCCESS: Both conditions met
            Ok(StatusResult::new(
                "success".to_string(),
                app_name.to_string(),
                true,
                true,
                Some(pid),
                None,
            ).with_message_template(format!(
                "Process '{app_name}' (PID: {pid}) is running with BRP enabled on port {port}"
            )))
        } else {
            // Process running but BRP not responding
            Ok(StatusResult::new(
                "error".to_string(),
                app_name.to_string(),
                true,
                false,
                Some(pid),
                None,
            ).with_message_template(format!(
                "Process '{app_name}' (PID: {pid}) is running but not responding to BRP on port {port}. Make sure RemotePlugin is added to your Bevy app."
            )))
        }
    })
}

/// Check if BRP is responding on the given port
async fn check_brp_on_port(port: Port) -> Result<bool> {
    // Try a simple BRP request to check connectivity using bevy/list

    match brp_tools::execute_brp_method(BrpMethod::BevyList, None, port).await {
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
