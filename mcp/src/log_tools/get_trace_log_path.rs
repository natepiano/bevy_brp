use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};

use super::tracing::get_trace_log_path;
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

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

/// Handler for the `brp_get_trace_log_path` tool using the `LocalFn` approach
pub struct GetTraceLogPath;

impl ToolFn for GetTraceLogPath {
    type Output = GetTraceLogPathResult;

    fn call(&self, _ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output>> {
        Box::pin(async move {
            // Get the trace log path
            let log_path = get_trace_log_path();
            let log_path_str = log_path.to_string_lossy().to_string();

            // Check if the file exists and get its size
            let (exists, file_size_bytes) = std::fs::metadata(&log_path)
                .map_or((false, None), |metadata| (true, Some(metadata.len())));

            let result = Ok(GetTraceLogPathResult::new(
                log_path_str,
                exists,
                file_size_bytes,
            ));

            Ok(ToolResult::without_port(result))
        })
    }
}
