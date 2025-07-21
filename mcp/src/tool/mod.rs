mod annotations;
mod constants;
mod definitions;
mod handler_context;
mod handler_fn;
mod mcp_tool_schema;
mod schema_utils;
mod tool_def;
mod types;

/// constants used in the wild
pub use constants::{
    BRP_EXTRAS_PREFIX, BRP_METHOD_EXTRAS_SHUTDOWN, BRP_METHOD_GET_WATCH, BRP_METHOD_INSERT,
    BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST, BRP_METHOD_LIST_WATCH,
    BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE, BRP_METHOD_REGISTRY_SCHEMA,
    BRP_METHOD_SPAWN,
};
pub use definitions::get_all_tool_definitions;
pub use handler_context::{HandlerContext, HasCallInfo, HasMethod, HasPort, NoMethod, NoPort};
pub use handler_fn::HandlerFn;
// Export schema utilities for parameter generation
pub use schema_utils::schema_to_parameters;
pub use tool_def::ToolDef;
pub use types::{BrpToolFn, HandlerResponse, LocalToolFn, LocalToolFnWithPort};

// Re-export from field_extraction for compatibility during migration
pub use crate::field_extraction::ParameterName;
