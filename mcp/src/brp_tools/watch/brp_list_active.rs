//! List all active watches

use rmcp::model::{CallToolRequestParam, CallToolResult};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::{Value, json};

use super::manager::WATCH_MANAGER;
use crate::BrpMcpService;
use crate::brp_tools::constants::{JSON_FIELD_COUNT, JSON_FIELD_WATCHES};
use crate::error::Result;
use crate::support::response::ResponseBuilder;

pub async fn handle(
    _service: &BrpMcpService,
    _request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> std::result::Result<CallToolResult, McpError> {
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

    let response = match build_response(&watches_json) {
        Ok(resp) => resp,
        Err(err) => return Err(crate::error::report_to_mcp_error(&err)),
    };

    Ok(response.to_call_tool_result())
}

fn build_response(watches_json: &[Value]) -> Result<crate::support::response::JsonResponse> {
    let response = ResponseBuilder::success()
        .message(format!("Found {} active watches", watches_json.len()))
        .add_field(JSON_FIELD_WATCHES, watches_json)?
        .add_field(JSON_FIELD_COUNT, watches_json.len())?
        .auto_inject_debug_info(None::<&serde_json::Value>)
        .build();

    Ok(response)
}
