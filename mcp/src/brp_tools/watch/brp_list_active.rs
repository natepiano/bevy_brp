//! List all active watches

use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::{Value, json};

use super::support::manager::WATCH_MANAGER;
use crate::BrpMcpService;
use crate::brp_tools::constants::{JSON_FIELD_COUNT, JSON_FIELD_WATCHES};
use crate::error::Result;
use crate::support::response::ResponseBuilder;
use crate::support::schema;
use crate::support::serialization::json_response_to_result;
use crate::tools::{DESC_BRP_LIST_ACTIVE_WATCHES, TOOL_BRP_LIST_ACTIVE_WATCHES};

pub fn register_tool() -> Tool {
    Tool {
        name:         TOOL_BRP_LIST_ACTIVE_WATCHES.into(),
        description:  DESC_BRP_LIST_ACTIVE_WATCHES.into(),
        input_schema: schema::SchemaBuilder::new().build(),
    }
}

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
                "timeout_reason": watch.timeout_reason,
            })
        })
        .collect();

    let response = match build_response(&watches_json) {
        Ok(resp) => resp,
        Err(err) => return Err(crate::error::report_to_mcp_error(&err)),
    };

    Ok(json_response_to_result(&response))
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
