use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use sysinfo::Process;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;

use super::constants::CARGO_BIN_FLAG;
use super::constants::CARGO_COMMAND_NAME;
use super::constants::CARGO_EXAMPLE_FLAG;
use super::constants::CARGO_RUN_SUBCOMMAND;
use super::constants::EXAMPLES_PATH_SEGMENT;
use super::constants::GENERIC_PROCESS_NAMES;
use super::constants::MCP_BINARY_NAME;
use super::constants::STATUS_MAX_RETRIES;
use super::constants::STATUS_POLL_INTERVAL;
use super::constants::TARGET_DEBUG_PATH;
use super::constants::TARGET_RELEASE_PATH;
use super::process;
use crate::brp_tools;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::error::Error;
use crate::error::Result;
use crate::tool::BrpMethod;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct StatusParams {
    /// Name of the process to check for
    pub app_name: String,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:     Port,
}

/// Result from checking status of a Bevy app
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct StatusResult {
    /// App name
    #[to_metadata]
    app_name:         String,
    /// Process ID
    #[to_metadata]
    pid:              u32,
    /// Port where BRP is responding
    #[to_metadata]
    port:             u16,
    /// Message template for formatting responses
    #[to_message(
        message_template = "Process '{app_name}' (PID: {pid}) is running with BRP enabled on port {port}"
    )]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "StatusParams", output = "StatusResult")]
pub struct Status;

async fn handle_impl(params: StatusParams) -> Result<StatusResult> {
    check_brp_for_app(&params.app_name, params.port).await
}

/// Error when process is not found
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
struct ProcessNotFoundError {
    #[to_error_info]
    app_name: String,

    #[to_error_info(skip_if_none)]
    similar_app_name: Option<String>,

    #[serde(rename = "brp_responding_on_port")]
    #[to_error_info]
    brp_port_status: BrpPortStatus,

    #[to_error_info]
    port: u16,

    #[to_message]
    message_template: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
enum BrpPortStatus {
    NotResponding,
    Responding,
}

impl From<bool> for BrpPortStatus {
    fn from(value: bool) -> Self {
        if value {
            Self::Responding
        } else {
            Self::NotResponding
        }
    }
}

impl From<BrpPortStatus> for bool {
    fn from(value: BrpPortStatus) -> Self { matches!(value, BrpPortStatus::Responding) }
}

impl BrpPortStatus {
    const fn is_responding(self) -> bool { matches!(self, Self::Responding) }
}

/// Error when process is running but BRP not responding
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
struct BrpNotRespondingError {
    #[to_error_info]
    app_name: String,

    #[to_error_info]
    pid: u32,

    #[to_error_info]
    port: u16,

    #[to_message(
        message_template = "Process '{app_name}' (PID: {pid}) is running but not responding to BRP on port {port}. Make sure RemotePlugin is added to your Bevy app."
    )]
    message_template: String,
}

/// Check if process matches the target app name with substring match
fn process_matches_app_substring(process: &Process, target_app: &str) -> bool {
    let normalized_target = process::normalize_process_name(target_app);

    // Check process name
    let process_name = process.name().to_string_lossy();
    let normalized_process_name = process::normalize_process_name(&process_name);

    if normalized_process_name.contains(&normalized_target) {
        return true;
    }

    // Check first command argument (usually the binary path) for substring matches
    // but skip generic process names that wouldn't be helpful
    if let Some(command_path) = process.cmd().first() {
        let command_string = command_path.to_string_lossy();
        let normalized_command = process::normalize_process_name(&command_string);

        // Skip if it's a generic utility that happens to have the target in its args
        if !GENERIC_PROCESS_NAMES.contains(&normalized_process_name.as_str())
            && normalized_command.contains(&normalized_target)
        {
            return true;
        }
    }

    false
}

/// Check if process is `bevy_brp_mcp` (the MCP tool itself)
fn is_bevy_brp_mcp(process: &Process) -> bool {
    let process_name = process.name().to_string_lossy();
    process_name == MCP_BINARY_NAME
}

/// Extract clean app name from process for suggestions
fn extract_app_name(process: &Process) -> String {
    let process_name = process.name().to_string_lossy();

    // Check if it's running through cargo
    if process_name == CARGO_COMMAND_NAME {
        // Look for "run" and then the binary name in args
        let command_line_arguments: Vec<String> = process
            .cmd()
            .iter()
            .map(|argument| argument.to_string_lossy().to_string())
            .collect();
        if command_line_arguments
            .iter()
            .any(|argument| argument == CARGO_RUN_SUBCOMMAND)
        {
            // Check for --bin argument
            if let Some(binary_position) = command_line_arguments
                .iter()
                .position(|argument| argument == CARGO_BIN_FLAG)
                && let Some(binary_name) = command_line_arguments.get(binary_position + 1)
            {
                return binary_name.clone();
            }
            // Check for --example argument
            if let Some(example_position) = command_line_arguments
                .iter()
                .position(|argument| argument == CARGO_EXAMPLE_FLAG)
                && let Some(example_name) = command_line_arguments.get(example_position + 1)
            {
                return example_name.clone();
            }
        }
    }

    // If the process name looks like a path to a binary, extract just the binary name
    if process_name.contains(TARGET_DEBUG_PATH) || process_name.contains(TARGET_RELEASE_PATH) {
        return process::normalize_process_name(&process_name);
    }

    // For processes run directly, check the first command argument
    if let Some(command_path) = process.cmd().first() {
        let command_string = command_path.to_string_lossy();
        if command_string.contains(TARGET_DEBUG_PATH)
            || command_string.contains(TARGET_RELEASE_PATH)
            || command_string.contains(EXAMPLES_PATH_SEGMENT)
        {
            return process::normalize_process_name(&command_string);
        }
    }

    process::normalize_process_name(&process_name)
}

async fn check_brp_for_app(app_name: &str, port: Port) -> Result<StatusResult> {
    let brp_port_status = check_brp_on_port(port).await?;
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    if let Some(process_id) = process::get_pid_for_port(port) {
        return resolve_pid_on_port(&system, app_name, port, brp_port_status, process_id);
    }

    if let Some(process_id) = find_exact_match_pid(&system, app_name) {
        Err(Error::Structured {
            result: Box::new(BrpNotRespondingError::new(
                app_name.to_string(),
                process_id,
                port.0,
            )),
        })?;
    }

    build_missing_process_result(
        app_name,
        collect_similar_app_names(&system, app_name)
            .first()
            .cloned(),
        brp_port_status,
        port,
    )
}

fn resolve_pid_on_port(
    system: &System,
    app_name: &str,
    port: Port,
    brp_port_status: BrpPortStatus,
    process_id: u32,
) -> Result<StatusResult> {
    if let Some(process) = system.process(sysinfo::Pid::from_u32(process_id))
        && process::process_matches_name_exact(process, app_name)
    {
        if brp_port_status.is_responding() {
            return Ok(StatusResult::new(app_name.to_string(), process_id, port.0));
        }

        Err(Error::Structured {
            result: Box::new(BrpNotRespondingError::new(
                app_name.to_string(),
                process_id,
                port.0,
            )),
        })?;
    }

    build_missing_process_result(app_name, None, brp_port_status, port)
}

fn build_missing_process_result(
    app_name: &str,
    similar_app_name: Option<String>,
    brp_port_status: BrpPortStatus,
    port: Port,
) -> Result<StatusResult> {
    let process_not_found_error = ProcessNotFoundError::new(
        app_name.to_string(),
        similar_app_name.clone(),
        brp_port_status,
        port.0,
    )
    .with_message_template(missing_process_message(
        app_name,
        similar_app_name.as_deref(),
        brp_port_status,
        port,
    ));

    Err(Error::Structured {
        result: Box::new(process_not_found_error),
    })?
}

fn missing_process_message(
    app_name: &str,
    similar_app_name: Option<&str>,
    brp_port_status: BrpPortStatus,
    port: Port,
) -> String {
    match (similar_app_name, brp_port_status) {
        (Some(suggestion), BrpPortStatus::Responding) => {
            format!(
                "Process '{app_name}' not found. Did you mean: {suggestion}? (BRP is responding on port {})",
                port.0
            )
        },
        (Some(suggestion), BrpPortStatus::NotResponding) => {
            format!("Process '{app_name}' not found. Did you mean: {suggestion}?")
        },
        (None, BrpPortStatus::Responding) => {
            format!(
                "Process '{app_name}' not found. BRP is responding on port {} - another process may be using it.",
                port.0
            )
        },
        (None, BrpPortStatus::NotResponding) => {
            format!(
                "Process '{app_name}' not found and BRP is not responding on port {}.",
                port.0
            )
        },
    }
}

fn find_exact_match_pid(system: &System, app_name: &str) -> Option<u32> {
    system.processes().values().find_map(|process| {
        (!matches!(process.status(), sysinfo::ProcessStatus::Zombie)
            && process::process_matches_name_exact(process, app_name))
        .then(|| process.pid().as_u32())
    })
}

fn collect_similar_app_names(system: &System, app_name: &str) -> Vec<String> {
    system
        .processes()
        .values()
        .filter(|process| {
            !matches!(process.status(), sysinfo::ProcessStatus::Zombie)
                && process_matches_app_substring(process, app_name)
                && !is_bevy_brp_mcp(process)
        })
        .map(extract_app_name)
        .collect()
}

/// Check if BRP is responding on the given port
async fn check_brp_on_port(port: Port) -> Result<BrpPortStatus> {
    // Retry with delays to account for BRP initialization timing
    for _ in 0..STATUS_MAX_RETRIES {
        let client = brp_tools::BrpClient::new(BrpMethod::WorldListComponents, port, None);
        match client.execute_raw().await {
            Ok(ResponseStatus::Success(_)) => {
                // BRP is responding and working
                return Ok(BrpPortStatus::Responding);
            },
            Ok(ResponseStatus::Error(_)) | Err(_) => {
                // BRP not responding or returned an error, wait and retry
                tokio::time::sleep(STATUS_POLL_INTERVAL).await;
            },
        }
    }

    // After all retries failed
    Ok(BrpPortStatus::NotResponding)
}
