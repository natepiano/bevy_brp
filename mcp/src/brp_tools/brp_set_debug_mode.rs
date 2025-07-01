use std::sync::atomic::{AtomicBool, Ordering};

use rmcp::model::{CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde::Deserialize;
use serde_json;

use crate::BrpMcpService;
use crate::brp_tools::support::brp_client::execute_brp_method;
use crate::error::{Error, Result, report_to_mcp_error};
use crate::support::response::{JsonResponse, ResponseBuilder};
use crate::support::schema;
use crate::support::serialization::json_response_to_result;
use crate::tools::BRP_METHOD_EXTRAS_SET_DEBUG_MODE;

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if debug mode is currently enabled
pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

#[derive(Debug, Deserialize)]
pub struct SetDebugModeParams {
    enabled: bool,
    #[serde(default = "default_port")]
    port:    u16,
}

const fn default_port() -> u16 {
    crate::brp_tools::constants::DEFAULT_BRP_PORT
}

/// Handle the `set_debug_mode` tool request
pub async fn handle_set_debug_mode(
    _service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> std::result::Result<CallToolResult, McpError> {
    let args = request.arguments.unwrap_or_default();
    let params: SetDebugModeParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| -> McpError {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(
                    "Invalid parameters for brp_set_debug_mode".to_string(),
                ))
                .attach_printable(format!("Deserialization error: {e}"))
                .attach_printable("Expected SetDebugModeParams structure"),
            )
        })?;

    // Update the debug state locally
    DEBUG_ENABLED.store(params.enabled, Ordering::Relaxed);

    // Try to update debug mode in bevy_brp_extras if available
    let extras_params = serde_json::json!({
        "enabled": params.enabled
    });

    let extras_result = match execute_brp_method(
        BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
        Some(extras_params),
        Some(params.port),
    )
    .await
    {
        Ok(_) => Some(("success", "bevy_brp_extras debug mode updated")),
        Err(_) => {
            // It's okay if bevy_brp_extras isn't available or doesn't support debug mode
            Some(("unavailable", "bevy_brp_extras debug mode not available"))
        }
    };

    let message = if params.enabled {
        "Debug mode enabled - comprehensive BRP diagnostic information will be included in responses"
    } else {
        "Debug mode disabled - comprehensive BRP diagnostic information will be excluded from responses"
    };

    let response = match build_response(message, params.enabled, extras_result) {
        Ok(resp) => resp,
        Err(err) => return Err(crate::error::report_to_mcp_error(&err)),
    };

    Ok(json_response_to_result(&response))
}

fn build_response(
    message: &str,
    debug_enabled: bool,
    extras_result: Option<(&str, &str)>,
) -> Result<JsonResponse> {
    let mut response_builder = ResponseBuilder::success()
        .message(message)
        .add_field("debug_enabled", debug_enabled)?;

    if let Some((status, msg)) = extras_result {
        response_builder = response_builder
            .add_field("bevy_brp_extras_status", status)?
            .add_field("bevy_brp_extras_message", msg)?;
    }

    let response = response_builder
        .auto_inject_debug_info(None::<&serde_json::Value>, None::<&serde_json::Value>)
        .build();
    Ok(response)
}

/// Register the `set_debug_mode` tool
pub fn register_tool() -> Tool {
    use crate::brp_tools::constants::DEFAULT_BRP_PORT;
    use crate::tools::{DESC_BRP_SET_DEBUG_MODE, TOOL_BRP_SET_DEBUG_MODE};

    Tool {
        name:         TOOL_BRP_SET_DEBUG_MODE.into(),
        description:  DESC_BRP_SET_DEBUG_MODE.into(),
        input_schema: schema::SchemaBuilder::new()
            .add_boolean_property(
                "enabled",
                "Set to true to enable debug output, false to disable",
                true,
            )
            .add_number_property(
                "port",
                &format!("The BRP port (default: {DEFAULT_BRP_PORT})"),
                false,
            )
            .build(),
    }
}
