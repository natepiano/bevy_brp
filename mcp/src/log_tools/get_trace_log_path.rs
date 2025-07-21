use serde::{Deserialize, Serialize};

use super::tracing::get_trace_log_path;
use crate::tool::{HandlerContext, HandlerResponse, LocalToolFn, NoMethod, NoPort};

/// Result from getting the trace log path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTraceLogPathResult {
    /// Full path to the trace log file
    pub log_path:        String,
    /// Whether the log file currently exists
    pub exists:          bool,
    /// Size of the log file in bytes (if it exists)
    pub file_size_bytes: Option<u64>,
}

/// Handler for the `brp_get_trace_log_path` tool using the `LocalFn` approach
pub struct GetTraceLogPath;

impl LocalToolFn for GetTraceLogPath {
    type Output = GetTraceLogPathResult;

    fn call(&self, _ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<Self::Output> {
        Box::pin(async move {
            // Get the trace log path
            let log_path = get_trace_log_path();
            let log_path_str = log_path.to_string_lossy().to_string();

            // Check if the file exists and get its size
            let (exists, file_size_bytes) = std::fs::metadata(&log_path)
                .map_or((false, None), |metadata| (true, Some(metadata.len())));

            let result = GetTraceLogPathResult {
                log_path: log_path_str,
                exists,
                file_size_bytes,
            };

            Ok(result)
        })
    }
}
