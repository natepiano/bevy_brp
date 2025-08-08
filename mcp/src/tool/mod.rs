mod annotations;
mod field_placement;
mod handler_context;
mod json_response;
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
pub use parameters::{ParamStruct, ParameterName};
//
// exported for mcp_macros
pub use response_builder::ResponseBuilder;
//
pub use tool_def::ToolDef;
//
// Macro creates and populates the `BrpMethod` enum from tools
// flagged in the `ToolName` enum as having a `brp_method`
pub use tool_name::{BrpMethod, ToolName};
pub use types::{HandlerResult, ResultStruct, ToolFn, ToolResult};
