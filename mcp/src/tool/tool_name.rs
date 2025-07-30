//! Tool constants and descriptions for the Bevy BRP MCP server.
//!
//! This module consolidates all tool names, descriptions, and help text for the MCP server.
//! It provides a single source of truth for all tool-related constants.

use bevy_brp_mcp_macros::{BrpTools, ToolDescription};
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

use crate::app_tools::{
    self, LaunchBevyAppParams, LaunchBevyExampleParams, ListBevyApps, ListBevyExamples,
    ListBrpApps, Shutdown, ShutdownParams, Status, StatusParams,
};
// Import parameter and result types so they're in scope for the macro
use crate::brp_tools::{
    DestroyParams, DestroyResult, DiscoverFormatParams, DiscoverFormatResult, ExecuteParams,
    GetParams, GetResourceParams, GetResourceResult, GetResult, GetWatchParams, InsertParams,
    InsertResourceParams, InsertResourceResult, InsertResult, ListParams, ListResourcesParams,
    ListResourcesResult, ListResult, ListWatchParams, MutateComponentParams, MutateComponentResult,
    MutateResourceParams, MutateResourceResult, QueryParams, QueryResult, RegistrySchemaParams,
    RegistrySchemaResult, RemoveParams, RemoveResourceParams, RemoveResourceResult, RemoveResult,
    ReparentParams, ReparentResult, RpcDiscoverParams, RpcDiscoverResult, ScreenshotParams,
    ScreenshotResult, SendKeysParams, SendKeysResult, SpawnParams, SpawnResult, StopWatchParams,
};
use crate::log_tools::{
    DeleteLogs, DeleteLogsParams, GetTraceLogPath, ListLogs, ListLogsParams, ReadLog,
    ReadLogParams, SetTracingLevel, SetTracingLevelParams,
};

/// Tool names enum with automatic `snake_case` serialization
#[derive(
    AsRefStr,
    BrpTools,
    Clone,
    Copy,
    Debug,
    Display,
    EnumIter,
    EnumString,
    Eq,
    IntoStaticStr,
    PartialEq,
    ToolDescription,
)]
#[strum(serialize_all = "snake_case")]
#[tool_description(path = "../../help_text")]
pub enum ToolName {
    // Core BRP Tools (Direct protocol methods)
    /// `bevy_list` - List components on an entity or all component types
    #[brp_tool(brp_method = "bevy/list", params = "ListParams", result = "ListResult")]
    BevyList,
    /// `bevy_get` - Get component data from entities
    #[brp_tool(brp_method = "bevy/get", params = "GetParams", result = "GetResult")]
    BevyGet,
    /// `bevy_destroy` - Destroy entities permanently
    #[brp_tool(
        brp_method = "bevy/destroy",
        params = "DestroyParams",
        result = "DestroyResult"
    )]
    BevyDestroy,
    /// `bevy_insert` - Insert or replace components on entities
    #[brp_tool(
        brp_method = "bevy/insert",
        params = "InsertParams",
        result = "InsertResult",
        format_discovery = true
    )]
    BevyInsert,
    /// `bevy_remove` - Remove components from entities
    #[brp_tool(
        brp_method = "bevy/remove",
        params = "RemoveParams",
        result = "RemoveResult"
    )]
    BevyRemove,
    /// `bevy_list_resources` - List all registered resources
    #[brp_tool(
        brp_method = "bevy/list_resources",
        params = "ListResourcesParams",
        result = "ListResourcesResult"
    )]
    BevyListResources,
    /// `bevy_get_resource` - Get resource data
    #[brp_tool(
        brp_method = "bevy/get_resource",
        params = "GetResourceParams",
        result = "GetResourceResult"
    )]
    BevyGetResource,
    /// `bevy_insert_resource` - Insert or update resources
    #[brp_tool(
        brp_method = "bevy/insert_resource",
        params = "InsertResourceParams",
        result = "InsertResourceResult",
        format_discovery = true
    )]
    BevyInsertResource,
    /// `bevy_remove_resource` - Remove resources
    #[brp_tool(
        brp_method = "bevy/remove_resource",
        params = "RemoveResourceParams",
        result = "RemoveResourceResult"
    )]
    BevyRemoveResource,
    /// `bevy_mutate_resource` - Mutate resource fields
    #[brp_tool(
        brp_method = "bevy/mutate_resource",
        params = "MutateResourceParams",
        result = "MutateResourceResult",
        format_discovery = true
    )]
    BevyMutateResource,
    /// `bevy_mutate_component` - Mutate component fields
    #[brp_tool(
        brp_method = "bevy/mutate_component",
        params = "MutateComponentParams",
        result = "MutateComponentResult",
        format_discovery = true
    )]
    BevyMutateComponent,
    /// `bevy_rpc_discover` - Discover available BRP methods
    #[brp_tool(
        brp_method = "rpc.discover",
        params = "RpcDiscoverParams",
        result = "RpcDiscoverResult"
    )]
    BevyRpcDiscover,
    /// `bevy_query` - Query entities by components
    #[brp_tool(
        brp_method = "bevy/query",
        params = "QueryParams",
        result = "QueryResult"
    )]
    BevyQuery,
    /// `bevy_spawn` - Spawn entities with components
    #[brp_tool(
        brp_method = "bevy/spawn",
        params = "SpawnParams",
        result = "SpawnResult",
        format_discovery = true
    )]
    BevySpawn,
    /// `bevy_registry_schema` - Get type schemas
    #[brp_tool(
        brp_method = "bevy/registry/schema",
        params = "RegistrySchemaParams",
        result = "RegistrySchemaResult"
    )]
    BevyRegistrySchema,
    /// `bevy_reparent` - Change entity parents
    #[brp_tool(
        brp_method = "bevy/reparent",
        params = "ReparentParams",
        result = "ReparentResult"
    )]
    BevyReparent,
    /// `bevy_get_watch` - Watch entity component changes
    #[brp_tool(brp_method = "bevy/get+watch")]
    BevyGetWatch,
    /// `bevy_list_watch` - Watch entity component list changes
    #[brp_tool(brp_method = "bevy/list+watch")]
    BevyListWatch,

    // BRP Execute Tool
    /// `brp_execute` - Execute arbitrary BRP method
    BrpExecute,

    // BRP Extras Tools
    /// `brp_extras_discover_format` - Discover component format information
    #[brp_tool(
        brp_method = "brp_extras/discover_format",
        params = "DiscoverFormatParams",
        result = "DiscoverFormatResult"
    )]
    BrpExtrasDiscoverFormat,
    /// `brp_extras_screenshot` - Capture screenshots
    #[brp_tool(
        brp_method = "brp_extras/screenshot",
        params = "ScreenshotParams",
        result = "ScreenshotResult"
    )]
    BrpExtrasScreenshot,
    /// `brp_extras_send_keys` - Send keyboard input
    #[brp_tool(
        brp_method = "brp_extras/send_keys",
        params = "SendKeysParams",
        result = "SendKeysResult"
    )]
    BrpExtrasSendKeys,

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
    #[brp_tool(brp_method = "brp_extras/shutdown")]
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

use std::sync::Arc;

use rmcp::model::CallToolResult;

// Import special tools that aren't generated by the macro
use crate::brp_tools as brp_tool_impl;
use crate::error::{Error, Result};
use crate::tool::annotations::{Annotation, EnvironmentImpact, ToolCategory};
use crate::tool::types::ErasedToolFn;
use crate::tool::{
    CallInfo, HandlerContext, JsonResponse, LargeResponseConfig, ParamStruct, ResponseBuilder,
    ResultStruct, ToolResult, handle_large_response, parameters,
};

impl ToolName {
    /// Get `CallInfo` for this tool
    ///
    /// This method creates the appropriate `CallInfo` variant based on the tool type:
    /// - BRP tools get `CallInfo::Brp`
    /// - Non-BRP tools get `CallInfo::Local`
    pub fn get_call_info(self) -> CallInfo {
        let tool_name = self.to_string();
        match self.to_brp_method() {
            Some(brp_method) => CallInfo::brp(tool_name, brp_method.as_str().to_string()),
            None => CallInfo::local(tool_name),
        }
    }

    /// Get annotations for this tool
    #[allow(clippy::too_many_lines)]
    pub fn get_annotations(self) -> Annotation {
        match self {
            Self::BevyDestroy => Annotation::new(
                "Destroy Bevy Entity",
                ToolCategory::Entity,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            Self::BevyGet => Annotation::new(
                "Get Component Data",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevyGetResource => Annotation::new(
                "Get Resource Data",
                ToolCategory::Resource,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevyInsert => Annotation::new(
                "Insert Components",
                ToolCategory::Component,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BevyInsertResource => Annotation::new(
                "Insert Resource",
                ToolCategory::Resource,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BevyList => Annotation::new(
                "List Components",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevyListResources => Annotation::new(
                "List Resources",
                ToolCategory::Resource,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevyMutateComponent => Annotation::new(
                "Mutate Component",
                ToolCategory::Component,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BevyMutateResource => Annotation::new(
                "Mutate Resource",
                ToolCategory::Resource,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BevyQuery => Annotation::new(
                "Query Entities/Components",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevyRegistrySchema => Annotation::new(
                "Get Type Schemas",
                ToolCategory::Discovery,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevyRemove => Annotation::new(
                "Remove Components",
                ToolCategory::Component,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            Self::BevyRemoveResource => Annotation::new(
                "Remove Resource",
                ToolCategory::Resource,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            Self::BevyReparent => Annotation::new(
                "Reparent Entities",
                ToolCategory::Entity,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BevyRpcDiscover => Annotation::new(
                "Discover BRP Methods",
                ToolCategory::Discovery,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BevySpawn => Annotation::new(
                "Spawn Entity",
                ToolCategory::Entity,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BrpExecute => Annotation::new(
                "Execute BRP Method",
                ToolCategory::DynamicBrp,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BrpExtrasDiscoverFormat => Annotation::new(
                "Discover Component Format",
                ToolCategory::Extras,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpExtrasScreenshot => Annotation::new(
                "Take Screenshot",
                ToolCategory::Extras,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BrpExtrasSendKeys => Annotation::new(
                "Send Keys",
                ToolCategory::Extras,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BevyGetWatch => Annotation::new(
                "Watch Component Changes",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BevyListWatch => Annotation::new(
                "Watch Component List",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BrpDeleteLogs => Annotation::new(
                "Delete Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            Self::BrpGetTraceLogPath => Annotation::new(
                "Get Trace Log Path",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpLaunchBevyApp => Annotation::new(
                "Launch Bevy App",
                ToolCategory::App,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BrpLaunchBevyExample => Annotation::new(
                "Launch Bevy Example",
                ToolCategory::App,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            Self::BrpListBevyApps => Annotation::new(
                "List Bevy Apps",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpListBevyExamples => Annotation::new(
                "List Bevy Examples",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpListBrpApps => Annotation::new(
                "List Bevy BRP-enabled Apps",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpListActiveWatches => Annotation::new(
                "List Active Watches",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpStopWatch => Annotation::new(
                "Stop Watch",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            Self::BrpListLogs => Annotation::new(
                "List Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpReadLog => Annotation::new(
                "Read Log File",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpSetTracingLevel => Annotation::new(
                "Set Tracing Level",
                ToolCategory::Logging,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            Self::BrpStatus => Annotation::new(
                "Check App Status",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            Self::BrpShutdown => Annotation::new(
                "Shutdown Bevy App",
                ToolCategory::App,
                EnvironmentImpact::DestructiveIdempotent,
            ),
        }
    }

    /// Get parameter builder function for this tool
    #[allow(clippy::too_many_lines)]
    pub fn get_parameters(self) -> Option<fn() -> parameters::ParameterBuilder> {
        match self {
            Self::BevyDestroy => Some(parameters::build_parameters_from::<DestroyParams>),
            Self::BevyGet => Some(parameters::build_parameters_from::<GetParams>),
            Self::BevyGetResource => Some(parameters::build_parameters_from::<GetResourceParams>),
            Self::BevyInsert => Some(parameters::build_parameters_from::<InsertParams>),
            Self::BevyInsertResource => {
                Some(parameters::build_parameters_from::<InsertResourceParams>)
            }
            Self::BevyList => Some(parameters::build_parameters_from::<ListParams>),
            Self::BevyListResources => {
                Some(parameters::build_parameters_from::<ListResourcesParams>)
            }
            Self::BevyMutateComponent => {
                Some(parameters::build_parameters_from::<MutateComponentParams>)
            }
            Self::BevyMutateResource => {
                Some(parameters::build_parameters_from::<MutateResourceParams>)
            }
            Self::BevyQuery => Some(parameters::build_parameters_from::<QueryParams>),
            Self::BevyRegistrySchema => {
                Some(parameters::build_parameters_from::<RegistrySchemaParams>)
            }
            Self::BevyRemove => Some(parameters::build_parameters_from::<RemoveParams>),
            Self::BevyRemoveResource => {
                Some(parameters::build_parameters_from::<RemoveResourceParams>)
            }
            Self::BevyReparent => Some(parameters::build_parameters_from::<ReparentParams>),
            Self::BevyRpcDiscover => Some(parameters::build_parameters_from::<RpcDiscoverParams>),
            Self::BevySpawn => Some(parameters::build_parameters_from::<SpawnParams>),
            Self::BrpExecute => Some(parameters::build_parameters_from::<ExecuteParams>),
            Self::BrpExtrasDiscoverFormat => {
                Some(parameters::build_parameters_from::<DiscoverFormatParams>)
            }
            Self::BrpExtrasScreenshot => {
                Some(parameters::build_parameters_from::<ScreenshotParams>)
            }
            Self::BrpExtrasSendKeys => Some(parameters::build_parameters_from::<SendKeysParams>),
            Self::BevyGetWatch => Some(parameters::build_parameters_from::<GetWatchParams>),
            Self::BevyListWatch => Some(parameters::build_parameters_from::<ListWatchParams>),
            Self::BrpDeleteLogs => Some(parameters::build_parameters_from::<DeleteLogsParams>),
            Self::BrpGetTraceLogPath
            | Self::BrpListBevyApps
            | Self::BrpListBevyExamples
            | Self::BrpListBrpApps
            | Self::BrpListActiveWatches => None,
            Self::BrpLaunchBevyApp => {
                Some(parameters::build_parameters_from::<LaunchBevyAppParams>)
            }
            Self::BrpLaunchBevyExample => {
                Some(parameters::build_parameters_from::<LaunchBevyExampleParams>)
            }
            Self::BrpStopWatch => Some(parameters::build_parameters_from::<StopWatchParams>),
            Self::BrpListLogs => Some(parameters::build_parameters_from::<ListLogsParams>),
            Self::BrpReadLog => Some(parameters::build_parameters_from::<ReadLogParams>),
            Self::BrpSetTracingLevel => {
                Some(parameters::build_parameters_from::<SetTracingLevelParams>)
            }
            Self::BrpStatus => Some(parameters::build_parameters_from::<StatusParams>),
            Self::BrpShutdown => Some(parameters::build_parameters_from::<ShutdownParams>),
        }
    }

    /// Type-safe formatter that accepts our internal Result directly
    pub fn format_result<T, P>(
        self,
        tool_result: ToolResult<T, P>,
        handler_context: &HandlerContext,
    ) -> CallToolResult
    where
        T: ResultStruct,
        P: ParamStruct,
    {
        // Create CallInfo using self
        let call_info = self.get_call_info();

        match tool_result.result {
            Ok(data) => {
                // Keep existing error handling logic but use build_with_result_struct
                let response = match Self::build_response_with_result_struct(
                    &data,
                    tool_result.params,
                    call_info.clone(),
                    handler_context,
                ) {
                    Ok(response) => response,
                    Err(e) => {
                        // If building the response fails, return an error response
                        ResponseBuilder::error(call_info)
                            .message(format!("Failed to build response: {}", e.current_context()))
                            .build()
                    }
                };
                Self::handle_large_response(response, self)
            }
            Err(report) => match report.current_context() {
                Error::Structured { result } => {
                    tracing::debug!("Processing structured error with result type");
                    let response = match ResponseBuilder::error(call_info.clone())
                        .build_with_result_struct(
                            result.as_ref(),
                            tool_result.params,
                            handler_context,
                        ) {
                        Ok(response) => {
                            tracing::debug!("Successfully built structured error response");
                            response
                        }
                        Err(e) => {
                            // If building the error response fails, return a fallback error
                            tracing::error!(
                                "Failed to build structured error response: {}",
                                e.current_context()
                            );
                            ResponseBuilder::error(call_info)
                                .message(format!(
                                    "Failed to build error response: {}",
                                    e.current_context()
                                ))
                                .build()
                        }
                    };
                    Self::handle_large_response(response, self)
                }
                Error::ToolCall { message, details } => ResponseBuilder::error(call_info)
                    .message(message)
                    .add_optional_details(details.as_ref())
                    .build()
                    .to_call_tool_result(),
                _ => ResponseBuilder::error(call_info)
                    .message(format!("Internal error: {}", report.current_context()))
                    .build()
                    .to_call_tool_result(),
            },
        }
    }

    /// Format framework errors (parameter extraction failures, etc)
    pub fn format_framework_error(
        self,
        error: error_stack::Report<crate::error::Error>,
        _handler_context: &HandlerContext,
    ) -> CallToolResult {
        tracing::debug!(
            "format_framework_error called for tool: {}",
            self.to_string()
        );
        tracing::trace!("Framework error details: {:#}", error);
        // Framework errors use the standard call info
        let call_info = self.get_call_info();

        ResponseBuilder::error(call_info)
            .message(format!("Framework error: {}", error.current_context()))
            .build()
            .to_call_tool_result()
    }

    /// large response processing - this would possibly be where we would implement pagination
    fn handle_large_response(response: JsonResponse, tool_name: Self) -> CallToolResult {
        // Check if response is too large and handle result field extraction
        match handle_large_response(response, tool_name, LargeResponseConfig::default()) {
            Ok(processed_response) => processed_response.to_call_tool_result(),
            Err(e) => {
                // If large response handling fails, return an error response
                ResponseBuilder::error(tool_name.get_call_info())
                    .message(format!(
                        "Failed to process response: {}",
                        e.current_context()
                    ))
                    .build()
                    .to_call_tool_result()
            }
        }
    }

    fn build_response_with_result_struct<R: ResultStruct + ?Sized, P: ParamStruct>(
        result: &R,
        params: Option<P>,
        call_info: CallInfo,
        handler_context: &HandlerContext,
    ) -> Result<JsonResponse> {
        ResponseBuilder::success(call_info).build_with_result_struct(
            result,
            params,
            handler_context,
        )
    }

    /// Create handler for this tool
    #[allow(clippy::too_many_lines)]
    pub fn create_handler(self) -> Arc<dyn ErasedToolFn> {
        match self {
            // BRP tools generated by the macro
            Self::BevyDestroy => Arc::new(BevyDestroy),
            Self::BevyGet => Arc::new(BevyGet),
            Self::BevyGetResource => Arc::new(BevyGetResource),
            Self::BevyInsert => Arc::new(BevyInsert),
            Self::BevyInsertResource => Arc::new(BevyInsertResource),
            Self::BevyList => Arc::new(BevyList),
            Self::BevyListResources => Arc::new(BevyListResources),
            Self::BevyMutateComponent => Arc::new(BevyMutateComponent),
            Self::BevyMutateResource => Arc::new(BevyMutateResource),
            Self::BevyQuery => Arc::new(BevyQuery),
            Self::BevyRegistrySchema => Arc::new(BevyRegistrySchema),
            Self::BevyRemove => Arc::new(BevyRemove),
            Self::BevyRemoveResource => Arc::new(BevyRemoveResource),
            Self::BevyReparent => Arc::new(BevyReparent),
            Self::BevyRpcDiscover => Arc::new(BevyRpcDiscover),
            Self::BevySpawn => Arc::new(BevySpawn),
            Self::BrpExtrasDiscoverFormat => Arc::new(BrpExtrasDiscoverFormat),
            Self::BrpExtrasScreenshot => Arc::new(BrpExtrasScreenshot),
            Self::BrpExtrasSendKeys => Arc::new(BrpExtrasSendKeys),

            // Special tools with their own implementations
            Self::BrpExecute => Arc::new(brp_tool_impl::BrpExecute),
            Self::BevyGetWatch => Arc::new(brp_tool_impl::BevyGetWatch),
            Self::BevyListWatch => Arc::new(brp_tool_impl::BevyListWatch),
            Self::BrpListActiveWatches => Arc::new(brp_tool_impl::BrpListActiveWatches),
            Self::BrpStopWatch => Arc::new(brp_tool_impl::BrpStopWatch),

            // App tools
            Self::BrpDeleteLogs => Arc::new(DeleteLogs),
            Self::BrpGetTraceLogPath => Arc::new(GetTraceLogPath),
            Self::BrpLaunchBevyApp => Arc::new(app_tools::create_launch_bevy_app_handler()),
            Self::BrpLaunchBevyExample => Arc::new(app_tools::create_launch_bevy_example_handler()),
            Self::BrpListBevyApps => Arc::new(ListBevyApps),
            Self::BrpListBevyExamples => Arc::new(ListBevyExamples),
            Self::BrpListBrpApps => Arc::new(ListBrpApps),
            Self::BrpListLogs => Arc::new(ListLogs),
            Self::BrpReadLog => Arc::new(ReadLog),
            Self::BrpSetTracingLevel => Arc::new(SetTracingLevel),
            Self::BrpStatus => Arc::new(Status),
            Self::BrpShutdown => Arc::new(Shutdown),
        }
    }

    /// Convert this tool name to a complete `ToolDef`
    pub fn to_tool_def(self) -> crate::tool::ToolDef {
        crate::tool::ToolDef {
            tool_name:   self,
            annotations: self.get_annotations(),
            handler:     self.create_handler(),
            parameters:  self.get_parameters(),
        }
    }
}

/// Get all tool definitions for registration with the MCP service
pub fn get_all_tool_definitions() -> Vec<crate::tool::ToolDef> {
    use strum::IntoEnumIterator;

    ToolName::iter().map(ToolName::to_tool_def).collect()
}
