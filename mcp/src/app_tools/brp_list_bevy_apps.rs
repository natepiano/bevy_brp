use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyAppsStrategy;
use crate::error::Error;
use crate::tool::{HandlerContext, HandlerResponse, LocalToolFn};

/// Result from listing Bevy apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBevyAppsResult {
    /// List of Bevy apps found
    pub apps: Vec<serde_json::Value>,
}

pub struct ListBevyApps;

impl LocalToolFn for ListBevyApps {
    type Output = ListBevyAppsResult;

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

async fn handle_impl(handler_context: &HandlerContext) -> Result<ListBevyAppsResult, McpError> {
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyAppsStrategy);

        Ok(ListBevyAppsResult { apps: items })
    })
    .await
}
