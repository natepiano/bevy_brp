use rmcp::model::{CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json;

use crate::BrpMcpService;
use crate::error::Result;
use crate::support::response::{JsonResponse, ResponseBuilder};
use crate::support::schema;
use crate::support::serialization::json_response_to_result;
use crate::support::tracing::get_trace_log_path;

/// Handle the `get_trace_log_path` tool request
pub fn handle_get_trace_log_path(
    _service: &BrpMcpService,
    _request: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> std::result::Result<CallToolResult, McpError> {
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

    let response = match build_response(&message, &log_path_str, exists, file_size) {
        Ok(resp) => resp,
        Err(err) => return Err(crate::error::report_to_mcp_error(&err)),
    };

    Ok(json_response_to_result(&response))
}

fn build_response(
    message: &str,
    log_path: &str,
    exists: bool,
    file_size: Option<u64>,
) -> Result<JsonResponse> {
    let mut response_builder = ResponseBuilder::success()
        .message(message)
        .add_field("log_path", log_path)?
        .add_field("exists", exists)?;

    if let Some(size) = file_size {
        response_builder = response_builder.add_field("file_size_bytes", size)?;
    }

    let response = response_builder
        .auto_inject_debug_info(None::<&serde_json::Value>)
        .build();
    Ok(response)
}

/// Register the `get_trace_log_path` tool
pub fn register_tool() -> Tool {
    use crate::tools::{DESC_BRP_GET_TRACE_LOG_PATH, TOOL_BRP_GET_TRACE_LOG_PATH};

    Tool {
        name:         TOOL_BRP_GET_TRACE_LOG_PATH.into(),
        description:  DESC_BRP_GET_TRACE_LOG_PATH.into(),
        input_schema: schema::SchemaBuilder::new().build(),
    }
}
