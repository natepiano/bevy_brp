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
