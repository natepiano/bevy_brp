//! Start watching an entity for component list changes

use schemars::JsonSchema;
use serde::Deserialize;

use super::types::WatchStartResult;
use crate::brp_tools::{default_port, deserialize_port};
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalWithPortCallInfo, ToolFn, ToolResult};

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
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            let params: ListWatchParams = ctx.extract_parameter_values()?;

            let port = params.port;
            Ok(ToolResult::from_result(
                handle_impl(params.entity, port).await,
                LocalWithPortCallInfo { port },
            ))
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
