use std::sync::Arc;

use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::support::collection_strategy::BrpAppsStrategy;
use super::support::generic_listing_handler;
use crate::handler::{HandlerContext, HandlerResponse, HandlerResult, LocalHandler};
use crate::service;

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
        let service = Arc::clone(&ctx.service);
        let context = ctx.context.clone();

        Box::pin(async move {
            handle_impl(service, context)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    service: Arc<crate::McpService>,
    context: rmcp::service::RequestContext<rmcp::RoleServer>,
) -> Result<ListBrpAppsResult, McpError> {
    service::handle_list_binaries_typed(service, context, |search_paths| async move {
        let items = generic_listing_handler::collect_all_items(&search_paths, &BrpAppsStrategy);

        Ok(ListBrpAppsResult {
            count: items.len(),
            apps:  items,
        })
    })
    .await
}
