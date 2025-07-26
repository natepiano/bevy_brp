use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyAppsStrategy;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

/// Result from listing Bevy apps
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListBevyAppsResult {
    /// Count of apps found
    #[to_metadata]
    count:            usize,
    /// List of Bevy apps found
    #[to_result]
    apps:             Vec<serde_json::Value>,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {count} Bevy apps")]
    message_template: String,
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

        Ok(ListBevyAppsResult::new(items.len(), items))
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
