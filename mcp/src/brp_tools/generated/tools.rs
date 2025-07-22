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
use crate::tool::{
    BRP_METHOD_DESTROY, BRP_METHOD_EXTRAS_DISCOVER_FORMAT, BRP_METHOD_EXTRAS_SCREENSHOT,
    BRP_METHOD_EXTRAS_SEND_KEYS, BRP_METHOD_EXTRAS_SET_DEBUG_MODE, BRP_METHOD_GET,
    BRP_METHOD_GET_RESOURCE, BRP_METHOD_INSERT, BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST,
    BRP_METHOD_LIST_RESOURCES, BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE,
    BRP_METHOD_QUERY, BRP_METHOD_REGISTRY_SCHEMA, BRP_METHOD_REMOVE, BRP_METHOD_REMOVE_RESOURCE,
    BRP_METHOD_REPARENT, BRP_METHOD_RPC_DISCOVER, BRP_METHOD_SPAWN,
};

// Generate tool implementations (alphabetical by tool name)
define_brp_tool!(BevyDestroy, DestroyParams, BRP_METHOD_DESTROY);
define_brp_tool!(BevyGet, GetParams, BRP_METHOD_GET);
define_brp_tool!(BevyGetResource, GetResourceParams, BRP_METHOD_GET_RESOURCE);
define_brp_tool!(BevyInsert, InsertParams, BRP_METHOD_INSERT);
define_brp_tool!(
    BevyInsertResource,
    InsertResourceParams,
    BRP_METHOD_INSERT_RESOURCE
);
define_brp_tool!(BevyList, ListParams, BRP_METHOD_LIST);
define_brp_tool!(
    BevyListResources,
    ListResourcesParams,
    BRP_METHOD_LIST_RESOURCES
);
define_brp_tool!(
    BevyMutateComponent,
    MutateComponentParams,
    BRP_METHOD_MUTATE_COMPONENT
);
define_brp_tool!(
    BevyMutateResource,
    MutateResourceParams,
    BRP_METHOD_MUTATE_RESOURCE
);
define_brp_tool!(BevyQuery, QueryParams, BRP_METHOD_QUERY);
define_brp_tool!(
    BevyRegistrySchema,
    RegistrySchemaParams,
    BRP_METHOD_REGISTRY_SCHEMA
);
define_brp_tool!(BevyRemove, RemoveParams, BRP_METHOD_REMOVE);
define_brp_tool!(
    BevyRemoveResource,
    RemoveResourceParams,
    BRP_METHOD_REMOVE_RESOURCE
);
define_brp_tool!(BevyReparent, ReparentParams, BRP_METHOD_REPARENT);
define_brp_tool!(BevyRpcDiscover, RpcDiscoverParams, BRP_METHOD_RPC_DISCOVER);
define_brp_tool!(BevySpawn, SpawnParams, BRP_METHOD_SPAWN);
define_brp_tool!(
    BrpExtrasDiscoverFormat,
    DiscoverFormatParams,
    BRP_METHOD_EXTRAS_DISCOVER_FORMAT
);
define_brp_tool!(
    BrpExtrasScreenshot,
    ScreenshotParams,
    BRP_METHOD_EXTRAS_SCREENSHOT
);
define_brp_tool!(
    BrpExtrasSendKeys,
    SendKeysParams,
    BRP_METHOD_EXTRAS_SEND_KEYS
);
define_brp_tool!(
    BrpExtrasSetDebugMode,
    SetDebugModeParams,
    BRP_METHOD_EXTRAS_SET_DEBUG_MODE
);
