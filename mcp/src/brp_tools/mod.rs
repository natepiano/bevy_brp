mod brp_client;
mod brp_type_guide;
mod constants;
mod port;
mod tools;
mod watch_tools;

// Public exports
//
// We export `JSON_RPC_ERROR_METHOD_NOT_FOUND` so that the `brp_shutdown` tool can determine if
// `brp_mcp_extras` is available
pub use brp_client::{
    BrpClient, BrpToolConfig, FormatCorrectionStatus, JSON_RPC_ERROR_METHOD_NOT_FOUND,
    ResponseStatus, ResultStructBrpExt,
};
//
// Export brp_type_guide tools
pub use brp_type_guide::{
    AllTypeGuidesParams, BrpAllTypeGuides, BrpTypeGuide, BrpTypeName, TypeGuideParams,
};
pub use constants::{BRP_EXTRAS_PORT_ENV_VAR, MAX_VALID_PORT};
pub use port::Port;
//
// Export all parameter and result structs by name
pub use tools::bevy_get::{GetParams, GetResult};
pub use tools::bevy_insert::{InsertParams, InsertResult};
pub use tools::bevy_insert_resource::{InsertResourceParams, InsertResourceResult};
pub use tools::bevy_mutate_component::{MutateComponentParams, MutateComponentResult};
pub use tools::bevy_mutate_resource::{MutateResourceParams, MutateResourceResult};
pub use tools::bevy_registry_schema::{RegistrySchemaParams, RegistrySchemaResult};
pub use tools::bevy_remove::{RemoveParams, RemoveResult};
pub use tools::bevy_remove_resource::{RemoveResourceParams, RemoveResourceResult};
pub use tools::bevy_reparent::{ReparentParams, ReparentResult};
pub use tools::bevy_rpc_discover::{RpcDiscoverParams, RpcDiscoverResult};
pub use tools::world_despawn_entity::{DespawnEntityParams, DespawnEntityResult};
pub use tools::world_get_resources::{GetResourcesParams, GetResourcesResult};
pub use tools::world_list_components::{ListComponentsParams, ListComponentsResult};
pub use tools::world_list_resources::{ListResourcesParams, ListResourcesResult};
pub use tools::world_query::{QueryParams, QueryResult};
pub use tools::world_spawn_entity::{SpawnEntityParams, SpawnEntityResult};
//
// Export special case tools that don't follow the standard pattern
pub use tools::brp_execute::{BrpExecute, ExecuteParams};
pub use tools::brp_extras_screenshot::{ScreenshotParams, ScreenshotResult};
pub use tools::brp_extras_send_keys::{SendKeysParams, SendKeysResult};
pub use tools::brp_extras_set_window_title::{SetWindowTitleParams, SetWindowTitleResult};
//
// Export watch tools
pub use watch_tools::{BevyGetWatch, GetWatchParams};
pub use watch_tools::{
    BevyListWatch, BrpListActiveWatches, BrpStopWatch, ListWatchParams, StopWatchParams,
    WatchManager,
};
