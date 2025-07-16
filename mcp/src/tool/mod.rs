mod constants;
mod definitions;
mod handler_fn;
mod mcp_tool_schema;
mod parameters;
mod tool_def;
mod tool_handler;
mod types;

/// constants used in the wild
pub use constants::{
    BRP_EXTRAS_PREFIX, BRP_METHOD_EXTRAS_SHUTDOWN, BRP_METHOD_GET_WATCH, BRP_METHOD_INSERT,
    BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST, BRP_METHOD_LIST_WATCH,
    BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE, BRP_METHOD_REGISTRY_SCHEMA,
    BRP_METHOD_SPAWN,
};
pub use definitions::get_all_tool_definitions;
pub use handler_fn::HandlerFn;
pub use parameters::ParamType;
pub use tool_def::ToolDef;
pub use types::{
    BrpHandlerResponse, BrpToolFn, HandlerResponse, HandlerResult, LocalToolFn,
    LocalToolFnWithPort, ToolHandler,
};
