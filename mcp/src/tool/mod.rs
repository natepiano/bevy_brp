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
pub use parameters::ParameterName;
pub use response_builder::{
    CallInfo, CallInfoProvider, JsonResponse, LocalCallInfo, LocalWithPortCallInfo, ResponseBuilder,
};
pub use tool_def::ToolDef;
pub use tool_name::{BrpMethod, ToolName, get_all_tool_definitions};
pub use types::{HandlerResponse, ToolFn, WithCallInfo};

/// Message template for tool responses
#[derive(Clone, Debug)]
pub struct MessageTemplate {
    /// Template for success messages
    pub ok: &'static str,
}
