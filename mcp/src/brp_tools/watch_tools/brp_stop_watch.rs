//! Stop an active watch

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::error::{Error, Result};
use crate::response::LocalCallInfo;
use crate::tool::{HandlerContext, HandlerResponse, ToolFn};

#[derive(Deserialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct StopWatchParams {
    /// The watch ID returned from `bevy_start_entity_watch` or `bevy_start_list_watch`
    #[to_metadata]
    pub watch_id: u32,
}

/// Result from stopping a watch operation
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct StopWatchResult {
    /// Watch ID that was stopped
    #[to_metadata]
    pub watch_id: u32,
}

pub struct BrpStopWatch;

impl ToolFn for BrpStopWatch {
    type Output = StopWatchResult;
    type CallInfoData = LocalCallInfo;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)> {
        // Extract typed parameters
        let params: StopWatchParams = match ctx.extract_parameter_values() {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let result = handle_impl(params.watch_id).await?;
            Ok((LocalCallInfo, result))
        })
    }
}

async fn handle_impl(watch_id: u32) -> Result<StopWatchResult> {
    // Stop the watch and release lock immediately
    let result = {
        let mut manager = WATCH_MANAGER.lock().await;
        manager.stop_watch(watch_id)
    };

    // Convert result to our typed response
    match result {
        Ok(()) => Ok(StopWatchResult { watch_id }),
        Err(e) => {
            Err(Error::tool_call_failed(format!("Failed to stop watch {watch_id}: {e}")).into())
        }
    }
}
