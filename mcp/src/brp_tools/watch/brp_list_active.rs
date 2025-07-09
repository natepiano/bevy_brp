//! List all active watches

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::{Value, json};

use super::manager::WATCH_MANAGER;
use crate::BrpMcpService;
use crate::brp_tools::constants::{JSON_FIELD_COUNT, JSON_FIELD_WATCHES};

pub async fn handle(
    _service: &BrpMcpService,
    _request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> std::result::Result<Value, McpError> {
    // Get active watches from manager and release lock immediately
    let active_watches = {
        let manager = WATCH_MANAGER.lock().await;
        manager.list_active_watches()
    };

    // Convert to JSON format
    let watches_json: Vec<Value> = active_watches
        .iter()
        .map(|watch| {
            json!({
                "watch_id": watch.watch_id,
                "entity_id": watch.entity_id,
                "watch_type": watch.watch_type,
                "log_path": watch.log_path.to_string_lossy(),
                "port": watch.port,
            })
        })
        .collect();

    let count = watches_json.len();
    Ok(json!({
        "status": "success",
        "message": format!("Found {} active watches", count),
        JSON_FIELD_WATCHES: watches_json,
        JSON_FIELD_COUNT: count
    }))
}
