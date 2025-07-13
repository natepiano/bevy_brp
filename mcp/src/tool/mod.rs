mod brp_handler;
mod constants;
mod definitions;
mod handlers;
mod local_handler;
mod parameters;
mod types;

pub use brp_handler::BrpToolHandler;
pub use constants::{
    BRP_EXTRAS_PREFIX, BRP_METHOD_EXTRAS_SHUTDOWN, BRP_METHOD_GET_WATCH, BRP_METHOD_INSERT,
    BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST, BRP_METHOD_LIST_WATCH,
    BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE, BRP_METHOD_REGISTRY_SCHEMA,
    BRP_METHOD_SPAWN,
};
pub use definitions::{McpToolDef, get_all_tool_definitions};
pub use handlers::{brp_method_tool_call, get_tool, local_tool_call};
pub use local_handler::LocalToolHandler;
pub use types::{HandlerResponse, HandlerResult, HandlerType, LocalToolFunction, ToolHandler};
