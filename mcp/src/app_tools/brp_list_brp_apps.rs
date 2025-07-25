use bevy_brp_mcp_macros::ResultFieldPlacement;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BrpAppsStrategy;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

/// Result from listing BRP apps
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
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

pub struct ListBrpApps;

impl ToolFn for ListBrpApps {
    type Output = ListBrpAppsResult;
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

async fn handle_impl(handler_context: HandlerContext) -> Result<ListBrpAppsResult>
where
{
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BrpAppsStrategy);

        Ok(ListBrpAppsResult::new(items.len(), items))
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
