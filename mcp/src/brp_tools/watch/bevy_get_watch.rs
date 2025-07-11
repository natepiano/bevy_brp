//! Start watching an entity for component changes

use rmcp::Error as McpError;

use super::types::WatchStartResult;
use crate::constants::{
    DEFAULT_BRP_PORT, JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITY, JSON_FIELD_PORT,
};
use crate::extractors::McpCallExtractor;
use crate::tool::{HandlerContext, HandlerResponse, HandlerResult, LocalHandler};

pub struct BevyGetWatch;

impl LocalHandler for BevyGetWatch {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        let extractor = McpCallExtractor::from_request(&ctx.request);
        let entity_id = match extractor.get_required_u64(JSON_FIELD_ENTITY, "entity ID") {
            Ok(id) => id,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let components = extractor.optional_string_array(JSON_FIELD_COMPONENTS);
        let port = match extractor.get_optional_u16(JSON_FIELD_PORT) {
            Ok(p) => p.unwrap_or(DEFAULT_BRP_PORT),
            Err(e) => return Box::pin(async move { Err(e) }),
        };

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
