//! Start watching an entity for component list changes

use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use crate::BrpMcpService;
use crate::brp_tools::constants::{
    DEFAULT_BRP_PORT, JSON_FIELD_ENTITY, JSON_FIELD_PORT, JSON_FIELD_TIMEOUT_SECONDS,
};
use crate::support::{params, schema};
use crate::tools::{DESC_BEVY_LIST_WATCH, TOOL_BEVY_LIST_WATCH};

pub fn register_tool() -> Tool {
    Tool {
        name:         TOOL_BEVY_LIST_WATCH.into(),
        description:  DESC_BEVY_LIST_WATCH.into(),
        input_schema: schema::SchemaBuilder::new()
            .add_number_property(
                JSON_FIELD_ENTITY,
                "The entity ID to watch for component list changes",
                true,
            )
            .add_number_property(
                JSON_FIELD_PORT,
                &format!("The BRP port (default: {DEFAULT_BRP_PORT})"),
                false,
            )
            .add_number_property(
                JSON_FIELD_TIMEOUT_SECONDS,
                "Optional timeout in seconds for the watch connection (default: 30 seconds, 0 = never timeout)",
                false,
            )
            .build(),
    }
}

pub async fn handle(
    _service: &BrpMcpService,
    request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let arguments = Value::Object(request.arguments.unwrap_or_default());

    // Extract parameters
    let entity_id = params::extract_required_u64(&arguments, JSON_FIELD_ENTITY, "entity")?;
    let port = params::extract_optional_u16(&arguments, JSON_FIELD_PORT, DEFAULT_BRP_PORT);
    let timeout_seconds = arguments[JSON_FIELD_TIMEOUT_SECONDS]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok());

    // Start the watch task
    let result = super::support::start_list_watch_task(entity_id, port, timeout_seconds)
        .await
        .map_err(|e| {
            crate::error::Error::WatchOperation(format!(
                "Failed to start list watch for entity {entity_id}: {e}"
            ))
        });
    Ok(super::support::format_watch_start_response(
        result,
        "list watch",
        entity_id,
    ))
}
