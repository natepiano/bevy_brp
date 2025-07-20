//! Stop an active watch

use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::tool::{
    HandlerContext, HandlerResponse, HandlerResult, LocalToolFn, NoMethod, NoPort, ParameterName,
    ToolError, ToolResult,
};

/// Result from stopping a watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopWatchResult {
    /// Watch ID that was stopped
    pub watch_id: u32,
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
        let watch_id = match ctx.extract_required(ParameterName::WatchId) {
            Ok(value) => match value.into_u32() {
                Ok(id) => id,
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let result = handle_impl(watch_id).await;
            let tool_result = ToolResult(result);
            Ok(Box::new(tool_result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(watch_id: u32) -> std::result::Result<StopWatchResult, ToolError> {
    // Stop the watch and release lock immediately
    let result = {
        let mut manager = WATCH_MANAGER.lock().await;
        manager.stop_watch(watch_id)
    };

    // Convert result to our typed response
    match result {
        Ok(()) => Ok(StopWatchResult { watch_id }),
        Err(e) => Err(ToolError::new(format!(
            "Failed to stop watch {watch_id}: {e}"
        ))),
    }
}
