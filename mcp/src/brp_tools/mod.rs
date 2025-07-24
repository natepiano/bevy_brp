mod brp_client;
mod brp_execute;
mod constants;
mod format_discovery;
pub mod handler;
mod http_client;
mod json_rpc_builder;
pub mod tools;
mod watch_tools;

// Public exports
pub use brp_client::{BrpError, BrpResult, build_brp_url, execute_brp_method};
// Export special case tools that don't follow the standard pattern
pub use brp_execute::{BrpExecute, ExecuteParams};
pub use constants::{
    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE, BRP_PORT_ENV_VAR, FormatCorrectionField,
    JSON_RPC_ERROR_METHOD_NOT_FOUND, VALID_PORT_RANGE, default_port,
};
pub use format_discovery::{FormatCorrection, FormatCorrectionStatus};
use json_rpc_builder::BrpJsonRpcBuilder;
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
pub use tools::brp_extras_set_debug_mode::{SetDebugModeParams, SetDebugModeResult};
//
// Export watch tools
pub use watch_tools::WatchInfo;
pub use watch_tools::bevy_get_watch::{BevyGetWatch, GetWatchParams};
pub use watch_tools::bevy_list_watch::{BevyListWatch, ListWatchParams};
pub use watch_tools::brp_list_active::BrpListActiveWatches;
pub use watch_tools::brp_stop_watch::{BrpStopWatch, StopWatchParams};
pub use watch_tools::manager::initialize_watch_manager;
