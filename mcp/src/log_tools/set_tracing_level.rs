use std::str::FromStr;

use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::support::params;
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::tracing::{TracingLevel, set_tracing_level};

/// Handle the `brp_set_tracing_level` tool request
pub fn handle(request: &rmcp::model::CallToolRequestParam) -> Result<CallToolResult, McpError> {
    // Extract the required level parameter
    let level_str = params::extract_required_string(request, "level").map_err(|e| {
        McpError::invalid_request(format!("Missing required parameter 'level': {e}"), None)
    })?;

    // Parse the tracing level
    let tracing_level = TracingLevel::from_str(level_str).map_err(|e| {
        McpError::invalid_request(
            format!("Invalid tracing level '{level_str}': {e}. Valid levels are: error, warn, info, debug, trace"),
            None,
        )
    })?;

    // Update the tracing level
    set_tracing_level(tracing_level);

    let message = format!(
        "Tracing level set to '{}' - diagnostic information will be logged to temp directory",
        tracing_level.as_str()
    );

    let response = ResponseBuilder::success()
        .message(&message)
        .add_field("tracing_level", tracing_level.as_str())
        .map_err(|e| {
            McpError::internal_error(format!("Failed to add tracing_level field: {e}"), None)
        })?
        .add_field("log_file", "bevy_brp_mcp_trace.log (in temp directory)")
        .map_err(|e| McpError::internal_error(format!("Failed to add log_file field: {e}"), None))?
        .auto_inject_debug_info(None::<&serde_json::Value>)
        .build();

    Ok(json_response_to_result(&response))
}
