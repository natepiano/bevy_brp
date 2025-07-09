//! Start watching an entity for component changes

use rmcp::Error as McpError;
use rmcp::model::CallToolRequestParam;
use serde_json::Value;

use crate::brp_tools::constants::{
    DEFAULT_BRP_PORT, JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITY, JSON_FIELD_PORT,
};
use crate::extractors::McpCallExtractor;

pub async fn handle(request: CallToolRequestParam) -> Result<Value, McpError> {
    // Extract parameters
    let extractor = McpCallExtractor::from_request(&request);
    let entity_id = extractor.get_required_u64(JSON_FIELD_ENTITY, "entity ID")?;
    let components = extractor.optional_string_array(JSON_FIELD_COMPONENTS);
    let port = extractor
        .get_optional_u16(JSON_FIELD_PORT)?
        .unwrap_or(DEFAULT_BRP_PORT);

    // Start the watch task
    let result = super::start_entity_watch_task(entity_id, components, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start entity watch", Some(entity_id), e));
    Ok(super::format_watch_start_response_value(
        result,
        "entity watch",
        entity_id,
    ))
}
