//! Start watching an entity for component list changes

use super::types::WatchStartResult;
use crate::tool::{
    HandlerContext, HandlerResponse, HasPort, LocalToolFnWithPort, NoMethod, ParameterName,
    ToolError, ToolResult,
};

pub struct BevyListWatch;

impl LocalToolFnWithPort for BevyListWatch {
    type Output = WatchStartResult;

    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<Self::Output> {
        let entity_id = match ctx.extract_required(ParameterName::Entity) {
            Ok(value) => match value.into_u64() {
                Ok(id) => id,
                Err(e) => return Box::pin(async move { Err(e) }),
            },
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let port = ctx.port();
        Box::pin(async move {
            let result = handle_impl(entity_id, port).await;
            let tool_result = ToolResult { result };
            Ok(tool_result)
        })
    }
}

async fn handle_impl(entity_id: u64, port: u16) -> Result<WatchStartResult, ToolError> {
    // Start the watch task
    let result = super::start_list_watch_task(entity_id, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => Ok(WatchStartResult {
            watch_id,
            log_path: log_path.to_string_lossy().to_string(),
        }),
        Err(e) => Err(ToolError::new(e.to_string())),
    }
}
