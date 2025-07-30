mod brp_client;
mod brp_execute;
mod constants;
mod format_correction_fields;
mod format_discovery;
mod handler;
mod json_rpc_builder;
mod port;
mod tools;
mod types;
mod watch_tools;

// Public exports
pub use brp_client::{BrpClient, BrpClientError, BrpClientResult};
// Export special case tools that don't follow the standard pattern
pub use brp_execute::{BrpExecute, ExecuteParams};
pub use constants::{
    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE, BRP_PORT_ENV_VAR, JSON_RPC_ERROR_METHOD_NOT_FOUND,
};
pub use format_discovery::FormatCorrectionStatus;
// Re-export handler items needed by macros
pub use handler::execute_static_brp_call;
use json_rpc_builder::BrpJsonRpcBuilder;
pub use port::Port;
//
// Export all parameter and result structs by name
pub use tools::bevy_destroy::{DestroyParams, DestroyResult};
pub use tools::bevy_get::{GetParams, GetResult};
pub use tools::bevy_get_resource::{GetResourceParams, GetResourceResult};
pub use tools::bevy_insert::{InsertParams, InsertResult};
pub use tools::bevy_insert_resource::{InsertResourceParams, InsertResourceResult};
pub use tools::bevy_list::{ListParams, ListResult};
pub use tools::bevy_list_resources::{ListResourcesParams, ListResourcesResult};
pub use tools::bevy_mutate_component::{MutateComponentParams, MutateComponentResult};
pub use tools::bevy_mutate_resource::{MutateResourceParams, MutateResourceResult};
pub use tools::bevy_query::{QueryParams, QueryResult};
pub use tools::bevy_registry_schema::{RegistrySchemaParams, RegistrySchemaResult};
pub use tools::bevy_remove::{RemoveParams, RemoveResult};
pub use tools::bevy_remove_resource::{RemoveResourceParams, RemoveResourceResult};
pub use tools::bevy_reparent::{ReparentParams, ReparentResult};
pub use tools::bevy_rpc_discover::{RpcDiscoverParams, RpcDiscoverResult};
pub use tools::bevy_spawn::{SpawnParams, SpawnResult};
pub use tools::brp_extras_discover_format::{DiscoverFormatParams, DiscoverFormatResult};
pub use tools::brp_extras_screenshot::{ScreenshotParams, ScreenshotResult};
pub use tools::brp_extras_send_keys::{SendKeysParams, SendKeysResult};
pub use types::{ExecuteMode, ResultStructBrpExt};
//
// Export watch tools
pub use watch_tools::{BevyGetWatch, GetWatchParams};
pub use watch_tools::{
    BevyListWatch, BrpListActiveWatches, BrpStopWatch, ListWatchParams, StopWatchParams,
    WatchManager,
};
