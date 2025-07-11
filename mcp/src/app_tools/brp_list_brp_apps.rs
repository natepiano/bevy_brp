use std::sync::Arc;

use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::support::collection_strategy::BrpAppsStrategy;
use super::support::list_common;
use crate::service::HandlerContext;
use crate::tool::{HandlerResponse, HandlerResult, LocalHandler};

/// Result from listing BRP apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBrpAppsResult {
    /// List of BRP-enabled apps found
    pub apps:  Vec<serde_json::Value>,
    /// Total count of apps
    pub count: usize,
}

impl HandlerResult for ListBrpAppsResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct ListBrpApps;

impl LocalHandler for ListBrpApps {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        let handler_context = crate::service::HandlerContext::new(
            Arc::clone(&ctx.service),
            ctx.request.clone(),
            ctx.context.clone(),
        );

        Box::pin(async move {
            handle_impl(&handler_context)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    handler_context: &crate::service::HandlerContext,
) -> Result<ListBrpAppsResult, McpError> {
    list_common::handle_list_binaries(handler_context, |search_paths| async move {
        let items = list_common::collect_all_items(&search_paths, &BrpAppsStrategy);

        Ok(ListBrpAppsResult {
            count: items.len(),
            apps:  items,
        })
    })
    .await
}
