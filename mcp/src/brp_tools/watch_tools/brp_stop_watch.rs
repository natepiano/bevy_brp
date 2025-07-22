//! Stop an active watch

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::error::Error;
use crate::tool::{HandlerContext, HandlerResponse, UnifiedToolFn};

#[derive(Deserialize, JsonSchema)]
pub struct StopWatchParams {
    /// The watch ID returned from `bevy_start_entity_watch` or `bevy_start_list_watch`
    pub watch_id: u32,
}

/// Result from stopping a watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopWatchResult {
    /// Watch ID that was stopped
    pub watch_id: u32,
}

pub struct BrpStopWatch;

impl UnifiedToolFn for BrpStopWatch {
    type Output = StopWatchResult;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<Self::Output> {
        // Extract typed parameters
        let params: StopWatchParams = match ctx.extract_typed_params() {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move { handle_impl(params.watch_id).await })
    }
}

async fn handle_impl(watch_id: u32) -> crate::error::Result<StopWatchResult> {
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
