use super::HandlerContext;
use super::HandlerResult;
use super::ParamStruct;
use super::ResultStruct;
use super::ToolDef;
use super::ToolResult;
use super::handler;
use super::registry;
use crate::error::Result;

/// Visibility facade for the tool catalog.
///
/// Callers outside `tool_name.rs` should depend on the `tool` subsystem boundary
/// rather than on `ToolName` owning whole-registry construction.
pub fn get_all_tool_definitions() -> Vec<ToolDef> { registry::get_all_tool_definitions() }

/// Visibility facade for parameter extraction used by generated and framework code.
///
/// This keeps request decoding owned by the `tool` subsystem instead of exposing
/// `HandlerContext`'s parsing method across sibling modules.
pub fn extract_parameter_values<T>(ctx: &HandlerContext) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    ctx.extract_parameter_values()
}

/// Visibility facade for custom `ToolFn::call()` implementations.
///
/// Sibling modules can request typed parameters through the top-level `tool`
/// boundary instead of depending on lower-level helper placement.
pub fn call_with_typed_params<O, P, F, Fut>(
    ctx: HandlerContext,
    f: F,
) -> HandlerResult<'static, ToolResult<O, P>>
where
    O: ResultStruct + Send + Sync + 'static,
    P: ParamStruct + Clone + for<'de> serde::Deserialize<'de> + Send + 'static,
    F: FnOnce(HandlerContext, P) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<O>> + Send + 'static,
{
    handler::call_with_typed_params(ctx, f)
}
