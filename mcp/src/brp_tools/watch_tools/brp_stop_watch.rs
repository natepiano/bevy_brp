//! Stop an active watch

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::manager::WATCH_MANAGER;
use crate::error::Error;
use crate::error::Result;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct StopWatchParams {
    /// The watch ID returned from `bevy_start_entity_watch` or `bevy_start_list_watch`
    pub watch_id: u32,
}

/// Result from stopping a watch operation
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct StopWatchResult {
    /// Watch ID that was stopped
    #[to_metadata]
    watch_id: u32,

    /// Message template for formatting responses
    #[to_message(message_template = "Stopped watch {watch_id}")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "StopWatchParams", output = "StopWatchResult")]
pub struct BrpStopWatch;

async fn handle_impl(params: StopWatchParams) -> Result<StopWatchResult> {
    // Stop the watch and release lock immediately
    let result = {
        let mut manager = WATCH_MANAGER.lock().await;
        manager.stop_watch(params.watch_id)
    };

    // Convert result to our typed response
    match result {
        Ok(()) => Ok(StopWatchResult::new(params.watch_id)),
        Err(e) => Err(Error::tool_call_failed(format!(
            "Failed to stop watch {}: {e}",
            params.watch_id
        ))
        .into()),
    }
}
