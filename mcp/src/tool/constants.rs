//! Tool constants and descriptions for the Bevy BRP MCP server.
//!
//! This module consolidates all tool names, descriptions, and help text for the MCP server.
//! It provides a single source of truth for all tool-related constants.

use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

// Macro to include help text files
macro_rules! include_help_text {
    ($file:expr) => {
        include_str!(concat!("../../help_text/", $file))
    };
}

/// Tool names enum with automatic snake_case serialization
#[derive(Display, EnumString, IntoStaticStr, AsRefStr, Debug, Clone, Copy, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
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
    /// `brp_list_logs` - List bevy_brp_mcp log files
    BrpListLogs,
    /// `brp_read_log` - Read bevy_brp_mcp log file contents
    BrpReadLog,
    /// `brp_delete_logs` - Delete bevy_brp_mcp log files
    BrpDeleteLogs,
    /// `brp_get_trace_log_path` - Get trace log path
    BrpGetTraceLogPath,
    /// `brp_set_tracing_level` - Set tracing level
    BrpSetTracingLevel,
}

// Macro to define BRP methods with consistent naming
macro_rules! define_tool_constants {
    // For Bevy protocol methods (bevy/*)
    (bevy, $method:ident) => {
        paste::paste! {
            pub const [<DESC_BEVY_ $method:upper>]: &str = include_help_text!(concat!("brp_tools/bevy_", stringify!($method), ".txt"));
            pub const [<BRP_METHOD_ $method:upper>]: &str = concat!("bevy/", stringify!($method));
        }
    };

    // For Bevy protocol methods with custom BRP path
    (bevy, $method:ident => $brp_path:expr) => {
        paste::paste! {
            pub const [<DESC_BEVY_ $method:upper>]: &str = include_help_text!(concat!("brp_tools/bevy_", stringify!($method), ".txt"));
            pub const [<BRP_METHOD_ $method:upper>]: &str = $brp_path;
        }
    };

    // For BRP extras methods (brp_extras/*)
    (brp_extras, $method:ident) => {
        paste::paste! {
            pub const [<DESC_BRP_EXTRAS_ $method:upper>]: &str = include_help_text!(concat!("brp_tools/brp_extras_", stringify!($method), ".txt"));
            pub const [<BRP_METHOD_EXTRAS_ $method:upper>]: &str = concat!("brp_extras/", stringify!($method));
        }
    };

    // For BRP internal tools (server-side functionality)
    (brp, $method:ident) => {
        paste::paste! {
            pub const [<DESC_$method:upper>]: &str = include_help_text!(concat!("brp_tools/brp_", stringify!($method), ".txt"));
        }
    };

    // For app management tools
    (app, $method:ident) => {
        paste::paste! {
            pub const [<DESC_ $method:upper>]: &str = include_help_text!(concat!("app_tools/brp_", stringify!($method), ".txt"));
        }
    };

    // For log management tools
    (log, $method:ident) => {
        paste::paste! {
            pub const [<DESC_ $method:upper>]: &str = include_help_text!(concat!("log_tools/brp_", stringify!($method), ".txt"));
        }
    };
}

// ============================================================================
// SPECIAL CONSTANTS
// ============================================================================

/// `bevy_brp_extras` prefix
pub const BRP_EXTRAS_PREFIX: &str = "brp_extras/";

// ============================================================================
// MCP TOOL NAMES AND DESCRIPTIONS - Generated by macros
// ============================================================================

// -----------------------------------------------------------------------------
// Core BRP Tools (Direct protocol methods)
// -----------------------------------------------------------------------------

// Generate tool constants for Bevy protocol methods
define_tool_constants!(bevy, list);
define_tool_constants!(bevy, get);
define_tool_constants!(bevy, destroy);
define_tool_constants!(bevy, insert);
define_tool_constants!(bevy, remove);
define_tool_constants!(bevy, list_resources);
define_tool_constants!(bevy, get_resource);
define_tool_constants!(bevy, insert_resource);
define_tool_constants!(bevy, remove_resource);
define_tool_constants!(bevy, mutate_resource);
define_tool_constants!(bevy, mutate_component);
define_tool_constants!(bevy, rpc_discover => "rpc.discover");
define_tool_constants!(bevy, query);
define_tool_constants!(bevy, spawn);
define_tool_constants!(bevy, registry_schema => "bevy/registry/schema");
define_tool_constants!(bevy, reparent);
define_tool_constants!(bevy, get_watch => "bevy/get+watch");
define_tool_constants!(bevy, list_watch => "bevy/list+watch");

// BRP execute tool (not a direct Bevy method, server-only) but still uses
// the HandlerType::Brp even though we don't define it the same
// we made this to execute an arbitrary command - largely for troubleshooting
// by bypassing the standard tool call flow
// it creates a bunch of exceptions in the code so i'm not sure if it's worth it...
// we'll keep it for now as it is a pretty good troubleshooting tool
pub const DESC_BRP_EXECUTE: &str = include_help_text!("brp_tools/brp_execute.txt");

// -----------------------------------------------------------------------------
// BRP Extras Tools (bevy_brp_extras plugin methods)
// -----------------------------------------------------------------------------

// Generate tool constants for BRP extras methods
define_tool_constants!(brp_extras, discover_format);
define_tool_constants!(brp_extras, screenshot);
define_tool_constants!(brp_extras, send_keys);
define_tool_constants!(brp_extras, set_debug_mode);

// Manual constant for BRP extras shutdown method (used by app shutdown tool)
// We don't want to expose it twice as an mcp tool so here we just define the method name only
// it will be called in app_tools::brp_shutdown
pub const BRP_METHOD_EXTRAS_SHUTDOWN: &str = "brp_extras/shutdown";

// -----------------------------------------------------------------------------
// BRP Watch Assist Tools (not direct protocol methods)
// -----------------------------------------------------------------------------

// Generate tool constants for BRP internal tools
define_tool_constants!(brp, stop_watch);
define_tool_constants!(brp, list_active_watches);

// -----------------------------------------------------------------------------
// Application Management Tools
// -----------------------------------------------------------------------------

// Generate tool constants for app management tools
define_tool_constants!(app, list_bevy_apps);
define_tool_constants!(app, list_bevy_examples);
define_tool_constants!(app, list_brp_apps);
define_tool_constants!(app, launch_bevy_app);
define_tool_constants!(app, launch_bevy_example);
define_tool_constants!(app, shutdown);
define_tool_constants!(app, status);

// -----------------------------------------------------------------------------
// Log Management Tools
// -----------------------------------------------------------------------------

// Generate tool constants for log management tools
define_tool_constants!(log, list_logs);
define_tool_constants!(log, read_log);
define_tool_constants!(log, delete_logs);
define_tool_constants!(log, get_trace_log_path);
define_tool_constants!(log, set_tracing_level);
