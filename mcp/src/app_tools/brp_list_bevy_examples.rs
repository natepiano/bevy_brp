use bevy_brp_mcp_macros::{ResultStruct, ToolFn};
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyExamplesStrategy;
use crate::error::Result;
use crate::tool::{HandlerContext, HandlerResult, NoParams, ToolFn, ToolResult};

/// Result from listing Bevy examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListBevyExamplesResult {
    /// Count of examples found
    #[to_metadata]
    count:            usize,
    /// List of Bevy examples found
    #[to_result]
    examples:         Vec<serde_json::Value>,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {count} Bevy examples")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "NoParams", output = "ListBevyExamplesResult", with_context)]
pub struct ListBevyExamples;

async fn handle_impl(ctx: HandlerContext, _params: NoParams) -> Result<ListBevyExamplesResult> {
    let search_paths = &ctx.roots;
    let items = support::collect_all_items(search_paths, &BevyExamplesStrategy);
    Ok(ListBevyExamplesResult::new(items.len(), items))
}
