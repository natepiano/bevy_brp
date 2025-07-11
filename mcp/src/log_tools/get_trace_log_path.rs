use serde::{Deserialize, Serialize};

use crate::service::HandlerContext;
use crate::support::tracing::get_trace_log_path;
use crate::tool::{HandlerResponse, HandlerResult, LocalHandler};

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

impl HandlerResult for GetTraceLogPathResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Handler for the `brp_get_trace_log_path` tool using the `LocalFn` approach
pub struct GetTraceLogPath;

impl LocalHandler for GetTraceLogPath {
    fn handle(&self, _ctx: &HandlerContext) -> HandlerResponse<'_> {
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

            Ok(Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}
