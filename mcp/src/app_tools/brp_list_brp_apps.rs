use bevy_brp_mcp_macros::{ResultStruct, ToolFn};
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BrpAppsStrategy;
use crate::error::Result;
use crate::tool::{HandlerContext, HandlerResult, NoParams, ToolFn, ToolResult};

/// Result from listing BRP apps
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListBrpAppsResult {
    /// Count of apps found
    #[to_metadata]
    count:            usize,
    /// List of BRP-enabled apps found
    #[to_result]
    apps:             Vec<serde_json::Value>,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {count} BRP-enabled apps")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "NoParams", output = "ListBrpAppsResult", with_context)]
pub struct ListBrpApps;

#[allow(clippy::unused_async)]
async fn handle_impl(ctx: HandlerContext, _params: NoParams) -> Result<ListBrpAppsResult> {
    let search_paths = &ctx.roots;
    let items = support::collect_all_items(search_paths, &BrpAppsStrategy);
    Ok(ListBrpAppsResult::new(items.len(), items))
}
