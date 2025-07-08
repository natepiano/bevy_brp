//! Start watching an entity for component list changes

use rmcp::model::{CallToolRequestParam, CallToolResult};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use crate::BrpMcpService;
use crate::brp_tools::constants::{DEFAULT_BRP_PORT, JSON_FIELD_ENTITY, JSON_FIELD_PORT};
use crate::support::params;

pub async fn handle(
    _service: &BrpMcpService,
    request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let arguments = Value::Object(request.arguments.unwrap_or_default());

    // Extract parameters
    let entity_id = params::extract_required_u64(&arguments, JSON_FIELD_ENTITY, "entity")?;
    let port = params::extract_optional_u16(&arguments, JSON_FIELD_PORT, DEFAULT_BRP_PORT);

    // Start the watch task
    let result = super::start_list_watch_task(entity_id, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(entity_id), e));
    Ok(super::format_watch_start_response(
        result,
        "list watch",
        entity_id,
    ))
}
