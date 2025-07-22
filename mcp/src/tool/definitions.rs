//! Tool definitions for BRP and local MCP tools.

use super::HandlerFn;
use super::annotations::{BrpToolAnnotations, EnvironmentImpact, ToolCategory};
use super::constants::{
    DESC_BEVY_DESTROY, DESC_BEVY_GET, DESC_BEVY_GET_RESOURCE, DESC_BEVY_GET_WATCH,
    DESC_BEVY_INSERT, DESC_BEVY_INSERT_RESOURCE, DESC_BEVY_LIST, DESC_BEVY_LIST_RESOURCES,
    DESC_BEVY_LIST_WATCH, DESC_BEVY_MUTATE_COMPONENT, DESC_BEVY_MUTATE_RESOURCE, DESC_BEVY_QUERY,
    DESC_BEVY_REGISTRY_SCHEMA, DESC_BEVY_REMOVE, DESC_BEVY_REMOVE_RESOURCE, DESC_BEVY_REPARENT,
    DESC_BEVY_RPC_DISCOVER, DESC_BEVY_SPAWN, DESC_BRP_EXECUTE, DESC_BRP_EXTRAS_DISCOVER_FORMAT,
    DESC_BRP_EXTRAS_SCREENSHOT, DESC_BRP_EXTRAS_SEND_KEYS, DESC_BRP_EXTRAS_SET_DEBUG_MODE,
    DESC_DELETE_LOGS, DESC_GET_TRACE_LOG_PATH, DESC_LAUNCH_BEVY_APP, DESC_LAUNCH_BEVY_EXAMPLE,
    DESC_LIST_ACTIVE_WATCHES, DESC_LIST_BEVY_APPS, DESC_LIST_BEVY_EXAMPLES, DESC_LIST_BRP_APPS,
    DESC_LIST_LOGS, DESC_READ_LOG, DESC_SET_TRACING_LEVEL, DESC_SHUTDOWN, DESC_STATUS,
    DESC_STOP_WATCH, TOOL_BEVY_DESTROY, TOOL_BEVY_GET, TOOL_BEVY_GET_RESOURCE, TOOL_BEVY_GET_WATCH,
    TOOL_BEVY_INSERT, TOOL_BEVY_INSERT_RESOURCE, TOOL_BEVY_LIST, TOOL_BEVY_LIST_RESOURCES,
    TOOL_BEVY_LIST_WATCH, TOOL_BEVY_MUTATE_COMPONENT, TOOL_BEVY_MUTATE_RESOURCE, TOOL_BEVY_QUERY,
    TOOL_BEVY_REGISTRY_SCHEMA, TOOL_BEVY_REMOVE, TOOL_BEVY_REMOVE_RESOURCE, TOOL_BEVY_REPARENT,
    TOOL_BEVY_RPC_DISCOVER, TOOL_BEVY_SPAWN, TOOL_BRP_EXECUTE, TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
    TOOL_BRP_EXTRAS_SCREENSHOT, TOOL_BRP_EXTRAS_SEND_KEYS, TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
    TOOL_DELETE_LOGS, TOOL_GET_TRACE_LOG_PATH, TOOL_LAUNCH_BEVY_APP, TOOL_LAUNCH_BEVY_EXAMPLE,
    TOOL_LIST_ACTIVE_WATCHES, TOOL_LIST_BEVY_APPS, TOOL_LIST_BEVY_EXAMPLES, TOOL_LIST_BRP_APPS,
    TOOL_LIST_LOGS, TOOL_READ_LOG, TOOL_SET_TRACING_LEVEL, TOOL_SHUTDOWN, TOOL_STATUS,
    TOOL_STOP_WATCH,
};
use super::schema_utils::parameters_from_schema;
use super::tool_def::ToolDef;
use crate::app_tools::{
    self, LaunchBevyAppParams, LaunchBevyExampleParams, ListBevyApps, ListBevyExamples,
    ListBrpApps, Shutdown, ShutdownParams, Status, StatusParams,
};
use crate::brp_tools::{
    BevyDestroy, BevyGet, BevyGetResource, BevyGetWatch, BevyInsert, BevyInsertResource, BevyList,
    BevyListResources, BevyListWatch, BevyMutateComponent, BevyMutateResource, BevyQuery,
    BevyRegistrySchema, BevyRemove, BevyRemoveResource, BevyReparent, BevyRpcDiscover, BevySpawn,
    BrpExecute, BrpExtrasDiscoverFormat, BrpExtrasScreenshot, BrpExtrasSendKeys,
    BrpExtrasSetDebugMode, BrpListActiveWatches, BrpStopWatch, DestroyParams, DiscoverFormatParams,
    ExecuteParams, GetParams, GetResourceParams, GetWatchParams, InsertParams,
    InsertResourceParams, ListParams, ListResourcesParams, ListWatchParams, MutateComponentParams,
    MutateResourceParams, QueryParams, RegistrySchemaParams, RemoveParams, RemoveResourceParams,
    ReparentParams, RpcDiscoverParams, ScreenshotParams, SendKeysParams, SetDebugModeParams,
    SpawnParams, StopWatchParams,
};
use crate::log_tools::{
    DeleteLogs, DeleteLogsParams, GetTraceLogPath, ListLogs, ListLogsParams, ReadLog,
    ReadLogParams, SetTracingLevel, SetTracingLevelParams,
};
use crate::response::{FieldPlacement, ResponseField, ResponseFieldName, ResponseSpecification};
use crate::tool::ParameterName;

/// Get all tool definitions for registration with the MCP service
#[allow(clippy::too_many_lines)]
pub fn get_all_tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name:            TOOL_BEVY_DESTROY,
            description:     DESC_BEVY_DESTROY,
            annotations:     BrpToolAnnotations::new(
                "Destroy Bevy Entity",
                ToolCategory::Entity,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyDestroy),
            parameters:      Some(parameters_from_schema::<DestroyParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully destroyed entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name: ResponseFieldName::Entity,
                    parameter_name:      ParameterName::Entity,
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_GET,
            description:     DESC_BEVY_GET,
            annotations:     BrpToolAnnotations::new(
                "Get Component Data",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyGet),
            parameters:      Some(parameters_from_schema::<GetParams>),
            response_format: ResponseSpecification {
                message_template: "Retrieved component data from entity {entity} - component count: {component_count}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entity,
                        parameter_name:      ParameterName::Entity,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Components,
                        source_path:         ResponseFieldName::Result.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ComponentCount,
                        source_path:         "result.components",
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ErrorCount,
                        source_path:         "result.errors",
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_GET_RESOURCE,
            description:     DESC_BEVY_GET_RESOURCE,
            annotations:     BrpToolAnnotations::new(
                "Get Resource Data",
                ToolCategory::Resource,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyGetResource),
            parameters:      Some(parameters_from_schema::<GetResourceParams>),
            response_format: ResponseSpecification {
                message_template: "Retrieved resource: {resource}",
                response_fields:  vec![ResponseField::BrpRawResultToResult],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_INSERT,
            description:     DESC_BEVY_INSERT,
            annotations:     BrpToolAnnotations::new(
                "Insert Components",
                ToolCategory::Component,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyInsert),
            parameters:      Some(parameters_from_schema::<InsertParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully inserted components into entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entity,
                        parameter_name:      ParameterName::Entity,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Components,
                        parameter_name:      ParameterName::Components,
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FormatCorrection,
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_INSERT_RESOURCE,
            description:     DESC_BEVY_INSERT_RESOURCE,
            annotations:     BrpToolAnnotations::new(
                "Insert Resource",
                ToolCategory::Resource,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyInsertResource),
            parameters:      Some(parameters_from_schema::<InsertResourceParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully inserted/updated resource: {resource}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Resource,
                        parameter_name:      ParameterName::Resource,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FormatCorrection,
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_LIST,
            description:     DESC_BEVY_LIST,
            annotations:     BrpToolAnnotations::new(
                "List Components",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyList),
            parameters:      Some(parameters_from_schema::<ListParams>),
            response_format: ResponseSpecification {
                message_template: "Listed {component_count} components",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ComponentCount,
                        source_path:         ResponseFieldName::Result.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_LIST_RESOURCES,
            description:     DESC_BEVY_LIST_RESOURCES,
            annotations:     BrpToolAnnotations::new(
                "List Resources",
                ToolCategory::Resource,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyListResources),
            parameters:      Some(parameters_from_schema::<ListResourcesParams>),
            response_format: ResponseSpecification {
                message_template: "Listed {resource_count} resources",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ResourceCount,
                        source_path:         ResponseFieldName::Result.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_MUTATE_COMPONENT,
            description:     DESC_BEVY_MUTATE_COMPONENT,
            annotations:     BrpToolAnnotations::new(
                "Mutate Component",
                ToolCategory::Component,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyMutateComponent),
            parameters:      Some(parameters_from_schema::<MutateComponentParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully mutated component on entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entity,
                        parameter_name:      ParameterName::Entity,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FormatCorrection,
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_MUTATE_RESOURCE,
            description:     DESC_BEVY_MUTATE_RESOURCE,
            annotations:     BrpToolAnnotations::new(
                "Mutate Resource",
                ToolCategory::Resource,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyMutateResource),
            parameters:      Some(parameters_from_schema::<MutateResourceParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully mutated resource: `{resource}`",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Resource,
                        parameter_name:      ParameterName::Resource,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FormatCorrection,
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_QUERY,
            description:     DESC_BEVY_QUERY,
            annotations:     BrpToolAnnotations::new(
                "Query Entities/Components",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyQuery),
            parameters:      Some(parameters_from_schema::<QueryParams>),
            response_format: ResponseSpecification {
                message_template: "Query completed successfully",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::EntityCount,
                        source_path:         ResponseFieldName::Result.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ComponentCount,
                        source_path:         ResponseFieldName::Result.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_REGISTRY_SCHEMA,
            description:     DESC_BEVY_REGISTRY_SCHEMA,
            annotations:     BrpToolAnnotations::new(
                "Get Type Schemas",
                ToolCategory::Discovery,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyRegistrySchema),
            parameters:      Some(parameters_from_schema::<RegistrySchemaParams>),
            response_format: ResponseSpecification {
                message_template: "Retrieved schema information",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TypeCount,
                        source_path:         ResponseFieldName::Result.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_REMOVE,
            description:     DESC_BEVY_REMOVE,
            annotations:     BrpToolAnnotations::new(
                "Remove Components",
                ToolCategory::Component,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyRemove),
            parameters:      Some(parameters_from_schema::<RemoveParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully removed components from entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entity,
                        parameter_name:      ParameterName::Entity,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Components,
                        parameter_name:      ParameterName::Components,
                        placement:           FieldPlacement::Result,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_REMOVE_RESOURCE,
            description:     DESC_BEVY_REMOVE_RESOURCE,
            annotations:     BrpToolAnnotations::new(
                "Remove Resource",
                ToolCategory::Resource,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:         HandlerFn::brp(BevyRemoveResource),
            parameters:      Some(parameters_from_schema::<RemoveResourceParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully removed resource",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name: ResponseFieldName::Resource,
                    parameter_name:      ParameterName::Resource,
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_REPARENT,
            description:     DESC_BEVY_REPARENT,
            annotations:     BrpToolAnnotations::new(
                "Reparent Entities",
                ToolCategory::Entity,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:         HandlerFn::brp(BevyReparent),
            parameters:      Some(parameters_from_schema::<ReparentParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully reparented entities",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entities,
                        parameter_name:      ParameterName::Entities,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Parent,
                        parameter_name:      ParameterName::Parent,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_RPC_DISCOVER,
            description:     DESC_BEVY_RPC_DISCOVER,
            annotations:     BrpToolAnnotations::new(
                "Discover BRP Methods",
                ToolCategory::Discovery,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BevyRpcDiscover),
            parameters:      Some(parameters_from_schema::<RpcDiscoverParams>),
            response_format: ResponseSpecification {
                message_template: "Retrieved BRP method discovery information for {method_count} methods",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::MethodCount,
                        source_path:         "result.methods",
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // todo: (later) make this match curl
        ToolDef {
            name:            TOOL_BEVY_SPAWN,
            description:     DESC_BEVY_SPAWN,
            annotations:     BrpToolAnnotations::new(
                "Spawn Entity",
                ToolCategory::Entity,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:         HandlerFn::brp(BevySpawn),
            parameters:      Some(parameters_from_schema::<SpawnParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully spawned entity",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Entity,
                        source_path:         "result.entity",
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FormatCorrection,
                ],
            },
        },
        // brp_execute is a LocalToolFnWithPort since it uses user-provided method names
        // rather than static method names from ToolDef constants
        ToolDef {
            name:            TOOL_BRP_EXECUTE,
            description:     DESC_BRP_EXECUTE,
            annotations:     BrpToolAnnotations::new(
                "Execute BRP Method",
                ToolCategory::DynamicBrp,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:         HandlerFn::local_with_port(BrpExecute),
            parameters:      Some(parameters_from_schema::<ExecuteParams>),
            response_format: ResponseSpecification {
                message_template: "Method executed successfully",
                response_fields:  vec![ResponseField::BrpRawResultToResult],
            },
        },
        ToolDef {
            name:            TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
            description:     DESC_BRP_EXTRAS_DISCOVER_FORMAT,
            annotations:     BrpToolAnnotations::new(
                "Discover Component Format",
                ToolCategory::Extras,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BrpExtrasDiscoverFormat),
            parameters:      Some(parameters_from_schema::<DiscoverFormatParams>),
            response_format: ResponseSpecification {
                message_template: "Format discovery completed",
                response_fields:  vec![ResponseField::BrpRawResultToResult],
            },
        },
        ToolDef {
            name:            TOOL_BRP_EXTRAS_SCREENSHOT,
            description:     DESC_BRP_EXTRAS_SCREENSHOT,
            annotations:     BrpToolAnnotations::new(
                "Take Screenshot",
                ToolCategory::Extras,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:         HandlerFn::brp(BrpExtrasScreenshot),
            parameters:      Some(parameters_from_schema::<ScreenshotParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully captured screenshot",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name: ResponseFieldName::Path,
                    parameter_name:      ParameterName::Path,
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:            TOOL_BRP_EXTRAS_SEND_KEYS,
            description:     DESC_BRP_EXTRAS_SEND_KEYS,
            annotations:     BrpToolAnnotations::new(
                "Send Keys",
                ToolCategory::Extras,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:         HandlerFn::brp(BrpExtrasSendKeys),
            parameters:      Some(parameters_from_schema::<SendKeysParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully sent keyboard input",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::KeysSent,
                        source_path:         "result.keys_sent",
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DurationMs,
                        source_path:         "result.duration_ms",
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
            description:     DESC_BRP_EXTRAS_SET_DEBUG_MODE,
            annotations:     BrpToolAnnotations::new(
                "Set Debug Mode",
                ToolCategory::Extras,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::brp(BrpExtrasSetDebugMode),
            parameters:      Some(parameters_from_schema::<SetDebugModeParams>),
            response_format: ResponseSpecification {
                message_template: "Debug mode updated successfully",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DebugEnabled,
                        source_path:         "result.debug_enabled",
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Details,
                        source_path:         "result.message",
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // BevyGetWatch and BevyListWatch are unusual in that
        // ultimately we do call bevy/get+watch and bevy/list+watch
        // but we need the local tool in order to set up the watch to stream
        // the results and log them to a file
        ToolDef {
            name:            TOOL_BEVY_GET_WATCH,
            description:     DESC_BEVY_GET_WATCH,
            annotations:     BrpToolAnnotations::new(
                "Watch Component Changes",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:         HandlerFn::local_with_port(BevyGetWatch),
            parameters:      Some(parameters_from_schema::<GetWatchParams>),
            response_format: ResponseSpecification {
                message_template: "Started entity watch {watch_id} for entity {entity}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::WatchId,
                        source_path:         ResponseFieldName::WatchId.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogPath,
                        source_path:         ResponseFieldName::LogPath.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entity,
                        parameter_name:      ParameterName::Entity,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_BEVY_LIST_WATCH,
            description:     DESC_BEVY_LIST_WATCH,
            annotations:     BrpToolAnnotations::new(
                "Watch Component List",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:         HandlerFn::local_with_port(BevyListWatch),
            parameters:      Some(parameters_from_schema::<ListWatchParams>),
            response_format: ResponseSpecification {
                message_template: "Started list watch {watch_id} for entity {entity}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::WatchId,
                        source_path:         ResponseFieldName::WatchId.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogPath,
                        source_path:         ResponseFieldName::LogPath.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::Entity,
                        parameter_name:      ParameterName::Entity,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_DELETE_LOGS,
            description:     DESC_DELETE_LOGS,
            annotations:     BrpToolAnnotations::new(
                "Delete Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:         HandlerFn::local(DeleteLogs),
            parameters:      Some(parameters_from_schema::<DeleteLogsParams>),
            response_format: ResponseSpecification {
                message_template: "Deleted {deleted_count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DeletedCount,
                        source_path:         ResponseFieldName::DeletedCount.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DeletedFiles,
                        source_path:         ResponseFieldName::DeletedFiles.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::AppNameFilter,
                        source_path:         ResponseFieldName::AppNameFilter.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::OlderThanSeconds,
                        source_path:         ResponseFieldName::OlderThanSeconds.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_GET_TRACE_LOG_PATH,
            description:     DESC_GET_TRACE_LOG_PATH,
            annotations:     BrpToolAnnotations::new(
                "Get Trace Log Path",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(GetTraceLogPath),
            parameters:      None,
            response_format: ResponseSpecification {
                message_template: "Trace log found",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogPath,
                        source_path:         ResponseFieldName::LogPath.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Exists,
                        source_path:         ResponseFieldName::Exists.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::FileSizeBytes,
                        source_path:         ResponseFieldName::FileSizeBytes.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_LAUNCH_BEVY_APP,
            description:     DESC_LAUNCH_BEVY_APP,
            annotations:     BrpToolAnnotations::new(
                "Launch Bevy App",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local_with_port(app_tools::create_launch_bevy_app_handler()),
            parameters:      Some(parameters_from_schema::<LaunchBevyAppParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully launched bevy app '{target_name}' (PID: {pid})",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Pid,
                        source_path:         ResponseFieldName::Pid.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::DirectToMetadata,
                ],
            },
        },
        ToolDef {
            name:            TOOL_LAUNCH_BEVY_EXAMPLE,
            description:     DESC_LAUNCH_BEVY_EXAMPLE,
            annotations:     BrpToolAnnotations::new(
                "Launch Bevy Example",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local_with_port(
                app_tools::create_launch_bevy_example_handler(),
            ),
            parameters:      Some(parameters_from_schema::<LaunchBevyExampleParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully launched example '{target_name}' (PID: {pid})",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Pid,
                        source_path:         ResponseFieldName::Pid.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::DirectToMetadata,
                ],
            },
        },
        ToolDef {
            name:            TOOL_LIST_BEVY_APPS,
            description:     DESC_LIST_BEVY_APPS,
            annotations:     BrpToolAnnotations::new(
                "List Bevy Apps",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(ListBevyApps),
            parameters:      None,
            response_format: ResponseSpecification {
                message_template: "Found {count} Bevy apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Apps,
                        source_path:         ResponseFieldName::Apps.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        source_path:         ResponseFieldName::Apps.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_LIST_BEVY_EXAMPLES,
            description:     DESC_LIST_BEVY_EXAMPLES,
            annotations:     BrpToolAnnotations::new(
                "List Bevy Examples",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(ListBevyExamples),
            parameters:      None,
            response_format: ResponseSpecification {
                message_template: "Found {count} Bevy examples",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Examples,
                        source_path:         ResponseFieldName::Examples.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        source_path:         ResponseFieldName::Examples.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_LIST_BRP_APPS,
            description:     DESC_LIST_BRP_APPS,
            annotations:     BrpToolAnnotations::new(
                "List Bevy BRP-enabled Apps",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(ListBrpApps),
            parameters:      None,
            response_format: ResponseSpecification {
                message_template: "Found {count} BRP-enabled apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Apps,
                        source_path:         ResponseFieldName::Apps.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        source_path:         ResponseFieldName::Apps.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_LIST_ACTIVE_WATCHES,
            description:     DESC_LIST_ACTIVE_WATCHES,
            annotations:     BrpToolAnnotations::new(
                "List Active Watches",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(BrpListActiveWatches),
            parameters:      None,
            response_format: ResponseSpecification {
                message_template: "Found {count} active watches",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Watches,
                        source_path:         ResponseFieldName::Watches.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        source_path:         ResponseFieldName::Watches.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_STOP_WATCH,
            description:     DESC_STOP_WATCH,
            annotations:     BrpToolAnnotations::new(
                "Stop Watch",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:         HandlerFn::local(BrpStopWatch),
            parameters:      Some(parameters_from_schema::<StopWatchParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully stopped watch",
                response_fields:  vec![ResponseField::FromResponse {
                    response_field_name: ResponseFieldName::WatchId,
                    source_path:         ResponseFieldName::WatchId.into(),
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:            TOOL_LIST_LOGS,
            description:     DESC_LIST_LOGS,
            annotations:     BrpToolAnnotations::new(
                "List Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(ListLogs),
            parameters:      Some(parameters_from_schema::<ListLogsParams>),
            response_format: ResponseSpecification {
                message_template: "Found {count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Logs,
                        source_path:         ResponseFieldName::Logs.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TempDirectory,
                        source_path:         ResponseFieldName::TempDirectory.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        source_path:         ResponseFieldName::Logs.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_READ_LOG,
            description:     DESC_READ_LOG,
            annotations:     BrpToolAnnotations::new(
                "Read Log File",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local(ReadLog),
            parameters:      Some(parameters_from_schema::<ReadLogParams>),
            response_format: ResponseSpecification {
                message_template: "Successfully read log file: {filename}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Filename,
                        source_path:         ResponseFieldName::Filename.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::FilePath,
                        source_path:         ResponseFieldName::FilePath.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::SizeBytes,
                        source_path:         ResponseFieldName::SizeBytes.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::SizeHuman,
                        source_path:         ResponseFieldName::SizeHuman.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LinesRead,
                        source_path:         ResponseFieldName::LinesRead.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Content,
                        source_path:         ResponseFieldName::Content.into(),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::FilteredByKeyword,
                        source_path:         ResponseFieldName::FilteredByKeyword.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TailMode,
                        source_path:         ResponseFieldName::TailMode.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_SET_TRACING_LEVEL,
            description:     DESC_SET_TRACING_LEVEL,
            annotations:     BrpToolAnnotations::new(
                "Set Tracing Level",
                ToolCategory::Logging,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:         HandlerFn::local(SetTracingLevel),
            parameters:      Some(parameters_from_schema::<SetTracingLevelParams>),
            response_format: ResponseSpecification {
                message_template: "Tracing level set to '{tracing_level}' - diagnostic information will be logged to temp directory",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TracingLevel,
                        source_path:         ResponseFieldName::TracingLevel.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TracingLogFile,
                        source_path:         ResponseFieldName::TracingLogFile.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_STATUS,
            description:     DESC_STATUS,
            annotations:     BrpToolAnnotations::new(
                "Check App Status",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:         HandlerFn::local_with_port(Status),
            parameters:      Some(parameters_from_schema::<StatusParams>),
            response_format: ResponseSpecification {
                message_template: "Process '{app_name}' (PID: {pid}) is running with BRP enabled on port {port}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::AppName,
                        parameter_name:      ParameterName::AppName,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::AppRunning,
                        source_path:         ResponseFieldName::AppRunning.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::BrpResponsive,
                        source_path:         ResponseFieldName::BrpResponsive.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Pid,
                        source_path:         ResponseFieldName::Pid.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        ToolDef {
            name:            TOOL_SHUTDOWN,
            description:     DESC_SHUTDOWN,
            annotations:     BrpToolAnnotations::new(
                "Shutdown Bevy App",
                ToolCategory::App,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:         HandlerFn::local_with_port(Shutdown),
            parameters:      Some(parameters_from_schema::<ShutdownParams>),
            response_format: ResponseSpecification {
                message_template: "{message}",
                response_fields:  vec![
                    // ResponseField::FromResponse {
                    //     response_field_name: ResponseFieldName::ShutdownMethod,
                    //     source_path: "shutdown_method",
                    //     placement:           FieldPlacement::Metadata,
                    // },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::AppName,
                        source_path:         ResponseFieldName::AppName.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ShutdownMethod,
                        source_path:         ResponseFieldName::ShutdownMethod.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponseNullableWithPlacement {
                        response_field_name: ResponseFieldName::Pid,
                        source_path:         ResponseFieldName::Pid.into(),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
    ]
}
