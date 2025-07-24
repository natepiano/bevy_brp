use serde::{Deserialize, Serialize};

use super::support;
use super::support::BevyExamplesStrategy;
use crate::error::{Error, Result};
use crate::response::LocalCallInfo;
use crate::tool::{HandlerContext, HandlerResponse, ToolFn, WithCallInfo};

/// Result from listing Bevy examples
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListBevyExamplesResult {
    /// List of Bevy examples found
    #[to_result]
    pub examples: Vec<serde_json::Value>,
}

pub struct ListBevyExamples;

impl ToolFn for ListBevyExamples {
    type Output = ListBevyExamplesResult;
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

async fn handle_impl(handler_context: &HandlerContext) -> Result<ListBevyExamplesResult>
where
{
    support::handle_list_binaries(handler_context, |search_paths| async move {
        let items = support::collect_all_items(&search_paths, &BevyExamplesStrategy);

        Ok(ListBevyExamplesResult { examples: items })
    })
    .await
    .map_err(|e| Error::tool_call_failed(e.message).into())
}
