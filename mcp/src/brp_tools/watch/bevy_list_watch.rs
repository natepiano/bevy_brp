//! Start watching an entity for component list changes

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use crate::BrpMcpService;
use crate::brp_tools::constants::{DEFAULT_BRP_PORT, JSON_FIELD_ENTITY, JSON_FIELD_PORT};
use crate::extractors::McpCallExtractor;

pub async fn handle(
    _service: &BrpMcpService,
    request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<Value, McpError> {
    // Extract parameters
    let extractor = McpCallExtractor::from_request(&request);
    let entity_id = extractor.get_required_u64(JSON_FIELD_ENTITY, "entity ID")?;
    let port = extractor
        .get_optional_u16(JSON_FIELD_PORT)?
        .unwrap_or(DEFAULT_BRP_PORT);

    // Start the watch task
    let result = super::start_list_watch_task(entity_id, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(entity_id), e));
    Ok(super::format_watch_start_response_value(
        result,
        "list watch",
        entity_id,
    ))
}
