//! Start watching an entity for component changes

use rmcp::Error as McpError;

use super::types::WatchStartResult;
use crate::constants::{PARAM_COMPONENTS, PARAM_ENTITY};
use crate::tool::{
    HandlerContext, HandlerResponse, HandlerResult, HasPort, LocalToolFnWithPort, NoMethod,
};

pub struct BevyGetWatch;

impl LocalToolFnWithPort for BevyGetWatch {
    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<'_> {
        let entity_id = match ctx.extract_required_u64(PARAM_ENTITY, "entity ID") {
            Ok(id) => id,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let components = ctx.extract_optional_string_array(PARAM_COMPONENTS);

        let port = ctx.port();
        Box::pin(async move {
            handle_impl(entity_id, components, port)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    entity_id: u64,
    components: Option<Vec<String>>,
    port: u16,
) -> Result<WatchStartResult, McpError> {
    // Start the watch task
    let result = super::start_entity_watch_task(entity_id, components, port)
        .await
        .map_err(|e| super::wrap_watch_error("Failed to start entity watch", Some(entity_id), e));

    match result {
        Ok((watch_id, log_path)) => {
            let message = format!("Started entity watch {watch_id} for entity {entity_id}");
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
