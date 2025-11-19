//! Start watching an entity for component list changes

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ToolFn;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::types::WatchStartResult;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListComponentsWatchParams {
    /// The entity ID to watch for component list changes
    pub entity: u64,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:   Port,
}

#[derive(ToolFn)]
#[tool_fn(params = "ListComponentsWatchParams", output = "WatchStartResult")]
pub struct BevyListWatch;

async fn handle_impl(params: ListComponentsWatchParams) -> Result<WatchStartResult> {
    // Start the watch task
    let result = super::start_list_watch_task(params.entity, params.port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(params.entity), e));

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult::new(
            watch_id,
            log_path.to_string_lossy().to_string(),
        )),
        Err(e) => Err(Error::tool_call_failed(e.to_string()).into()),
    }
}
