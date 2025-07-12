use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::support::collection_strategy::BevyAppsStrategy;
use super::support::list_common;
use crate::service::{HandlerContext, LocalContext};
use crate::tool::{HandlerResponse, HandlerResult, LocalHandler};

/// Result from listing Bevy apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBevyAppsResult {
    /// List of Bevy apps found
    pub apps:  Vec<serde_json::Value>,
    /// Total count of apps
    pub count: usize,
}

impl HandlerResult for ListBevyAppsResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct ListBevyApps;

impl LocalHandler for ListBevyApps {
    fn handle(&self, ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_> {
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
    handler_context: &HandlerContext<LocalContext>,
) -> Result<ListBevyAppsResult, McpError> {
    list_common::handle_list_binaries(handler_context, |search_paths| async move {
        let items = list_common::collect_all_items(&search_paths, &BevyAppsStrategy);

        Ok(ListBevyAppsResult {
            count: items.len(),
            apps:  items,
        })
    })
    .await
}
