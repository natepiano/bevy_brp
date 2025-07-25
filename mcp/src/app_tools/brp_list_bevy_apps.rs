use bevy_brp_mcp_macros::ResultFieldPlacement;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyAppsStrategy;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

/// Result from listing Bevy apps
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
pub struct ListBevyAppsResult {
    /// List of Bevy apps found
    #[to_result]
    pub apps: Vec<serde_json::Value>,
}

pub struct ListBevyApps;

impl ToolFn for ListBevyApps {
    type Output = ListBevyAppsResult;
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

async fn handle_impl(handler_context: HandlerContext) -> Result<ListBevyAppsResult> {
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyAppsStrategy);

        Ok(ListBevyAppsResult { apps: items })
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
