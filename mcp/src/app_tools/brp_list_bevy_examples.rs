use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::support::collection_strategy::BevyExamplesStrategy;
use super::support::list_common;
use crate::service::HandlerContext;
use crate::tool::{HandlerResponse, HandlerResult, LocalHandler};

/// Result from listing Bevy examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBevyExamplesResult {
    /// List of Bevy examples found
    pub examples: Vec<serde_json::Value>,
    /// Total count of examples
    pub count:    usize,
}

impl HandlerResult for ListBevyExamplesResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct ListBevyExamples;

impl LocalHandler for ListBevyExamples {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        // Clone context to owned data for async move closure
        let owned_ctx = ctx.clone();

        Box::pin(async move {
            handle_impl(&owned_ctx)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    handler_context: &crate::service::HandlerContext,
) -> Result<ListBevyExamplesResult, McpError> {
    list_common::handle_list_binaries(handler_context, |search_paths| async move {
        let items = list_common::collect_all_items(&search_paths, &BevyExamplesStrategy);

        Ok(ListBevyExamplesResult {
            count:    items.len(),
            examples: items,
        })
    })
    .await
}
