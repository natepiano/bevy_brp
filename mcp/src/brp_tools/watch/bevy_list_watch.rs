//! Start watching an entity for component list changes

use rmcp::Error as McpError;

use super::types::WatchStartResult;
use crate::constants::JSON_FIELD_ENTITY;
use crate::tool::{
    HandlerContext, HandlerResponse, HandlerResult, HasPort, LocalToolFnWithPort, NoMethod,
};

pub struct BevyListWatch;

impl LocalToolFnWithPort for BevyListWatch {
    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<'_> {
        let entity_id = match ctx.extract_required_u64(JSON_FIELD_ENTITY, "entity ID") {
            Ok(id) => id,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let port = ctx.port();
        Box::pin(async move {
            handle_impl(entity_id, port)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(entity_id: u64, port: u16) -> Result<WatchStartResult, McpError> {
    // Start the watch task
    let result = super::start_list_watch_task(entity_id, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start list watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => {
            let message = format!("Started list watch {watch_id} for entity {entity_id}");
            Ok(WatchStartResult {
                status: "success".to_string(),
                message,
                watch_id: Some(watch_id),
                log_path: Some(log_path.to_string_lossy().to_string()),
            })
        }
        Err(e) => Ok(WatchStartResult {
            status:   "error".to_string(),
            message:  e.to_string(),
            watch_id: None,
            log_path: None,
        }),
    }
}
