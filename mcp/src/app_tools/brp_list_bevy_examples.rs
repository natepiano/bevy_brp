use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyExamplesStrategy;
use crate::tool::{HandlerContext, HandlerResponse, HandlerResult, LocalToolFn, NoMethod, NoPort};

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

impl LocalToolFn for ListBevyExamples {
    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<'_> {
        // Clone context to owned data for async move closure
        let owned_ctx = ctx.clone();

        Box::pin(async move {
            handle_impl(&owned_ctx)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl<Port, Method>(
    handler_context: &HandlerContext<Port, Method>,
) -> Result<ListBevyExamplesResult, McpError>
where
    Port: Send + Sync,
    Method: Send + Sync,
{
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyExamplesStrategy);

        Ok(ListBevyExamplesResult {
            count:    items.len(),
            examples: items,
        })
    })
    .await
}
