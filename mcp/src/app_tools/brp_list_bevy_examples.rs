use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyExamplesStrategy;
use crate::error::Error;
use crate::tool::{HandlerContext, HandlerResponse, UnifiedToolFn};

/// Result from listing Bevy examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBevyExamplesResult {
    /// List of Bevy examples found
    pub examples: Vec<serde_json::Value>,
}

pub struct ListBevyExamples;

impl UnifiedToolFn for ListBevyExamples {
    type Output = ListBevyExamplesResult;
    type CallInfoData = crate::response::LocalCallInfo;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)> {
        // Clone context to owned data for async move closure
        let owned_ctx = ctx.clone();

        Box::pin(async move {
            let result = handle_impl(&owned_ctx)
                .await
                .map_err(|e| Error::tool_call_failed(e.message))?;
            Ok((crate::response::LocalCallInfo, result))
        })
    }
}

async fn handle_impl(handler_context: &HandlerContext) -> Result<ListBevyExamplesResult, McpError>
where
{
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyExamplesStrategy);

        Ok(ListBevyExamplesResult { examples: items })
    })
    .await
}
