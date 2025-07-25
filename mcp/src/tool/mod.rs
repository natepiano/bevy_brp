mod annotations;
mod field_placement;
mod handler_context;
mod large_response;
mod parameters;
mod response_builder;
mod tool_def;
mod tool_name;
mod types;

// Re-export types that were previously in response module
pub use field_placement::{FieldPlacement, FieldPlacementInfo, HasFieldPlacement, ResponseData};
pub use handler_context::HandlerContext;
pub use large_response::{LargeResponseConfig, handle_large_response};
pub use parameters::{JsonFieldAccess, ParameterName};
pub use response_builder::{
    CallInfo, CallInfoProvider, JsonResponse, LocalCallInfo, LocalWithPortCallInfo, ResponseBuilder,
};
pub use tool_def::ToolDef;
pub use tool_name::{BrpMethod, ToolName, get_all_tool_definitions};
pub use types::{HandlerResult, ToolFn, ToolResult};

/// Trait for types that can provide dynamic message templates
///
/// This trait is automatically implemented by the `ResultFieldPlacement` derive macro
/// for structs that have a field with `#[to_message(message_template = "...")]`.
///
/// **Important**: When this trait is implemented via the macro:
/// - All struct fields become private
/// - A `::new()` constructor is generated
/// - The struct can ONLY be constructed via `::new()` to ensure the message template is set
///
/// # Example
/// ```ignore
/// #[derive(ResultFieldPlacement)]
/// struct MyResult {
///     #[to_metadata]
///     count: usize,  // This becomes private!
///     
///     #[to_message(message_template = "Processed {count} items")]
///     message_template: String,  // This becomes private!
/// }
///
/// // Can only construct via:
/// let result = MyResult::new(42);
/// ```
pub trait MessageTemplateProvider {
    /// Get the message template for this response
    fn get_message_template(&self) -> &str;
}
