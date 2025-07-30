mod annotations;
mod field_placement;
mod handler_context;
mod large_response;
mod parameters;
mod response_builder;
mod tool_def;
mod tool_name;
mod types;

// exported for mcp_macros
pub use field_placement::{FieldPlacement, FieldPlacementInfo, HasFieldPlacement};
//
pub use handler_context::HandlerContext;
pub use large_response::{LargeResponseConfig, handle_large_response};
pub use parameters::{JsonFieldAccess, ParamStruct, ParameterName};
//
// exported for mcp_macros
pub use response_builder::ResponseBuilder;
//
pub use tool_def::ToolDef;
pub use tool_name::{BrpMethod, ToolName, get_all_tool_definitions};
pub use types::{HandlerResult, ResultStruct, ToolFn, ToolResult};
