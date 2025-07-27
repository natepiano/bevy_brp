//! Start watching an entity for component list changes

use bevy_brp_mcp_macros::ParamStruct;
use schemars::JsonSchema;
use serde::Deserialize;

use super::types::WatchStartResult;
use crate::brp_tools::Port;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Deserialize, JsonSchema, ParamStruct)]
pub struct ListWatchParams {
    /// The entity ID to watch for component list changes
    #[to_metadata]
    pub entity: u64,
    /// The BRP port (default: 15702)
    #[serde(default)]
    #[to_call_info]
    pub port:   Port,
}

pub struct BevyListWatch;

impl ToolFn for BevyListWatch {
    type Output = WatchStartResult;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output>> {
        Box::pin(async move {
            let params: ListWatchParams = ctx.extract_parameter_values()?;

            let port = params.port;
            Ok(ToolResult::with_port(
                handle_impl(params.entity, port).await,
                port,
            ))
        })
    }
}

async fn handle_impl(entity_id: u64, port: Port) -> Result<WatchStartResult> {
    // Start the watch task
    let result = super::start_list_watch_task(entity_id, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult::new(
            watch_id,
            log_path.to_string_lossy().to_string(),
        )),
        Err(e) => Err(Error::tool_call_failed(e.to_string()).into()),
    }
}
