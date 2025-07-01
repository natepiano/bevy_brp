use std::str::FromStr;

use rmcp::model::{CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde::Deserialize;
use serde_json;

use crate::BrpMcpService;
use crate::error::{Error, Result, report_to_mcp_error};
use crate::support::response::{JsonResponse, ResponseBuilder};
use crate::support::schema;
use crate::support::serialization::json_response_to_result;
use crate::support::tracing::{TracingLevel, get_current_tracing_level, set_tracing_level};

#[derive(Debug, Deserialize)]
pub struct SetTracingLevelParams {
    level: String,
}

/// Handle the `set_tracing_level` tool request
pub fn handle_set_tracing_level(
    _service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> std::result::Result<CallToolResult, McpError> {
    let args = request.arguments.unwrap_or_default();
    let params: SetTracingLevelParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| -> McpError {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(
                    "Invalid parameters for brp_set_tracing_level".to_string(),
                ))
                .attach_printable(format!("Deserialization error: {e}"))
                .attach_printable("Expected SetTracingLevelParams structure"),
            )
        })?;

    // Parse the tracing level
    let tracing_level = TracingLevel::from_str(&params.level).map_err(|e| -> McpError {
        report_to_mcp_error(
            &error_stack::Report::new(Error::ParameterExtraction(
                "Invalid tracing level".to_string(),
            ))
            .attach_printable(e)
            .attach_printable("Valid levels are: error, warn, info, debug, trace"),
        )
    })?;

    // Update the tracing level
    set_tracing_level(tracing_level);

    let message = format!(
        "Tracing level set to '{}' - diagnostic information will be logged to temp directory",
        tracing_level.as_str()
    );

    let response = match build_response(&message, tracing_level) {
        Ok(resp) => resp,
        Err(err) => return Err(crate::error::report_to_mcp_error(&err)),
    };

    Ok(json_response_to_result(&response))
}

fn build_response(message: &str, tracing_level: TracingLevel) -> Result<JsonResponse> {
    let response = ResponseBuilder::success()
        .message(message)
        .add_field("tracing_level", tracing_level.as_str())?
        .add_field("log_file", "bevy_brp_mcp_trace.log (in temp directory)")?
        .auto_inject_debug_info(None::<&serde_json::Value>)
        .build();
    Ok(response)
}

/// Get the current tracing level for external access
pub fn get_current_level() -> TracingLevel {
    get_current_tracing_level()
}

/// Register the `set_tracing_level` tool
pub fn register_tool() -> Tool {
    use crate::tools::{DESC_BRP_SET_TRACING_LEVEL, TOOL_BRP_SET_TRACING_LEVEL};

    Tool {
        name:         TOOL_BRP_SET_TRACING_LEVEL.into(),
        description:  DESC_BRP_SET_TRACING_LEVEL.into(),
        input_schema: schema::SchemaBuilder::new()
            .add_string_property(
                "level",
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            )
            .build(),
    }
}
