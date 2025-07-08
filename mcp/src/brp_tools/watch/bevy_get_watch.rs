//! Start watching an entity for component changes

use rmcp::Error as McpError;
use rmcp::model::{CallToolRequestParam, CallToolResult};
use serde_json::Value;

use crate::brp_tools::constants::{
    DEFAULT_BRP_PORT, JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITY, JSON_FIELD_PORT,
};
use crate::support::params;

pub async fn handle(request: CallToolRequestParam) -> Result<CallToolResult, McpError> {
    let arguments = Value::Object(request.arguments.unwrap_or_default());

    // Extract parameters
    let entity_id = params::extract_required_u64(&arguments, JSON_FIELD_ENTITY, "entity")?;
    let components = params::extract_optional_string_array(&arguments, JSON_FIELD_COMPONENTS);
    let port = params::extract_optional_u16(&arguments, JSON_FIELD_PORT, DEFAULT_BRP_PORT);

    // Start the watch task
    let result = super::start_entity_watch_task(entity_id, components, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start entity watch", Some(entity_id), e));
    Ok(super::format_watch_start_response(
        result,
        "entity watch",
        entity_id,
    ))
}
