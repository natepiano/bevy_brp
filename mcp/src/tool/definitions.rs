//! Tool definitions for BRP and local MCP tools.

use std::sync::Arc;

use super::annotations::{BrpToolAnnotations, EnvironmentImpact, ToolCategory};
use super::constants::ToolName;
use super::parameters::extract_parameters;
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
use crate::response::{FieldPlacement, ResponseDef, ResponseField, ResponseFieldName};
use crate::tool::ParameterName;

/// Get all tool definitions for registration with the MCP service
#[allow(clippy::too_many_lines)]
pub fn get_all_tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name:        ToolName::BevyDestroy.as_ref(),
            description: ToolName::BevyDestroy.description(),
            annotations: BrpToolAnnotations::new(
                "Destroy Bevy Entity",
                ToolCategory::Entity,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:     Arc::new(BevyDestroy),
            parameters:  Some(extract_parameters::<DestroyParams>),
            response:    ResponseDef {
                message_template: "Successfully destroyed entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name: ResponseFieldName::Entity,
                    parameter_name:      ParameterName::Entity,
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:        ToolName::BevyGet.as_ref(),
            description: ToolName::BevyGet.description(),
            annotations: BrpToolAnnotations::new(
                "Get Component Data",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyGet),
            parameters:  Some(extract_parameters::<GetParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyGetResource.as_ref(),
            description: ToolName::BevyGetResource.description(),
            annotations: BrpToolAnnotations::new(
                "Get Resource Data",
                ToolCategory::Resource,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyGetResource),
            parameters:  Some(extract_parameters::<GetResourceParams>),
            response:    ResponseDef {
                message_template: "Retrieved resource: {resource}",
                response_fields:  vec![ResponseField::BrpRawResultToResult],
            },
        },
        ToolDef {
            name:        ToolName::BevyInsert.as_ref(),
            description: ToolName::BevyInsert.description(),
            annotations: BrpToolAnnotations::new(
                "Insert Components",
                ToolCategory::Component,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:     Arc::new(BevyInsert),
            parameters:  Some(extract_parameters::<InsertParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyInsertResource.as_ref(),
            description: ToolName::BevyInsertResource.description(),
            annotations: BrpToolAnnotations::new(
                "Insert Resource",
                ToolCategory::Resource,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:     Arc::new(BevyInsertResource),
            parameters:  Some(extract_parameters::<InsertResourceParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyList.as_ref(),
            description: ToolName::BevyList.description(),
            annotations: BrpToolAnnotations::new(
                "List Components",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyList),
            parameters:  Some(extract_parameters::<ListParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyListResources.as_ref(),
            description: ToolName::BevyListResources.description(),
            annotations: BrpToolAnnotations::new(
                "List Resources",
                ToolCategory::Resource,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyListResources),
            parameters:  Some(extract_parameters::<ListResourcesParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyMutateComponent.as_ref(),
            description: ToolName::BevyMutateComponent.description(),
            annotations: BrpToolAnnotations::new(
                "Mutate Component",
                ToolCategory::Component,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:     Arc::new(BevyMutateComponent),
            parameters:  Some(extract_parameters::<MutateComponentParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyMutateResource.as_ref(),
            description: ToolName::BevyMutateResource.description(),
            annotations: BrpToolAnnotations::new(
                "Mutate Resource",
                ToolCategory::Resource,
                EnvironmentImpact::AdditiveIdempotent,
            ),
            handler:     Arc::new(BevyMutateResource),
            parameters:  Some(extract_parameters::<MutateResourceParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyQuery.as_ref(),
            description: ToolName::BevyQuery.description(),
            annotations: BrpToolAnnotations::new(
                "Query Entities/Components",
                ToolCategory::Component,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyQuery),
            parameters:  Some(extract_parameters::<QueryParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyRegistrySchema.as_ref(),
            description: ToolName::BevyRegistrySchema.description(),
            annotations: BrpToolAnnotations::new(
                "Get Type Schemas",
                ToolCategory::Discovery,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyRegistrySchema),
            parameters:  Some(extract_parameters::<RegistrySchemaParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyRemove.as_ref(),
            description: ToolName::BevyRemove.description(),
            annotations: BrpToolAnnotations::new(
                "Remove Components",
                ToolCategory::Component,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:     Arc::new(BevyRemove),
            parameters:  Some(extract_parameters::<RemoveParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyRemoveResource.as_ref(),
            description: ToolName::BevyRemoveResource.description(),
            annotations: BrpToolAnnotations::new(
                "Remove Resource",
                ToolCategory::Resource,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:     Arc::new(BevyRemoveResource),
            parameters:  Some(extract_parameters::<RemoveResourceParams>),
            response:    ResponseDef {
                message_template: "Successfully removed resource",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name: ResponseFieldName::Resource,
                    parameter_name:      ParameterName::Resource,
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:        ToolName::BevyReparent.as_ref(),
            description: ToolName::BevyReparent.description(),
            annotations: BrpToolAnnotations::new(
                "Reparent Entities",
                ToolCategory::Entity,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:     Arc::new(BevyReparent),
            parameters:  Some(extract_parameters::<ReparentParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyRpcDiscover.as_ref(),
            description: ToolName::BevyRpcDiscover.description(),
            annotations: BrpToolAnnotations::new(
                "Discover BRP Methods",
                ToolCategory::Discovery,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BevyRpcDiscover),
            parameters:  Some(extract_parameters::<RpcDiscoverParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevySpawn.as_ref(),
            description: ToolName::BevySpawn.description(),
            annotations: BrpToolAnnotations::new(
                "Spawn Entity",
                ToolCategory::Entity,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:     Arc::new(BevySpawn),
            parameters:  Some(extract_parameters::<SpawnParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpExecute.as_ref(),
            description: ToolName::BrpExecute.description(),
            annotations: BrpToolAnnotations::new(
                "Execute BRP Method",
                ToolCategory::DynamicBrp,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:     Arc::new(BrpExecute),
            parameters:  Some(extract_parameters::<ExecuteParams>),
            response:    ResponseDef {
                message_template: "Method executed successfully",
                response_fields:  vec![ResponseField::BrpRawResultToResult],
            },
        },
        ToolDef {
            name:        ToolName::BrpExtrasDiscoverFormat.as_ref(),
            description: ToolName::BrpExtrasDiscoverFormat.description(),
            annotations: BrpToolAnnotations::new(
                "Discover Component Format",
                ToolCategory::Extras,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BrpExtrasDiscoverFormat),
            parameters:  Some(extract_parameters::<DiscoverFormatParams>),
            response:    ResponseDef {
                message_template: "Format discovery completed",
                response_fields:  vec![ResponseField::BrpRawResultToResult],
            },
        },
        ToolDef {
            name:        ToolName::BrpExtrasScreenshot.as_ref(),
            description: ToolName::BrpExtrasScreenshot.description(),
            annotations: BrpToolAnnotations::new(
                "Take Screenshot",
                ToolCategory::Extras,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:     Arc::new(BrpExtrasScreenshot),
            parameters:  Some(extract_parameters::<ScreenshotParams>),
            response:    ResponseDef {
                message_template: "Successfully captured screenshot",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name: ResponseFieldName::Path,
                    parameter_name:      ParameterName::Path,
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:        ToolName::BrpExtrasSendKeys.as_ref(),
            description: ToolName::BrpExtrasSendKeys.description(),
            annotations: BrpToolAnnotations::new(
                "Send Keys",
                ToolCategory::Extras,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:     Arc::new(BrpExtrasSendKeys),
            parameters:  Some(extract_parameters::<SendKeysParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpExtrasSetDebugMode.as_ref(),
            description: ToolName::BrpExtrasSetDebugMode.description(),
            annotations: BrpToolAnnotations::new(
                "Set Debug Mode",
                ToolCategory::Extras,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BrpExtrasSetDebugMode),
            parameters:  Some(extract_parameters::<SetDebugModeParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyGetWatch.as_ref(),
            description: ToolName::BevyGetWatch.description(),
            annotations: BrpToolAnnotations::new(
                "Watch Component Changes",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:     Arc::new(BevyGetWatch),
            parameters:  Some(extract_parameters::<GetWatchParams>),
            response:    ResponseDef {
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
            name:        ToolName::BevyListWatch.as_ref(),
            description: ToolName::BevyListWatch.description(),
            annotations: BrpToolAnnotations::new(
                "Watch Component List",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::AdditiveNonIdempotent,
            ),
            handler:     Arc::new(BevyListWatch),
            parameters:  Some(extract_parameters::<ListWatchParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpDeleteLogs.as_ref(),
            description: ToolName::BrpDeleteLogs.description(),
            annotations: BrpToolAnnotations::new(
                "Delete Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:     Arc::new(DeleteLogs),
            parameters:  Some(extract_parameters::<DeleteLogsParams>),
            response:    ResponseDef {
                message_template: "Deleted {deleted_count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DeletedCount,
                        source_path:         ResponseFieldName::DeletedFiles.into(),
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
            name:        ToolName::BrpGetTraceLogPath.as_ref(),
            description: ToolName::BrpGetTraceLogPath.description(),
            annotations: BrpToolAnnotations::new(
                "Get Trace Log Path",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(GetTraceLogPath),
            parameters:  None,
            response:    ResponseDef {
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
            name:        ToolName::BrpLaunchBevyApp.as_ref(),
            description: ToolName::BrpLaunchBevyApp.description(),
            annotations: BrpToolAnnotations::new(
                "Launch Bevy App",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(app_tools::create_launch_bevy_app_handler()),
            parameters:  Some(extract_parameters::<LaunchBevyAppParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpLaunchBevyExample.as_ref(),
            description: ToolName::BrpLaunchBevyExample.description(),
            annotations: BrpToolAnnotations::new(
                "Launch Bevy Example",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(app_tools::create_launch_bevy_example_handler()),
            parameters:  Some(extract_parameters::<LaunchBevyExampleParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpListBevyApps.as_ref(),
            description: ToolName::BrpListBevyApps.description(),
            annotations: BrpToolAnnotations::new(
                "List Bevy Apps",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(ListBevyApps),
            parameters:  None,
            response:    ResponseDef {
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
            name:        ToolName::BrpListBevyExamples.as_ref(),
            description: ToolName::BrpListBevyExamples.description(),
            annotations: BrpToolAnnotations::new(
                "List Bevy Examples",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(ListBevyExamples),
            parameters:  None,
            response:    ResponseDef {
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
            name:        ToolName::BrpListBrpApps.as_ref(),
            description: ToolName::BrpListBrpApps.description(),
            annotations: BrpToolAnnotations::new(
                "List Bevy BRP-enabled Apps",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(ListBrpApps),
            parameters:  None,
            response:    ResponseDef {
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
            name:        ToolName::BrpListActiveWatches.as_ref(),
            description: ToolName::BrpListActiveWatches.description(),
            annotations: BrpToolAnnotations::new(
                "List Active Watches",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(BrpListActiveWatches),
            parameters:  None,
            response:    ResponseDef {
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
            name:        ToolName::BrpStopWatch.as_ref(),
            description: ToolName::BrpStopWatch.description(),
            annotations: BrpToolAnnotations::new(
                "Stop Watch",
                ToolCategory::WatchMonitoring,
                EnvironmentImpact::DestructiveIdempotent,
            ),
            handler:     Arc::new(BrpStopWatch),
            parameters:  Some(extract_parameters::<StopWatchParams>),
            response:    ResponseDef {
                message_template: "Successfully stopped watch",
                response_fields:  vec![ResponseField::FromResponse {
                    response_field_name: ResponseFieldName::WatchId,
                    source_path:         ResponseFieldName::WatchId.into(),
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        ToolDef {
            name:        ToolName::BrpListLogs.as_ref(),
            description: ToolName::BrpListLogs.description(),
            annotations: BrpToolAnnotations::new(
                "List Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(ListLogs),
            parameters:  Some(extract_parameters::<ListLogsParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpReadLog.as_ref(),
            description: ToolName::BrpReadLog.description(),
            annotations: BrpToolAnnotations::new(
                "Read Log File",
                ToolCategory::Logging,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(ReadLog),
            parameters:  Some(extract_parameters::<ReadLogParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpSetTracingLevel.as_ref(),
            description: ToolName::BrpSetTracingLevel.description(),
            annotations: BrpToolAnnotations::new(
                "Set Tracing Level",
                ToolCategory::Logging,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:     Arc::new(SetTracingLevel),
            parameters:  Some(extract_parameters::<SetTracingLevelParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpStatus.as_ref(),
            description: ToolName::BrpStatus.description(),
            annotations: BrpToolAnnotations::new(
                "Check App Status",
                ToolCategory::App,
                EnvironmentImpact::ReadOnly,
            ),
            handler:     Arc::new(Status),
            parameters:  Some(extract_parameters::<StatusParams>),
            response:    ResponseDef {
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
            name:        ToolName::BrpShutdown.as_ref(),
            description: ToolName::BrpShutdown.description(),
            annotations: BrpToolAnnotations::new(
                "Shutdown Bevy App",
                ToolCategory::App,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:     Arc::new(Shutdown),
            parameters:  Some(extract_parameters::<ShutdownParams>),
            response:    ResponseDef {
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
