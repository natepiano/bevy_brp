use bevy_brp_mcp_macros::ResultFieldPlacement;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BrpAppsStrategy;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

/// Result from listing BRP apps
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
pub struct ListBrpAppsResult {
    /// List of BRP-enabled apps found
    #[to_result]
    pub apps: Vec<serde_json::Value>,
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

        Ok(ListBrpAppsResult { apps: items })
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
