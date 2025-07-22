//! Start watching an entity for component changes

use schemars::JsonSchema;
use serde::Deserialize;

use super::types::WatchStartResult;
use crate::constants::default_port;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResponse, ToolFn};

#[derive(Deserialize, JsonSchema)]
pub struct GetWatchParams {
    /// The entity ID to watch for component changes
    pub entity: u64,
    /// Required array of component types to watch. Must contain at least one component. Without
    /// this, the watch will not detect any changes.
    pub types:  Vec<String>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:   u16,
}

pub struct BevyGetWatch;

impl ToolFn for BevyGetWatch {
    type Output = WatchStartResult;
    type CallInfoData = crate::response::LocalWithPortCallInfo;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)> {
        // Extract typed parameters
        let params: GetWatchParams = match ctx.extract_typed_params() {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let port = params.port;
        Box::pin(async move {
            let result = handle_impl(params.entity, Some(params.types), port).await?;
            Ok((crate::response::LocalWithPortCallInfo { port }, result))
        })
    }
}

async fn handle_impl(
    entity_id: u64,
    components: Option<Vec<String>>,
    port: u16,
) -> Result<WatchStartResult> {
    // Start the watch task
    let result = super::start_entity_watch_task(entity_id, components, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start entity watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult {
            watch_id,
            log_path: log_path.to_string_lossy().to_string(),
        }),
        Err(e) => Err(Error::tool_call_failed(e.to_string()).into()),
    }
}
