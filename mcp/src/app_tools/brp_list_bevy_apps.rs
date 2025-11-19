use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use serde::Deserialize;
use serde::Serialize;

use super::support;
use super::support::BevyAppsStrategy;
use crate::error::Result;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::NoParams;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

/// Result from listing Bevy apps
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListBevyAppsResult {
    /// Count of apps found
    #[to_metadata]
    count: usize,
    /// List of Bevy apps found
    #[to_result]
    apps: Vec<serde_json::Value>,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {count} Bevy apps")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "NoParams", output = "ListBevyAppsResult", with_context)]
pub struct ListBevyApps;

#[allow(clippy::unused_async)]
async fn handle_impl(ctx: HandlerContext, _params: NoParams) -> Result<ListBevyAppsResult> {
    let search_paths = &ctx.roots;
    let items = support::collect_all_items(search_paths, &BevyAppsStrategy);
    Ok(ListBevyAppsResult::new(items.len(), items))
}
