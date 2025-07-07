use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::tracing::get_trace_log_path;

/// Handle the `brp_get_trace_log_path` tool request
pub fn handle() -> Result<CallToolResult, McpError> {
    // Get the trace log path
    let log_path = get_trace_log_path();
    let log_path_str = log_path.to_string_lossy().to_string();

    // Check if the file exists and get its size
    let (exists, file_size) =
        std::fs::metadata(&log_path).map_or((false, None), |metadata| (true, Some(metadata.len())));

    let message = if exists {
        format!("Trace log file found at: {log_path_str}")
    } else {
        format!("Trace log file not found (will be created when logging starts): {log_path_str}")
    };

    let mut response_builder = ResponseBuilder::success()
        .message(&message)
        .add_field("log_path", &log_path_str)
        .map_err(|e| McpError::internal_error(format!("Failed to add log_path field: {e}"), None))?
        .add_field("exists", exists)
        .map_err(|e| McpError::internal_error(format!("Failed to add exists field: {e}"), None))?;

    if let Some(size) = file_size {
        response_builder = response_builder
            .add_field("file_size_bytes", size)
            .map_err(|e| {
                McpError::internal_error(format!("Failed to add file_size_bytes field: {e}"), None)
            })?;
    }

    let response = response_builder
        .auto_inject_debug_info(None::<&serde_json::Value>)
        .build();

    Ok(json_response_to_result(&response))
}
