//! Stop an active watch

use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::constants::PARAM_WATCH_ID;
use crate::tool::{HandlerContext, HandlerResponse, HandlerResult, LocalToolFn, NoMethod, NoPort};

/// Result from stopping a watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopWatchResult {
    /// Status of the operation
    pub status:  String,
    /// Status message
    pub message: String,
}

impl HandlerResult for StopWatchResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct BrpStopWatch;

impl LocalToolFn for BrpStopWatch {
    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<'_> {
        // Extract parameters before async block
        let watch_id = match ctx.extract_required_u32(PARAM_WATCH_ID, "watch ID") {
            Ok(id) => id,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            handle_impl(watch_id)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(watch_id: u32) -> std::result::Result<StopWatchResult, McpError> {
    // Stop the watch and release lock immediately
    let result = {
        let mut manager = WATCH_MANAGER.lock().await;
        manager.stop_watch(watch_id)
    };

    // Convert result to our typed response
    match result {
        Ok(()) => Ok(StopWatchResult {
            status:  "success".to_string(),
            message: format!("Successfully stopped watch {watch_id}"),
        }),
        Err(e) => Ok(StopWatchResult {
            status:  "error".to_string(),
            message: format!("Failed to stop watch {watch_id}: {e}"),
        }),
    }
}
