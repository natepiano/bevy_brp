//! Individual tool modules containing parameter and result structs for each BRP tool

mod bevy_destroy;
mod bevy_get;
mod bevy_get_resource;
mod bevy_insert;
mod bevy_insert_resource;
mod bevy_list;
mod bevy_list_resources;
mod bevy_mutate_component;
mod bevy_mutate_resource;
mod bevy_query;
mod bevy_registry_schema;
mod bevy_remove;
mod bevy_remove_resource;
mod bevy_reparent;
mod bevy_rpc_discover;
mod bevy_spawn;
mod brp_extras_discover_format;
mod brp_extras_screenshot;
mod brp_extras_send_keys;
mod brp_extras_set_debug_mode;

// Export all parameter and result structs by name
pub use bevy_destroy::{DestroyParams, DestroyResult};
pub use bevy_get::{GetParams, GetResult};
pub use bevy_get_resource::{GetResourceParams, GetResourceResult};
pub use bevy_insert::{InsertParams, InsertResult};
pub use bevy_insert_resource::{InsertResourceParams, InsertResourceResult};
pub use bevy_list::{ListParams, ListResult};
pub use bevy_list_resources::{ListResourcesParams, ListResourcesResult};
pub use bevy_mutate_component::{MutateComponentParams, MutateComponentResult};
pub use bevy_mutate_resource::{MutateResourceParams, MutateResourceResult};
pub use bevy_query::{QueryParams, QueryResult};
pub use bevy_registry_schema::{RegistrySchemaParams, RegistrySchemaResult};
pub use bevy_remove::{RemoveParams, RemoveResult};
pub use bevy_remove_resource::{RemoveResourceParams, RemoveResourceResult};
pub use bevy_reparent::{ReparentParams, ReparentResult};
pub use bevy_rpc_discover::{RpcDiscoverParams, RpcDiscoverResult};
pub use bevy_spawn::{SpawnParams, SpawnResult};
pub use brp_extras_discover_format::{DiscoverFormatParams, DiscoverFormatResult};
pub use brp_extras_screenshot::{ScreenshotParams, ScreenshotResult};
pub use brp_extras_send_keys::{SendKeysParams, SendKeysResult};
pub use brp_extras_set_debug_mode::{SetDebugModeParams, SetDebugModeResult};
