//! Start watching an entity for component changes

use bevy_brp_mcp_macros::{ParamStruct, ToolFn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::WatchStartResult;
use crate::brp_tools::Port;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct GetWatchParams {
    /// The entity ID to watch for component changes
    pub entity: u64,
    /// Required array of component types to watch. Must contain at least one component. Without
    /// this, the watch will not detect any changes.
    pub types:  Vec<String>,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:   Port,
}

#[derive(ToolFn)]
#[tool_fn(params = "GetWatchParams", output = "WatchStartResult")]
pub struct BevyGetWatch;

async fn handle_impl(params: GetWatchParams) -> Result<WatchStartResult> {
    // Start the watch task
    let result = super::start_entity_watch_task(params.entity, Some(params.types), params.port)
        .await
        .map_err(|e| {
            super::wrap_watch_error("Failed to start entity watch", Some(params.entity), e)
        });

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult::new(
            watch_id,
            log_path.to_string_lossy().to_string(),
        )),
        Err(e) => Err(Error::tool_call_failed(e.to_string()).into()),
    }
}
