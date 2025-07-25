use bevy_brp_mcp_macros::ResultFieldPlacement;
use serde::{Deserialize, Serialize};

use super::tracing::get_trace_log_path;
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

/// Result from getting the trace log path
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
pub struct GetTraceLogPathResult {
    /// Full path to the trace log file
    #[to_metadata]
    pub log_path:        String,
    /// Whether the log file currently exists
    #[to_metadata]
    pub exists:          bool,
    /// Size of the log file in bytes (if it exists)
    #[to_metadata(skip_if_none)]
    pub file_size_bytes: Option<u64>,
}

/// Handler for the `brp_get_trace_log_path` tool using the `LocalFn` approach
pub struct GetTraceLogPath;

impl ToolFn for GetTraceLogPath {
    type Output = GetTraceLogPathResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        _ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            // Get the trace log path
            let log_path = get_trace_log_path();
            let log_path_str = log_path.to_string_lossy().to_string();

            // Check if the file exists and get its size
            let (exists, file_size_bytes) = std::fs::metadata(&log_path)
                .map_or((false, None), |metadata| (true, Some(metadata.len())));

            let result = Ok(GetTraceLogPathResult {
                log_path: log_path_str,
                exists,
                file_size_bytes,
            });

            Ok(ToolResult::from_result(result, LocalCallInfo))
        })
    }
}
