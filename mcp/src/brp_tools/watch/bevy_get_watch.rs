//! Start watching an entity for component changes

use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use crate::BrpMcpService;
use crate::brp_tools::constants::{
    DEFAULT_BRP_PORT, JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITY, JSON_FIELD_PORT,
};
use crate::support::{params, schema};
use crate::tools::{DESC_BEVY_GET_WATCH, TOOL_BEVY_GET_WATCH};

pub fn register_tool() -> Tool {
    Tool {
        name: TOOL_BEVY_GET_WATCH.into(),
        description: DESC_BEVY_GET_WATCH.into(),
        input_schema: schema::SchemaBuilder::new()
            .add_number_property(JSON_FIELD_ENTITY, "The entity ID to watch for component changes", true)
            .add_any_property(
                JSON_FIELD_COMPONENTS,
                "Required array of component types to watch. Must contain at least one component. Without this, the watch will not detect any changes.",
                true
            )
            .add_number_property(JSON_FIELD_PORT, &format!("The BRP port (default: {DEFAULT_BRP_PORT})"), false)
            .build()
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
    let components = params::extract_optional_string_array(&arguments, JSON_FIELD_COMPONENTS);
    let port = params::extract_optional_u16(&arguments, JSON_FIELD_PORT, DEFAULT_BRP_PORT);

    // Start the watch task
    let result = super::support::start_entity_watch_task(entity_id, components, port)
        .await
        .map_err(|e| {
            crate::error::Error::WatchOperation(format!(
                "Failed to start entity watch for entity {entity_id}: {e}"
            ))
        });
    Ok(super::support::format_watch_start_response(
        result,
        "entity watch",
        entity_id,
    ))
}
