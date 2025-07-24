//! Tool constants and descriptions for the Bevy BRP MCP server.
//!
//! This module consolidates all tool names, descriptions, and help text for the MCP server.
//! It provides a single source of truth for all tool-related constants.

use bevy_brp_mcp_macros::{BrpTools, ToolDescription};
use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

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
    strum::EnumIter,
)]
#[strum(serialize_all = "snake_case")]
#[tool_description(path = "../../help_text")]
pub enum ToolName {
    // Core BRP Tools (Direct protocol methods)
    /// `bevy_list` - List components on an entity or all component types
    #[tool(brp_method = "bevy/list", params = "ListParams", result = "ListResult")]
    BevyList,
    /// `bevy_get` - Get component data from entities
    #[tool(brp_method = "bevy/get", params = "GetParams", result = "GetResult")]
    BevyGet,
    /// `bevy_destroy` - Destroy entities permanently
    #[tool(
        brp_method = "bevy/destroy",
        params = "DestroyParams",
        result = "DestroyResult"
    )]
    BevyDestroy,
    /// `bevy_insert` - Insert or replace components on entities
    #[tool(
        brp_method = "bevy/insert",
        params = "InsertParams",
        result = "InsertResult"
    )]
    BevyInsert,
    /// `bevy_remove` - Remove components from entities
    #[tool(
        brp_method = "bevy/remove",
        params = "RemoveParams",
        result = "RemoveResult"
    )]
    BevyRemove,
    /// `bevy_list_resources` - List all registered resources
    #[tool(
        brp_method = "bevy/list_resources",
        params = "ListResourcesParams",
        result = "ListResourcesResult"
    )]
    BevyListResources,
    /// `bevy_get_resource` - Get resource data
    #[tool(
        brp_method = "bevy/get_resource",
        params = "GetResourceParams",
        result = "GetResourceResult"
    )]
    BevyGetResource,
    /// `bevy_insert_resource` - Insert or update resources
    #[tool(
        brp_method = "bevy/insert_resource",
        params = "InsertResourceParams",
        result = "InsertResourceResult"
    )]
    BevyInsertResource,
    /// `bevy_remove_resource` - Remove resources
    #[tool(
        brp_method = "bevy/remove_resource",
        params = "RemoveResourceParams",
        result = "RemoveResourceResult"
    )]
    BevyRemoveResource,
    /// `bevy_mutate_resource` - Mutate resource fields
    #[tool(
        brp_method = "bevy/mutate_resource",
        params = "MutateResourceParams",
        result = "MutateResourceResult"
    )]
    BevyMutateResource,
    /// `bevy_mutate_component` - Mutate component fields
    #[tool(
        brp_method = "bevy/mutate_component",
        params = "MutateComponentParams",
        result = "MutateComponentResult"
    )]
    BevyMutateComponent,
    /// `bevy_rpc_discover` - Discover available BRP methods
    #[tool(
        brp_method = "rpc.discover",
        params = "RpcDiscoverParams",
        result = "RpcDiscoverResult"
    )]
    BevyRpcDiscover,
    /// `bevy_query` - Query entities by components
    #[tool(
        brp_method = "bevy/query",
        params = "QueryParams",
        result = "QueryResult"
    )]
    BevyQuery,
    /// `bevy_spawn` - Spawn entities with components
    #[tool(
        brp_method = "bevy/spawn",
        params = "SpawnParams",
        result = "SpawnResult"
    )]
    BevySpawn,
    /// `bevy_registry_schema` - Get type schemas
    #[tool(
        brp_method = "bevy/registry/schema",
        params = "RegistrySchemaParams",
        result = "RegistrySchemaResult"
    )]
    BevyRegistrySchema,
    /// `bevy_reparent` - Change entity parents
    #[tool(
        brp_method = "bevy/reparent",
        params = "ReparentParams",
        result = "ReparentResult"
    )]
    BevyReparent,
    /// `bevy_get_watch` - Watch entity component changes
    #[tool(brp_method = "bevy/get+watch")]
    BevyGetWatch,
    /// `bevy_list_watch` - Watch entity component list changes
    #[tool(brp_method = "bevy/list+watch")]
    BevyListWatch,

    // BRP Execute Tool
    /// `brp_execute` - Execute arbitrary BRP method
    BrpExecute,

    // BRP Extras Tools
    /// `brp_extras_discover_format` - Discover component format information
    #[tool(
        brp_method = "brp_extras/discover_format",
        params = "DiscoverFormatParams",
        result = "DiscoverFormatResult"
    )]
    BrpExtrasDiscoverFormat,
    /// `brp_extras_screenshot` - Capture screenshots
    #[tool(
        brp_method = "brp_extras/screenshot",
        params = "ScreenshotParams",
        result = "ScreenshotResult"
    )]
    BrpExtrasScreenshot,
    /// `brp_extras_send_keys` - Send keyboard input
    #[tool(
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
    #[tool(brp_method = "brp_extras/shutdown")]
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
use serde_json::Value;

// Import special tools that aren't generated by the macro
use crate::brp_tools as brp_tool_impl;
use crate::error::{Error, Result};
use crate::tool::annotations::{Annotation, EnvironmentImpact, ToolCategory};
use crate::tool::types::ErasedUnifiedToolFn;
use crate::tool::{
    CallInfoProvider, HandlerContext, JsonResponse, LargeResponseConfig, MessageTemplate,
    ResponseBuilder, ResponseData, handle_large_response, parameters,
};

impl ToolName {
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

    /// Get message template for this tool
    pub const fn get_message_template(self) -> MessageTemplate {
        MessageTemplate {
            ok: match self {
                Self::BevyDestroy => "Successfully destroyed entity {entity}",
                Self::BevyGet => {
                    "Retrieved component data from entity {entity} - component count: {component_count}"
                }
                Self::BevyGetResource => "Retrieved resource: {resource}",
                Self::BevyInsert => "Successfully inserted components into entity {entity}",
                Self::BevyInsertResource => "Successfully inserted/updated resource: {resource}",
                Self::BevyList => "Listed {component_count} components",
                Self::BevyListResources => "Listed {resource_count} resources",
                Self::BevyMutateComponent => "Successfully mutated component on entity {entity}",
                Self::BevyMutateResource => "Successfully mutated resource: `{resource}`",
                Self::BevyQuery => "Query completed successfully",
                Self::BevyRegistrySchema => "Retrieved schema information",
                Self::BevyRemove => "Successfully removed components from entity {entity}",
                Self::BevyRemoveResource => "Successfully removed resource",
                Self::BevyReparent => "Successfully reparented entities",
                Self::BevyRpcDiscover => {
                    "Retrieved BRP method discovery information for {method_count} methods"
                }
                Self::BevySpawn => "Successfully spawned entity",
                Self::BrpExecute => "Method executed successfully",
                Self::BrpExtrasDiscoverFormat => "Format discovery completed",
                Self::BrpExtrasScreenshot => "Successfully captured screenshot",
                Self::BrpExtrasSendKeys => "Successfully sent keyboard input",
                Self::BevyGetWatch => "Started entity watch {watch_id} for entity {entity}",
                Self::BevyListWatch => "Started list watch {watch_id} for entity {entity}",
                Self::BrpDeleteLogs => "Deleted {deleted_count} log files",
                Self::BrpGetTraceLogPath => "Trace log found",
                Self::BrpLaunchBevyApp => {
                    "Successfully launched bevy app '{target_name}' (PID: {pid})"
                }
                Self::BrpLaunchBevyExample => {
                    "Successfully launched example '{target_name}' (PID: {pid})"
                }
                Self::BrpListBevyApps => "Found {count} Bevy apps",
                Self::BrpListBevyExamples => "Found {count} Bevy examples",
                Self::BrpListBrpApps => "Found {count} BRP-enabled apps",
                Self::BrpListActiveWatches => "Found {count} active watches",
                Self::BrpStopWatch => "Successfully stopped watch",
                Self::BrpListLogs => "Found {log_count} log files",
                Self::BrpReadLog => "Successfully read log file: {filename}",
                Self::BrpSetTracingLevel => {
                    "Tracing level set to '{tracing_level}' - diagnostic information will be logged to temp directory"
                }
                Self::BrpStatus | Self::BrpShutdown => "{message}",
            },
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
    pub fn format_result<T, C>(
        self,
        call_info_data: C,
        result: Result<T>,
        handler_context: &HandlerContext,
    ) -> Result<CallToolResult>
    where
        T: ResponseData,
        C: CallInfoProvider,
    {
        let call_info = call_info_data.to_call_info(self.to_string());

        match result {
            Ok(data) => {
                // Build response using ResponseData trait
                let builder = ResponseBuilder::success(call_info);
                let builder = data
                    .add_response_fields(builder)
                    .map_err(|e| Error::failed_to("add response fields", e))?;

                // Perform template substitution
                let message_template = self.get_message_template();
                let message =
                    Self::substitute_template(&message_template, &builder, handler_context);
                let builder = builder.message(message);

                let response = builder.build();
                Self::handle_large_response(response, self)
            }
            Err(report) => match report.current_context() {
                Error::ToolCall { message, details } => Ok(ResponseBuilder::error(call_info)
                    .message(message)
                    .add_optional_details(details.as_ref())
                    .build()
                    .to_call_tool_result()),
                _ => Ok(ResponseBuilder::error(call_info)
                    .message(format!("Internal error: {}", report.current_context()))
                    .build()
                    .to_call_tool_result()),
            },
        }
    }

    /// Format framework errors (parameter extraction failures, etc) with `LocalCallInfo` default
    pub fn format_framework_error(
        self,
        error: error_stack::Report<crate::error::Error>,
        _handler_context: &HandlerContext,
    ) -> CallToolResult {
        let call_info = crate::tool::LocalCallInfo.to_call_info(self.to_string());

        ResponseBuilder::error(call_info)
            .message(format!("Framework error: {}", error.current_context()))
            .build()
            .to_call_tool_result()
    }

    /// Substitute template placeholders with values from the builder
    fn substitute_template(
        template: &MessageTemplate,
        builder: &ResponseBuilder,
        handler_context: &HandlerContext,
    ) -> String {
        let mut result = template.ok.to_string();

        // Extract placeholders from template
        let placeholders = Self::parse_template_placeholders(&result);

        for placeholder in placeholders {
            if let Some(replacement) =
                Self::find_placeholder_value(&placeholder, builder, handler_context)
            {
                let placeholder_str = format!("{{{placeholder}}}");
                result = result.replace(&placeholder_str, &replacement);
            }
        }

        result
    }

    /// Parse template to find placeholder names
    fn parse_template_placeholders(template: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut remaining = template;

        while let Some(start) = remaining.find('{') {
            if let Some(end) = remaining[start + 1..].find('}') {
                let placeholder = &remaining[start + 1..start + 1 + end];
                if !placeholder.is_empty() {
                    placeholders.push(placeholder.to_string());
                }
                remaining = &remaining[start + 1 + end + 1..];
            } else {
                break;
            }
        }

        placeholders
    }

    /// Find value for a placeholder
    fn find_placeholder_value(
        placeholder: &str,
        builder: &ResponseBuilder,
        handler_context: &HandlerContext,
    ) -> Option<String> {
        // First check metadata
        if let Some(Value::Object(metadata)) = builder.metadata() {
            if let Some(value) = metadata.get(placeholder) {
                return Some(Self::value_to_string(value));
            }
        }

        // Then check result if placeholder is "result"
        if placeholder == "result" {
            if let Some(result_value) = builder.result() {
                return Some(Self::value_to_string(result_value));
            }
        }

        // Finally check request parameters
        if let Some(value) = handler_context.extract_optional_named_field(placeholder) {
            return Some(Self::value_to_string(value));
        }

        None
    }

    /// Convert value to string for template substitution
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => format!("{} items", arr.len()),
            _ => value.to_string(),
        }
    }

    /// large response processing - this would possibly be where we would implement pagination
    fn handle_large_response(response: JsonResponse, tool_name: Self) -> Result<CallToolResult> {
        // Check if response is too large and handle result field extraction
        let processed_response =
            handle_large_response(response, tool_name, LargeResponseConfig::default())?;
        Ok(processed_response.to_call_tool_result())
    }

    /// Create handler for this tool
    #[allow(clippy::too_many_lines)]
    pub fn create_handler(self) -> Arc<dyn ErasedUnifiedToolFn> {
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

/// Get all tool definitions for registration with the MCP service
pub fn get_all_tool_definitions() -> Vec<crate::tool::ToolDef> {
    use strum::IntoEnumIterator;

    ToolName::iter().map(ToolName::to_tool_def).collect()
}
