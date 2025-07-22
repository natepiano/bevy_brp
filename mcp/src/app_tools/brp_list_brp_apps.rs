use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BrpAppsStrategy;
use crate::error::Error;
use crate::tool::{HandlerContext, HandlerResponse, LocalToolFn};

/// Result from listing BRP apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBrpAppsResult {
    /// List of BRP-enabled apps found
    pub apps: Vec<serde_json::Value>,
}

pub struct ListBrpApps;

impl LocalToolFn for ListBrpApps {
    type Output = ListBrpAppsResult;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<Self::Output> {
        // Clone context to owned data for async move closure
        let owned_ctx = ctx.clone();

        Box::pin(async move {
            handle_impl(&owned_ctx)
                .await
                .map_err(|e| Error::tool_call_failed(e.message).into())
        })
    }
}

async fn handle_impl(handler_context: &HandlerContext) -> Result<ListBrpAppsResult, McpError>
where
{
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BrpAppsStrategy);

        Ok(ListBrpAppsResult { apps: items })
    })
    .await
}
