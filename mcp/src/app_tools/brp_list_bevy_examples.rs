use bevy_brp_mcp_macros::ResultFieldPlacement;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyExamplesStrategy;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

/// Result from listing Bevy examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
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

pub struct ListBevyExamples;

impl ToolFn for ListBevyExamples {
    type Output = ListBevyExamplesResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            let result = handle_impl(ctx).await;
            Ok(ToolResult::from_result(result, LocalCallInfo))
        })
    }
}

async fn handle_impl(handler_context: HandlerContext) -> Result<ListBevyExamplesResult>
where
{
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyExamplesStrategy);

        Ok(ListBevyExamplesResult::new(items.len(), items))
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
