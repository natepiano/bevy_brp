use std::path::{Path, PathBuf};

use rmcp::model::CallToolResult;

use crate::brp_tools::constants::{JSON_FIELD_LOG_PATH, JSON_FIELD_WATCH_ID};
use crate::error::{Error, Result};
use crate::support::response::ResponseBuilder;

pub fn format_watch_start_response(
    result: std::result::Result<(u32, PathBuf), Error>,
    operation_name: &str,
    entity_id: u64,
) -> CallToolResult {
    match result {
        Ok((watch_id, log_path)) => {
            build_watch_start_success_response(operation_name, entity_id, watch_id, &log_path)
                .map_or_else(
                    |_| {
                        let fallback_response = ResponseBuilder::error()
                            .message("Failed to build watch start response")
                            .auto_inject_debug_info(None::<&serde_json::Value>)
                            .build();
                        fallback_response.to_call_tool_result()
                    },
                    |response| response.to_call_tool_result(),
                )
        }
        Err(e) => {
            let response = ResponseBuilder::error()
                .message(e.to_string())
                .auto_inject_debug_info(None::<&serde_json::Value>)
                .build();
            response.to_call_tool_result()
        }
    }
}

fn build_watch_start_success_response(
    operation_name: &str,
    entity_id: u64,
    watch_id: u32,
    log_path: &Path,
) -> Result<crate::support::response::JsonResponse> {
    let response = ResponseBuilder::success()
        .message(format!(
            "Started {operation_name} {watch_id} for entity {entity_id}"
        ))
        .add_field(JSON_FIELD_WATCH_ID, watch_id)?
        .add_field(JSON_FIELD_LOG_PATH, log_path.to_string_lossy())?
        .auto_inject_debug_info(None::<&serde_json::Value>)
        .build();
    Ok(response)
}

pub fn format_watch_stop_response(
    result: std::result::Result<(), Error>,
    watch_id: u32,
) -> CallToolResult {
    match result {
        Ok(()) => {
            let response = ResponseBuilder::success()
                .message(format!("Stopped watch {watch_id}"))
                .auto_inject_debug_info(None::<&serde_json::Value>)
                .build();
            response.to_call_tool_result()
        }
        Err(e) => {
            let response = ResponseBuilder::error()
                .message(e.to_string())
                .auto_inject_debug_info(None::<&serde_json::Value>)
                .build();
            response.to_call_tool_result()
        }
    }
}
