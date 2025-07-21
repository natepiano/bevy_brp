/// BRP tool definitions generated via macro
///
/// Tools are listed alphabetically by tool struct name (e.g., `BevyDestroy`, `BevyGet`, etc.)
/// Each tool references its corresponding params struct from params.rs
use super::macro_def::define_brp_tool;
use super::params::{
    DestroyParams, DiscoverFormatParams, GetParams, GetResourceParams, InsertParams,
    InsertResourceParams, ListParams, ListResourcesParams, MutateComponentParams,
    MutateResourceParams, QueryParams, RegistrySchemaParams, RemoveParams, RemoveResourceParams,
    ReparentParams, RpcDiscoverParams, ScreenshotParams, SendKeysParams, SetDebugModeParams,
    SpawnParams,
};

// Generate tool implementations (alphabetical by tool name)
define_brp_tool!(BevyDestroy, DestroyParams);
define_brp_tool!(BevyGet, GetParams);
define_brp_tool!(BevyGetResource, GetResourceParams);
define_brp_tool!(BevyInsert, InsertParams);
define_brp_tool!(BevyInsertResource, InsertResourceParams);
define_brp_tool!(BevyList, ListParams);
define_brp_tool!(BevyListResources, ListResourcesParams);
define_brp_tool!(BevyMutateComponent, MutateComponentParams);
define_brp_tool!(BevyMutateResource, MutateResourceParams);
define_brp_tool!(BevyQuery, QueryParams);
define_brp_tool!(BevyRegistrySchema, RegistrySchemaParams);
define_brp_tool!(BevyRemove, RemoveParams);
define_brp_tool!(BevyRemoveResource, RemoveResourceParams);
define_brp_tool!(BevyReparent, ReparentParams);
define_brp_tool!(BevyRpcDiscover, RpcDiscoverParams);
define_brp_tool!(BevySpawn, SpawnParams);
define_brp_tool!(BrpExtrasDiscoverFormat, DiscoverFormatParams);
define_brp_tool!(BrpExtrasScreenshot, ScreenshotParams);
define_brp_tool!(BrpExtrasSendKeys, SendKeysParams);
define_brp_tool!(BrpExtrasSetDebugMode, SetDebugModeParams);
