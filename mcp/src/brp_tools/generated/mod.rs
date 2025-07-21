/// Generated BRP tools module
///
/// This module contains:
/// - Strongly-typed parameter structs (`params.rs`)
/// - Macro for generating tool implementations (`macro_def.rs`)
/// - Tool implementations generated via macro (`tools.rs`)
#[macro_use]
mod macro_def;
mod params;
mod tools;

// Re-export all parameter structs
pub use params::{
    DestroyParams, DiscoverFormatParams, GetParams, GetResourceParams, InsertParams,
    InsertResourceParams, ListParams, ListResourcesParams, MutateComponentParams,
    MutateResourceParams, QueryParams, RegistrySchemaParams, RemoveParams, RemoveResourceParams,
    ReparentParams, RpcDiscoverParams, ScreenshotParams, SendKeysParams, SetDebugModeParams,
    SpawnParams,
};
// Re-export all tool structs
pub use tools::{
    BevyDestroy, BevyGet, BevyGetResource, BevyInsert, BevyInsertResource, BevyList,
    BevyListResources, BevyMutateComponent, BevyMutateResource, BevyQuery, BevyRegistrySchema,
    BevyRemove, BevyRemoveResource, BevyReparent, BevyRpcDiscover, BevySpawn,
    BrpExtrasDiscoverFormat, BrpExtrasScreenshot, BrpExtrasSendKeys, BrpExtrasSetDebugMode,
};
