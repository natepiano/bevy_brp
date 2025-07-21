//! Start watching an entity for component changes

use super::types::WatchStartResult;
use crate::error::{Error, Result};
use crate::tool::{
    HandlerContext, HandlerResponse, HasPort, LocalToolFnWithPort, NoMethod, ParameterName,
};

pub struct BevyGetWatch;

impl LocalToolFnWithPort for BevyGetWatch {
    type Output = WatchStartResult;

    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<Self::Output> {
        let entity_id = match ctx.extract_required(ParameterName::Entity) {
            Ok(value) => match value.into_u64() {
                Ok(id) => id,
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let components = match ctx.extract_required(ParameterName::Types) {
            Ok(value) => match value.into_string_array() {
                Ok(arr) => Some(arr),
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let port = ctx.port();
        Box::pin(async move { handle_impl(entity_id, components, port).await })
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
