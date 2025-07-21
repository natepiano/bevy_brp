//! Stop an active watch

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::error::Error;
use crate::tool::{HandlerContext, HandlerResponse, LocalToolFn, NoMethod, NoPort, ParameterName};

#[derive(Deserialize, JsonSchema)]
pub struct StopWatchParams {
    /// The watch ID returned from bevy_start_entity_watch or bevy_start_list_watch
    pub watch_id: u32,
}

/// Result from stopping a watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopWatchResult {
    /// Watch ID that was stopped
    pub watch_id: u32,
}

pub struct BrpStopWatch;

impl LocalToolFn for BrpStopWatch {
    type Output = StopWatchResult;

    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<Self::Output> {
        // Extract parameters before async block
        let watch_id = match ctx.extract_required(ParameterName::WatchId) {
            Ok(value) => match value.into_u32() {
                Ok(id) => id,
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move { handle_impl(watch_id).await })
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
