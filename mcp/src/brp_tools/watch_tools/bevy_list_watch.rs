//! Start watching an entity for component list changes

use schemars::JsonSchema;
use serde::Deserialize;

use super::types::WatchStartResult;
use crate::brp_tools::{default_port, deserialize_port};
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResponse, LocalWithPortCallInfo, ToolFn, WithCallInfo};

#[derive(Deserialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListWatchParams {
    /// The entity ID to watch for component list changes
    #[to_metadata]
    pub entity: u64,
    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port:   u16,
}

pub struct BevyListWatch;

impl ToolFn for BevyListWatch {
    type Output = WatchStartResult;
    type CallInfoData = LocalWithPortCallInfo;

    fn call(
        &self,
        ctx: &HandlerContext,
    ) -> HandlerResponse<(Self::CallInfoData, crate::error::Result<Self::Output>)> {
        // Extract typed parameters
        let params: ListWatchParams = match ctx.extract_parameter_values() {
            Ok(params) => params,
            Err(e) => {
                return Box::pin(async move {
                    Ok(Err(e).with_call_info(LocalWithPortCallInfo { port: 15702 }))
                });
            }
        };

        let port = params.port;
        Box::pin(async move {
            Ok(handle_impl(params.entity, port)
                .await
                .with_call_info(LocalWithPortCallInfo { port }))
        })
    }
}

async fn handle_impl(entity_id: u64, port: u16) -> Result<WatchStartResult> {
    // Start the watch task
    let result = super::start_list_watch_task(entity_id, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult {
            watch_id,
            log_path: log_path.to_string_lossy().to_string(),
        }),
        Err(e) => Err(Error::tool_call_failed(e.to_string()).into()),
    }
}
