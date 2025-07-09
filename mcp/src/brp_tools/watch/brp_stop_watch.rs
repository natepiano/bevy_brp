//! Stop an active watch

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use super::manager::WATCH_MANAGER;
use crate::BrpMcpService;
use crate::brp_tools::constants::JSON_FIELD_WATCH_ID;
use crate::extractors::McpCallExtractor;

pub async fn handle(
    _service: &BrpMcpService,
    request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<Value, McpError> {
    // Extract watch ID
    let extractor = McpCallExtractor::from_request(&request);
    let watch_id = extractor.get_required_u32(JSON_FIELD_WATCH_ID, "watch ID")?;

    // Stop the watch and release lock immediately
    let result = {
        let mut manager = WATCH_MANAGER.lock().await;
        manager.stop_watch(watch_id).map_err(|e| {
            super::wrap_watch_error("Failed to stop watch", None, format!("{watch_id}: {e}"))
        })
    };
    Ok(super::format_watch_stop_response_value(result, watch_id))
}
