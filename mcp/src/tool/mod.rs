mod annotations;
mod constants;
mod definitions;
mod handler_context;
mod mcp_tool_schema;
mod parameters;
mod tool_def;
mod types;

/// constants used in the wild
pub use constants::{
    BRP_EXTRAS_PREFIX, BRP_METHOD_DESTROY, BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
    BRP_METHOD_EXTRAS_SCREENSHOT, BRP_METHOD_EXTRAS_SEND_KEYS, BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
    BRP_METHOD_EXTRAS_SHUTDOWN, BRP_METHOD_GET, BRP_METHOD_GET_RESOURCE, BRP_METHOD_GET_WATCH,
    BRP_METHOD_INSERT, BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST, BRP_METHOD_LIST_RESOURCES,
    BRP_METHOD_LIST_WATCH, BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE,
    BRP_METHOD_QUERY, BRP_METHOD_REGISTRY_SCHEMA, BRP_METHOD_REMOVE, BRP_METHOD_REMOVE_RESOURCE,
    BRP_METHOD_REPARENT, BRP_METHOD_RPC_DISCOVER, BRP_METHOD_SPAWN,
};
pub use definitions::get_all_tool_definitions;
pub use handler_context::HandlerContext;
pub use parameters::{ParameterName, deserialize_port};
pub use tool_def::ToolDef;
pub use types::{HandlerResponse, ToolFn};
