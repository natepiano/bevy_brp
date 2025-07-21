use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyExamplesStrategy;
use crate::error::Error;
use crate::tool::{HandlerContext, HandlerResponse, LocalToolFn, NoMethod, NoPort};

/// Result from listing Bevy examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBevyExamplesResult {
    /// List of Bevy examples found
    pub examples: Vec<serde_json::Value>,
}

pub struct ListBevyExamples;

impl LocalToolFn for ListBevyExamples {
    type Output = ListBevyExamplesResult;

    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<Self::Output> {
        // Clone context to owned data for async move closure
        let owned_ctx = ctx.clone();

        Box::pin(async move {
            handle_impl(&owned_ctx)
                .await
                .map_err(|e| Error::tool_call_failed(e.message).into())
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

        Ok(ListBevyExamplesResult { examples: items })
    })
    .await
}
