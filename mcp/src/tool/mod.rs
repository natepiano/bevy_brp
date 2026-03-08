mod annotations;
mod field_placement;
mod handler_context;
mod json_response;
mod large_response;
mod parameters;
mod registry;
mod response_builder;
mod tool_def;
mod tool_name;
mod types;

// exported for mcp_macros
pub use field_placement::FieldPlacement;
pub use field_placement::FieldPlacementInfo;
pub use field_placement::HasFieldPlacement;
//
pub use handler_context::HandlerContext;
pub use parameters::NoParams;
pub use parameters::ParamStruct;
pub use parameters::ParameterName;
//
// exported for mcp_macros
pub use response_builder::ResponseBuilder;
//
pub use tool_def::ToolDef;
//
// Macro creates and populates the `BrpMethod` enum from tools
// flagged in the `ToolName` enum as having a `brp_method`
pub use tool_name::{BrpMethod, ToolName};
pub use types::HandlerResult;
pub use types::ResultStruct;
pub use types::ToolFn;
pub use types::ToolResult;

/// Visibility facade for the tool catalog.
///
/// Callers outside `tool_name.rs` should depend on the `tool` subsystem boundary
/// rather than on `ToolName` owning whole-registry construction.
pub fn get_all_tool_definitions() -> Vec<ToolDef> { registry::get_all_tool_definitions() }

/// Visibility facade for parameter extraction used by generated and framework code.
///
/// This keeps request decoding owned by the `tool` subsystem instead of exposing
/// `HandlerContext`'s parsing method across sibling modules.
pub fn extract_parameter_values<T>(ctx: &HandlerContext) -> crate::error::Result<T>
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
    Fut: std::future::Future<Output = crate::error::Result<O>> + Send + 'static,
{
    types::call_with_typed_params(ctx, f)
}
