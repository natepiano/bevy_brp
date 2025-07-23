//! Tool constants and descriptions for the Bevy BRP MCP server.
//!
//! This module consolidates all tool names, descriptions, and help text for the MCP server.
//! It provides a single source of truth for all tool-related constants.

use bevy_brp_mcp_macros::{BrpTools, ToolDescription};
use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

/// Tool names enum with automatic `snake_case` serialization
#[derive(
    Display,
    EnumString,
    AsRefStr,
    IntoStaticStr,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ToolDescription,
    BrpTools,
)]
#[strum(serialize_all = "snake_case")]
#[tool_description(path = "../../help_text")]
pub enum ToolName {
    // Core BRP Tools (Direct protocol methods)
    /// `bevy_list` - List components on an entity or all component types
    #[brp_method("bevy/list")]
    #[brp_tool(params = "ListParams")]
    BevyList,
    /// `bevy_get` - Get component data from entities
    #[brp_method("bevy/get")]
    #[brp_tool(params = "GetParams")]
    BevyGet,
    /// `bevy_destroy` - Destroy entities permanently
    #[brp_method("bevy/destroy")]
    #[brp_tool(params = "DestroyParams")]
    BevyDestroy,
    /// `bevy_insert` - Insert or replace components on entities
    #[brp_method("bevy/insert")]
    #[brp_tool(params = "InsertParams")]
    BevyInsert,
    /// `bevy_remove` - Remove components from entities
    #[brp_method("bevy/remove")]
    #[brp_tool(params = "RemoveParams")]
    BevyRemove,
    /// `bevy_list_resources` - List all registered resources
    #[brp_method("bevy/list_resources")]
    #[brp_tool(params = "ListResourcesParams")]
    BevyListResources,
    /// `bevy_get_resource` - Get resource data
    #[brp_method("bevy/get_resource")]
    #[brp_tool(params = "GetResourceParams")]
    BevyGetResource,
    /// `bevy_insert_resource` - Insert or update resources
    #[brp_method("bevy/insert_resource")]
    #[brp_tool(params = "InsertResourceParams")]
    BevyInsertResource,
    /// `bevy_remove_resource` - Remove resources
    #[brp_method("bevy/remove_resource")]
    #[brp_tool(params = "RemoveResourceParams")]
    BevyRemoveResource,
    /// `bevy_mutate_resource` - Mutate resource fields
    #[brp_method("bevy/mutate_resource")]
    #[brp_tool(params = "MutateResourceParams")]
    BevyMutateResource,
    /// `bevy_mutate_component` - Mutate component fields
    #[brp_method("bevy/mutate_component")]
    #[brp_tool(params = "MutateComponentParams")]
    BevyMutateComponent,
    /// `bevy_rpc_discover` - Discover available BRP methods
    #[brp_method("rpc.discover")]
    #[brp_tool(params = "RpcDiscoverParams")]
    BevyRpcDiscover,
    /// `bevy_query` - Query entities by components
    #[brp_method("bevy/query")]
    #[brp_tool(params = "QueryParams")]
    BevyQuery,
    /// `bevy_spawn` - Spawn entities with components
    #[brp_method("bevy/spawn")]
    #[brp_tool(params = "SpawnParams")]
    BevySpawn,
    /// `bevy_registry_schema` - Get type schemas
    #[brp_method("bevy/registry/schema")]
    #[brp_tool(params = "RegistrySchemaParams")]
    BevyRegistrySchema,
    /// `bevy_reparent` - Change entity parents
    #[brp_method("bevy/reparent")]
    #[brp_tool(params = "ReparentParams")]
    BevyReparent,
    /// `bevy_get_watch` - Watch entity component changes
    #[brp_method("bevy/get+watch")]
    BevyGetWatch,
    /// `bevy_list_watch` - Watch entity component list changes
    #[brp_method("bevy/list+watch")]
    BevyListWatch,

    // BRP Execute Tool
    /// `brp_execute` - Execute arbitrary BRP method
    BrpExecute,

    // BRP Extras Tools
    /// `brp_extras_discover_format` - Discover component format information
    #[brp_method("brp_extras/discover_format")]
    #[brp_tool(params = "DiscoverFormatParams")]
    BrpExtrasDiscoverFormat,
    /// `brp_extras_screenshot` - Capture screenshots
    #[brp_method("brp_extras/screenshot")]
    #[brp_tool(params = "ScreenshotParams")]
    BrpExtrasScreenshot,
    /// `brp_extras_send_keys` - Send keyboard input
    #[brp_method("brp_extras/send_keys")]
    #[brp_tool(params = "SendKeysParams")]
    BrpExtrasSendKeys,
    /// `brp_extras_set_debug_mode` - Enable/disable debug mode
    #[brp_method("brp_extras/set_debug_mode")]
    #[brp_tool(params = "SetDebugModeParams")]
    BrpExtrasSetDebugMode,

    // BRP Watch Assist Tools
    /// `brp_stop_watch` - Stop active watch subscriptions
    BrpStopWatch,
    /// `brp_list_active_watches` - List active watch subscriptions
    BrpListActiveWatches,

    // Application Management Tools
    /// `brp_list_bevy_apps` - List Bevy apps in workspace
    BrpListBevyApps,
    /// `brp_list_bevy_examples` - List Bevy examples in workspace
    BrpListBevyExamples,
    /// `brp_list_brp_apps` - List BRP-enabled Bevy apps
    BrpListBrpApps,
    /// `brp_launch_bevy_app` - Launch Bevy applications
    BrpLaunchBevyApp,
    /// `brp_launch_bevy_example` - Launch Bevy examples
    BrpLaunchBevyExample,
    /// `brp_shutdown` - Shutdown running Bevy applications
    #[brp_method("brp_extras/shutdown")]
    BrpShutdown,
    /// `brp_status` - Check if Bevy app is running with BRP
    BrpStatus,

    // Log Management Tools
    /// `brp_list_logs` - List `bevy_brp_mcp` log files
    BrpListLogs,
    /// `brp_read_log` - Read `bevy_brp_mcp` log file contents
    BrpReadLog,
    /// `brp_delete_logs` - Delete `bevy_brp_mcp` log files
    BrpDeleteLogs,
    /// `brp_get_trace_log_path` - Get trace log path
    BrpGetTraceLogPath,
    /// `brp_set_tracing_level` - Set tracing level
    BrpSetTracingLevel,
}

/// `BrpMethod` is created by `BrpTools` macro - which is why you don't see the enum defined here.
/// We wanted to make it part of the `ToolName` definition as they go hand in hand. Helps prevent
/// errors of omission.
impl BrpMethod {
    /// Check if this method supports format discovery
    pub const fn supports_format_discovery(self) -> bool {
        matches!(
            self,
            Self::BevySpawn
                | Self::BevyInsert
                | Self::BevyMutateComponent
                | Self::BevyInsertResource
                | Self::BevyMutateResource
        )
    }
}
