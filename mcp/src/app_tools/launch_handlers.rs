use async_trait::async_trait;

use super::constants::DEFAULT_PROFILE;
use super::launch_params::LaunchBevyBinaryParams;
use super::support;
use super::support::LaunchResult;
use crate::error::Result;
use crate::tool;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

/// Handler for launching Bevy targets (apps or examples) using unified search
pub struct LaunchBevyTarget;

#[async_trait]
impl ToolFn for LaunchBevyTarget {
    type Output = LaunchResult;
    type Params = LaunchBevyBinaryParams;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<'_, ToolResult<Self::Output, Self::Params>> {
        tool::call_with_typed_params(ctx, |ctx, params: LaunchBevyBinaryParams| async move {
            support::launch_bevy_target(params, ctx.roots, DEFAULT_PROFILE)
        })
    }

    async fn handle_impl(&self, _params: Self::Params) -> Result<Self::Output> {
        Err(crate::error::Error::InvalidState(
            "LaunchBevyTarget uses its custom call implementation".to_string(),
        )
        .into())
    }
}

/// Create a `LaunchBevyTarget` handler instance
pub const fn create_launch_handler() -> LaunchBevyTarget { LaunchBevyTarget }
