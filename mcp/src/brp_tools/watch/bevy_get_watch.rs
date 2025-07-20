//! Start watching an entity for component changes

use super::types::WatchStartResult;
use crate::tool::{
    HandlerContext, HandlerResponse, HandlerResult, HasPort, LocalToolFnWithPort, NoMethod,
    ParameterName, ToolError, ToolResult,
};

pub struct BevyGetWatch;

impl LocalToolFnWithPort for BevyGetWatch {
    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<'_> {
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
        Box::pin(async move {
            let result = handle_impl(entity_id, components, port).await;
            let tool_result = ToolResult(result);
            Ok(Box::new(tool_result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    entity_id: u64,
    components: Option<Vec<String>>,
    port: u16,
) -> Result<WatchStartResult, ToolError> {
    // Start the watch task
    let result = super::start_entity_watch_task(entity_id, components, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start entity watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult {
            watch_id,
            log_path: log_path.to_string_lossy().to_string(),
        }),
        Err(e) => Err(ToolError::new(e.to_string())),
    }
}
