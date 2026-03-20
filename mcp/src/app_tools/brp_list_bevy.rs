use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use serde::Deserialize;
use serde::Serialize;

use super::support;
use crate::error::Result;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::NoParams;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

/// Result from listing all Bevy targets (apps and examples)
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListBevyResult {
    /// Count of targets found
    #[to_metadata]
    count:            usize,
    /// List of all Bevy targets found (apps and examples)
    #[to_result]
    targets:          Vec<serde_json::Value>,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {count} Bevy targets")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "NoParams", output = "ListBevyResult", with_context)]
pub struct ListBevy;

#[allow(clippy::unused_async)]
async fn handle_impl(ctx: HandlerContext, _params: NoParams) -> Result<ListBevyResult> {
    let search_paths = &ctx.roots;
    let items = support::collect_all_bevy_targets(search_paths);
    Ok(ListBevyResult::new(items.len(), items))
}
