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
pub use brp_client::BrpClient;
pub use brp_client::BrpToolConfig;
pub use brp_client::FormatCorrectionStatus;
pub use brp_client::JSON_RPC_ERROR_METHOD_NOT_FOUND;
pub use brp_client::ResponseStatus;
pub use brp_client::ResultStructBrpExt;
//
// Export brp_type_guide tools
pub use brp_type_guide::{
    AllTypeGuidesParams, BrpAllTypeGuides, BrpTypeGuide, BrpTypeName, TypeGuideParams,
};
pub use constants::BRP_EXTRAS_PORT_ENV_VAR;
pub use constants::MAX_VALID_PORT;
pub use port::Port;
//
// Export special case tools that don't follow the standard pattern
pub use tools::brp_execute::{BrpExecute, ExecuteParams};
pub use tools::brp_extras_screenshot::ScreenshotParams;
pub use tools::brp_extras_screenshot::ScreenshotResult;
pub use tools::brp_extras_send_keys::SendKeysParams;
pub use tools::brp_extras_send_keys::SendKeysResult;
pub use tools::brp_extras_set_window_title::SetWindowTitleParams;
pub use tools::brp_extras_set_window_title::SetWindowTitleResult;
//
// Export all parameter and result structs by name
pub use tools::registry_schema::{RegistrySchemaParams, RegistrySchemaResult};
pub use tools::rpc_discover::RpcDiscoverParams;
pub use tools::rpc_discover::RpcDiscoverResult;
pub use tools::world_despawn_entity::DespawnEntityParams;
pub use tools::world_despawn_entity::DespawnEntityResult;
pub use tools::world_get_components::GetComponentsParams;
pub use tools::world_get_components::GetComponentsResult;
pub use tools::world_get_resources::GetResourcesParams;
pub use tools::world_get_resources::GetResourcesResult;
pub use tools::world_insert_components::InsertComponentsParams;
pub use tools::world_insert_components::InsertComponentsResult;
pub use tools::world_insert_resources::InsertResourcesParams;
pub use tools::world_insert_resources::InsertResourcesResult;
pub use tools::world_list_components::ListComponentsParams;
pub use tools::world_list_components::ListComponentsResult;
pub use tools::world_list_resources::ListResourcesParams;
pub use tools::world_list_resources::ListResourcesResult;
pub use tools::world_mutate_components::MutateComponentsParams;
pub use tools::world_mutate_components::MutateComponentsResult;
pub use tools::world_mutate_resources::MutateResourcesParams;
pub use tools::world_mutate_resources::MutateResourcesResult;
pub use tools::world_query::QueryParams;
pub use tools::world_query::QueryResult;
pub use tools::world_remove_components::RemoveComponentsParams;
pub use tools::world_remove_components::RemoveComponentsResult;
pub use tools::world_remove_resources::RemoveResourcesParams;
pub use tools::world_remove_resources::RemoveResourcesResult;
pub use tools::world_reparent_entities::ReparentEntitiesParams;
pub use tools::world_reparent_entities::ReparentEntitiesResult;
pub use tools::world_spawn_entity::SpawnEntityParams;
pub use tools::world_spawn_entity::SpawnEntityResult;
pub use watch_tools::GetComponentsWatchParams;
pub use watch_tools::WorldGetComponentsWatch;
//
// Export watch tools
pub use watch_tools::{
    BevyListWatch, BrpListActiveWatches, BrpStopWatch, ListComponentsWatchParams, StopWatchParams,
    WatchManager,
};
