use bevy_brp_mcp_macros::{ResultStruct, ToolFn};
use serde::{Deserialize, Serialize};

use super::TracingLevel;
use crate::tool::{HandlerContext, HandlerResult, NoParams, ToolFn, ToolResult};

/// Result from getting the trace log path
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct GetTraceLogPathResult {
    /// Full path to the trace log file
    #[to_metadata]
    log_path:         String,
    /// Whether the log file currently exists
    #[to_metadata]
    exists:           bool,
    /// Size of the log file in bytes (if it exists)
    #[to_metadata(skip_if_none)]
    file_size_bytes:  Option<u64>,
    /// Message template for formatting responses
    #[to_message(message_template = "Trace log path: {log_path}")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "NoParams", output = "GetTraceLogPathResult")]
pub struct GetTraceLogPath;

async fn handle_impl(_params: NoParams) -> crate::error::Result<GetTraceLogPathResult> {
    // Get the trace log path
    let log_path = TracingLevel::get_trace_log_path();

    let log_path_str = log_path.to_string_lossy().to_string();

    // Check if the file exists and get its size
    let (exists, file_size_bytes) =
        std::fs::metadata(&log_path).map_or((false, None), |metadata| (true, Some(metadata.len())));

    Ok(GetTraceLogPathResult::new(
        log_path_str,
        exists,
        file_size_bytes,
    ))
}
