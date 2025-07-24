use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyAppsStrategy;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResponse, LocalCallInfo, ToolFn, WithCallInfo};

/// Result from listing Bevy apps
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
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
        ctx: &HandlerContext,
    ) -> HandlerResponse<(Self::CallInfoData, Result<Self::Output>)> {
        // Clone context to owned data for async move closure
        let owned_ctx = ctx.clone();

        Box::pin(async move { Ok(handle_impl(&owned_ctx).await.with_call_info(LocalCallInfo)) })
    }
}

async fn handle_impl(handler_context: &HandlerContext) -> Result<ListBevyAppsResult> {
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyAppsStrategy);

        Ok(ListBevyAppsResult { apps: items })
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
