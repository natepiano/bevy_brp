//! Tool constants and descriptions for the Bevy BRP MCP server.
//!
//! This module consolidates all tool names, descriptions, and help text for the MCP server.
//! It provides a single source of truth for all tool-related constants.

use bevy_brp_mcp_macros::ToolDescription;
use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

/// Tool names enum with automatic `snake_case` serialization
#[derive(
    Display, EnumString, IntoStaticStr, AsRefStr, Debug, Clone, Copy, PartialEq, Eq, ToolDescription,
)]
#[strum(serialize_all = "snake_case")]
#[tool_description(path = "../../help_text")]
pub enum ToolName {
    // Core BRP Tools (Direct protocol methods)
    /// `bevy_list` - List components on an entity or all component types
    BevyList,
    /// `bevy_get` - Get component data from entities
    BevyGet,
    /// `bevy_destroy` - Destroy entities permanently
    BevyDestroy,
    /// `bevy_insert` - Insert or replace components on entities
    BevyInsert,
    /// `bevy_remove` - Remove components from entities
    BevyRemove,
    /// `bevy_list_resources` - List all registered resources
    BevyListResources,
    /// `bevy_get_resource` - Get resource data
    BevyGetResource,
    /// `bevy_insert_resource` - Insert or update resources
    BevyInsertResource,
    /// `bevy_remove_resource` - Remove resources
    BevyRemoveResource,
    /// `bevy_mutate_resource` - Mutate resource fields
    BevyMutateResource,
    /// `bevy_mutate_component` - Mutate component fields
    BevyMutateComponent,
    /// `bevy_rpc_discover` - Discover available BRP methods
    BevyRpcDiscover,
    /// `bevy_query` - Query entities by components
    BevyQuery,
    /// `bevy_spawn` - Spawn entities with components
    BevySpawn,
    /// `bevy_registry_schema` - Get type schemas
    BevyRegistrySchema,
    /// `bevy_reparent` - Change entity parents
    BevyReparent,
    /// `bevy_get_watch` - Watch entity component changes
    BevyGetWatch,
    /// `bevy_list_watch` - Watch entity component list changes
    BevyListWatch,

    // BRP Execute Tool
    /// `brp_execute` - Execute arbitrary BRP method
    BrpExecute,

    // BRP Extras Tools
    /// `brp_extras_discover_format` - Discover component format information
    BrpExtrasDiscoverFormat,
    /// `brp_extras_screenshot` - Capture screenshots
    BrpExtrasScreenshot,
    /// `brp_extras_send_keys` - Send keyboard input
    BrpExtrasSendKeys,
    /// `brp_extras_set_debug_mode` - Enable/disable debug mode
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

// ============================================================================
// SPECIAL CONSTANTS
// ============================================================================

/// `bevy_brp_extras` prefix
pub const BRP_EXTRAS_PREFIX: &str = "brp_extras/";

// ============================================================================
// BRP METHOD CONSTANTS
// ============================================================================

// -----------------------------------------------------------------------------
// Core BRP Protocol Methods
// -----------------------------------------------------------------------------

// Bevy protocol methods
pub const BRP_METHOD_LIST: &str = "bevy/list";
pub const BRP_METHOD_GET: &str = "bevy/get";
pub const BRP_METHOD_DESTROY: &str = "bevy/destroy";
pub const BRP_METHOD_INSERT: &str = "bevy/insert";
pub const BRP_METHOD_REMOVE: &str = "bevy/remove";
pub const BRP_METHOD_LIST_RESOURCES: &str = "bevy/list_resources";
pub const BRP_METHOD_GET_RESOURCE: &str = "bevy/get_resource";
pub const BRP_METHOD_INSERT_RESOURCE: &str = "bevy/insert_resource";
pub const BRP_METHOD_REMOVE_RESOURCE: &str = "bevy/remove_resource";
pub const BRP_METHOD_MUTATE_RESOURCE: &str = "bevy/mutate_resource";
pub const BRP_METHOD_MUTATE_COMPONENT: &str = "bevy/mutate_component";
pub const BRP_METHOD_RPC_DISCOVER: &str = "rpc.discover";
pub const BRP_METHOD_QUERY: &str = "bevy/query";
pub const BRP_METHOD_SPAWN: &str = "bevy/spawn";
pub const BRP_METHOD_REGISTRY_SCHEMA: &str = "bevy/registry/schema";
pub const BRP_METHOD_REPARENT: &str = "bevy/reparent";
pub const BRP_METHOD_GET_WATCH: &str = "bevy/get+watch";
pub const BRP_METHOD_LIST_WATCH: &str = "bevy/list+watch";

// -----------------------------------------------------------------------------
// BRP Extras Protocol Methods
// -----------------------------------------------------------------------------

pub const BRP_METHOD_EXTRAS_DISCOVER_FORMAT: &str = "brp_extras/discover_format";
pub const BRP_METHOD_EXTRAS_SCREENSHOT: &str = "brp_extras/screenshot";
pub const BRP_METHOD_EXTRAS_SEND_KEYS: &str = "brp_extras/send_keys";
pub const BRP_METHOD_EXTRAS_SET_DEBUG_MODE: &str = "brp_extras/set_debug_mode";
pub const BRP_METHOD_EXTRAS_SHUTDOWN: &str = "brp_extras/shutdown";
