//! Stop an active watch

use rmcp::model::{CallToolRequestParam, CallToolResult};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use super::manager::WATCH_MANAGER;
use crate::BrpMcpService;
use crate::brp_tools::constants::JSON_FIELD_WATCH_ID;
use crate::support::params;

pub async fn handle(
    _service: &BrpMcpService,
    request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let arguments = Value::Object(request.arguments.unwrap_or_default());

    // Extract watch ID
    let watch_id = params::extract_required_u32(&arguments, JSON_FIELD_WATCH_ID, "watch_id")?;

    // Stop the watch and release lock immediately
    let result = {
        let mut manager = WATCH_MANAGER.lock().await;
        manager.stop_watch(watch_id).map_err(|e| {
            crate::error::Error::WatchOperation(format!("Failed to stop watch {watch_id}: {e}"))
        })
    };
    Ok(super::format_watch_stop_response(result, watch_id))
}
