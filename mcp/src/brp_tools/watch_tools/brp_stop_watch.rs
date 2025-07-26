//! Stop an active watch

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

#[derive(Deserialize, JsonSchema, ParamStruct)]
pub struct StopWatchParams {
    /// The watch ID returned from `bevy_start_entity_watch` or `bevy_start_list_watch`
    #[to_metadata]
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

pub struct BrpStopWatch;

impl ToolFn for BrpStopWatch {
    type Output = StopWatchResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            // Extract typed parameters
            let params: StopWatchParams = ctx.extract_parameter_values()?;

            Ok(ToolResult::from_result(
                handle_impl(params.watch_id).await,
                LocalCallInfo,
            ))
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
        Ok(()) => Ok(StopWatchResult::new(watch_id)),
        Err(e) => {
            Err(Error::tool_call_failed(format!("Failed to stop watch {watch_id}: {e}")).into())
        }
    }
}
