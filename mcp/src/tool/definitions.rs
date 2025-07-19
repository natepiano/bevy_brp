//! Tool definitions for BRP and local MCP tools.

use super::HandlerFn;
use super::annotations::{BrpToolAnnotations, EnvironmentImpact, ToolCategory};
use super::constants::{
    BRP_METHOD_DESTROY, BRP_METHOD_EXTRAS_DISCOVER_FORMAT, BRP_METHOD_EXTRAS_SCREENSHOT,
    BRP_METHOD_EXTRAS_SEND_KEYS, BRP_METHOD_EXTRAS_SET_DEBUG_MODE, BRP_METHOD_GET,
    BRP_METHOD_GET_RESOURCE, BRP_METHOD_INSERT, BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST,
    BRP_METHOD_LIST_RESOURCES, BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE,
    BRP_METHOD_QUERY, BRP_METHOD_REGISTRY_SCHEMA, BRP_METHOD_REMOVE, BRP_METHOD_REMOVE_RESOURCE,
    BRP_METHOD_REPARENT, BRP_METHOD_RPC_DISCOVER, BRP_METHOD_SPAWN, DESC_BEVY_DESTROY,
    DESC_BEVY_GET, DESC_BEVY_GET_RESOURCE, DESC_BEVY_GET_WATCH, DESC_BEVY_INSERT,
    DESC_BEVY_INSERT_RESOURCE, DESC_BEVY_LIST, DESC_BEVY_LIST_RESOURCES, DESC_BEVY_LIST_WATCH,
    DESC_BEVY_MUTATE_COMPONENT, DESC_BEVY_MUTATE_RESOURCE, DESC_BEVY_QUERY,
    DESC_BEVY_REGISTRY_SCHEMA, DESC_BEVY_REMOVE, DESC_BEVY_REMOVE_RESOURCE, DESC_BEVY_REPARENT,
    DESC_BEVY_RPC_DISCOVER, DESC_BEVY_SPAWN, DESC_BRP_EXECUTE, DESC_BRP_EXTRAS_DISCOVER_FORMAT,
    DESC_BRP_EXTRAS_SCREENSHOT, DESC_BRP_EXTRAS_SEND_KEYS, DESC_BRP_EXTRAS_SET_DEBUG_MODE,
    DESC_CLEANUP_LOGS, DESC_GET_TRACE_LOG_PATH, DESC_LAUNCH_BEVY_APP, DESC_LAUNCH_BEVY_EXAMPLE,
    DESC_LIST_BEVY_APPS, DESC_LIST_BEVY_EXAMPLES, DESC_LIST_BRP_APPS, DESC_LIST_LOGS,
    DESC_READ_LOG, DESC_SET_TRACING_LEVEL, DESC_SHUTDOWN, DESC_STATUS, TOOL_BEVY_DESTROY,
    TOOL_BEVY_GET, TOOL_BEVY_GET_RESOURCE, TOOL_BEVY_GET_WATCH, TOOL_BEVY_INSERT,
    TOOL_BEVY_INSERT_RESOURCE, TOOL_BEVY_LIST, TOOL_BEVY_LIST_RESOURCES, TOOL_BEVY_LIST_WATCH,
    TOOL_BEVY_MUTATE_COMPONENT, TOOL_BEVY_MUTATE_RESOURCE, TOOL_BEVY_QUERY,
    TOOL_BEVY_REGISTRY_SCHEMA, TOOL_BEVY_REMOVE, TOOL_BEVY_REMOVE_RESOURCE, TOOL_BEVY_REPARENT,
    TOOL_BEVY_RPC_DISCOVER, TOOL_BEVY_SPAWN, TOOL_BRP_EXECUTE, TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
    TOOL_BRP_EXTRAS_SCREENSHOT, TOOL_BRP_EXTRAS_SEND_KEYS, TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
    TOOL_CLEANUP_LOGS, TOOL_GET_TRACE_LOG_PATH, TOOL_LAUNCH_BEVY_APP, TOOL_LAUNCH_BEVY_EXAMPLE,
    TOOL_LIST_BEVY_APPS, TOOL_LIST_BEVY_EXAMPLES, TOOL_LIST_BRP_APPS, TOOL_LIST_LOGS,
    TOOL_READ_LOG, TOOL_SET_TRACING_LEVEL, TOOL_SHUTDOWN, TOOL_STATUS,
};
use super::parameters::{Parameter, ParameterName};
use super::tool_def::ToolDef;
use crate::app_tools::brp_launch_bevy_example;
use crate::app_tools::brp_list_bevy_apps::ListBevyApps;
use crate::app_tools::brp_list_bevy_examples::ListBevyExamples;
use crate::app_tools::brp_list_brp_apps::ListBrpApps;
use crate::app_tools::brp_shutdown::Shutdown;
use crate::app_tools::brp_status::Status;
use crate::brp_tools::request_handler::BrpMethodHandlerV2;
use crate::brp_tools::watch::bevy_get_watch::BevyGetWatch;
use crate::brp_tools::watch::bevy_list_watch::BevyListWatch;
use crate::brp_tools::watch::brp_list_active::BrpListActiveWatches;
use crate::brp_tools::watch::brp_stop_watch::BrpStopWatch;
use crate::log_tools::cleanup_logs::CleanupLogs;
use crate::log_tools::get_trace_log_path::GetTraceLogPath;
use crate::log_tools::list_logs::ListLogs;
use crate::log_tools::read_log::ReadLog;
use crate::log_tools::set_tracing_level::SetTracingLevel;
use crate::response::{
    FieldPlacement, ResponseExtractorType, ResponseField, ResponseFieldName, ResponseSpecification,
};
use crate::tool::constants::{
    DESC_LIST_ACTIVE_WATCHES, DESC_STOP_WATCH, TOOL_LIST_ACTIVE_WATCHES, TOOL_STOP_WATCH,
};

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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_DESTROY),
            parameters:      vec![Parameter::entity("The entity ID to destroy", true)],
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_GET),
            parameters:      vec![
                Parameter::entity("The entity ID to get component data from", true),
                Parameter::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                ),
            ],
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
                        response_extractor:  ResponseExtractorType::Field("result"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ComponentCount,
                        response_extractor:  ResponseExtractorType::KeyCount("result.components"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ErrorCount,
                        response_extractor:  ResponseExtractorType::KeyCount("result.errors"),
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_GET_RESOURCE),
            parameters:      vec![Parameter::resource(
                "The fully-qualified type name of the resource to get",
            )],
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_INSERT),
            parameters:      vec![
                Parameter::entity("The entity ID to insert components into", true),
                Parameter::components(
                    "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_INSERT_RESOURCE,
            ),
            parameters:      vec![
                Parameter::resource(
                    "The fully-qualified type name of the resource to insert or update",
                ),
                Parameter::value(
                    "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_LIST),
            parameters:      vec![Parameter::entity(
                "Optional entity ID to list components for",
                false,
            )],
            response_format: ResponseSpecification {
                message_template: "Listed {component_count} components",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ComponentCount,
                        response_extractor:  ResponseExtractorType::ArrayCount("result"),
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_LIST_RESOURCES,
            ),
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Listed {resource_count} resources",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ResourceCount,
                        response_extractor:  ResponseExtractorType::ArrayCount("result"),
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_MUTATE_COMPONENT,
            ),
            parameters:      vec![
                Parameter::entity("The entity ID containing the component to mutate", true),
                Parameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                Parameter::component("The fully-qualified type name of the component to mutate"),
                Parameter::path(
                    "The path to the field within the component (e.g., 'translation.x')",
                ),
            ],
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_MUTATE_RESOURCE,
            ),
            parameters:      vec![
                Parameter::resource("The fully-qualified type name of the resource to mutate"),
                Parameter::path(
                    "The path to the field within the resource (e.g., 'settings.volume')",
                ),
                Parameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_QUERY),
            parameters:      vec![Parameter::data(), Parameter::filter(), Parameter::strict()],
            response_format: ResponseSpecification {
                message_template: "Query completed successfully",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::EntityCount,
                        response_extractor:  ResponseExtractorType::ArrayCount("result"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ComponentCount,
                        response_extractor:  ResponseExtractorType::QueryComponentCount("result"),
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_REGISTRY_SCHEMA,
            ),
            parameters:      vec![
                Parameter::with_crates(),
                Parameter::without_crates(),
                Parameter::with_types(),
                Parameter::without_types(),
            ],
            response_format: ResponseSpecification {
                message_template: "Retrieved schema information",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TypeCount,
                        response_extractor:  ResponseExtractorType::KeyCount("result"),
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_REMOVE),
            parameters:      vec![
                Parameter::entity("The entity ID to remove components from", true),
                Parameter::components("Array of component type names to remove", true),
            ],
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_REMOVE_RESOURCE,
            ),
            parameters:      vec![Parameter::resource(
                "The fully-qualified type name of the resource to remove",
            )],
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_REPARENT),
            parameters:      vec![
                Parameter::entities("Array of entity IDs to reparent"),
                Parameter::parent(),
            ],
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_RPC_DISCOVER),
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Retrieved BRP method discovery information for {method_count} methods",
                response_fields:  vec![
                    ResponseField::BrpRawResultToResult,
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::MethodCount,
                        response_extractor:  ResponseExtractorType::ArrayCount("result.methods"),
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
            handler:         HandlerFn::brp_v2_static(BrpMethodHandlerV2, BRP_METHOD_SPAWN),
            parameters:      vec![Parameter::components(
                "Object containing component data to spawn with. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                false,
            )],
            response_format: ResponseSpecification {
                message_template: "Successfully spawned entity",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Entity,
                        response_extractor:  ResponseExtractorType::Field("result.entity"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FormatCorrection,
                ],
            },
        },
        // this is the one brp tool that uses dynamic method handling
        // as the user can dynamically pass in the method
        ToolDef {
            name:            TOOL_BRP_EXECUTE,
            description:     DESC_BRP_EXECUTE,
            annotations:     BrpToolAnnotations::new(
                "Execute BRP Method",
                ToolCategory::DynamicBrp,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:         HandlerFn::brp_v2_dynamic(BrpMethodHandlerV2),
            parameters:      vec![Parameter::dynamic_params(
                "Optional parameters for the method, as a JSON object or array",
                false,
            )],
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
            ),
            parameters:      vec![Parameter::types(
                "Array of fully-qualified component type names to discover formats for",
                true,
            )],
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_EXTRAS_SCREENSHOT,
            ),
            parameters:      vec![Parameter::path(
                "File path where the screenshot should be saved",
            )],
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_EXTRAS_SEND_KEYS,
            ),
            parameters:      vec![Parameter::keys(), Parameter::duration_ms()],
            response_format: ResponseSpecification {
                message_template: "Successfully sent keyboard input",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::KeysSent,
                        response_extractor:  ResponseExtractorType::Field("result.keys_sent"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DurationMs,
                        response_extractor:  ResponseExtractorType::Field("result.duration_ms"),
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
            handler:         HandlerFn::brp_v2_static(
                BrpMethodHandlerV2,
                BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
            ),
            parameters:      vec![Parameter::enabled()],
            response_format: ResponseSpecification {
                message_template: "Debug mode updated successfully",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DebugEnabled,
                        response_extractor:  ResponseExtractorType::Field("result.debug_enabled"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Details,
                        response_extractor:  ResponseExtractorType::Field("result.message"),
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
            parameters:      vec![
                Parameter::number(
                    ParameterName::Entity,
                    "The entity ID to watch for component changes",
                    true,
                ),
                Parameter::types(
                    "Required array of component types to watch. Must contain at least one component. Without this, the watch will not detect any changes.",
                    true,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Started entity watch for entity {entity}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::WatchId,
                        response_extractor:  ResponseExtractorType::Field("watch_id"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogPath,
                        response_extractor:  ResponseExtractorType::Field("log_path"),
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
            parameters:      vec![Parameter::number(
                ParameterName::Entity,
                "The entity ID to watch for component list changes",
                true,
            )],
            response_format: ResponseSpecification {
                message_template: "Started list watch for entity {entity}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::WatchId,
                        response_extractor:  ResponseExtractorType::Field("watch_id"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogPath,
                        response_extractor:  ResponseExtractorType::Field("log_path"),
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
            name:            TOOL_CLEANUP_LOGS,
            description:     DESC_CLEANUP_LOGS,
            annotations:     BrpToolAnnotations::new(
                "Cleanup Log Files",
                ToolCategory::Logging,
                EnvironmentImpact::DestructiveNonIdempotent,
            ),
            handler:         HandlerFn::local(CleanupLogs),
            parameters:      vec![
                Parameter::string(
                    ParameterName::AppName,
                    "Optional filter to delete logs for a specific app only",
                    false,
                ),
                Parameter::number(
                    ParameterName::OlderThanSeconds,
                    "Optional filter to delete logs older than N seconds",
                    false,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Deleted {deleted_count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DeletedCount,
                        response_extractor:  ResponseExtractorType::Field("deleted_count"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::DeletedFiles,
                        response_extractor:  ResponseExtractorType::Field("deleted_files"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::AppNameFilter,
                        response_extractor:  ResponseExtractorType::Field("app_name_filter"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::OlderThanSeconds,
                        response_extractor:  ResponseExtractorType::Field("older_than_seconds"),
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
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Trace log found",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogPath,
                        response_extractor:  ResponseExtractorType::Field("log_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Exists,
                        response_extractor:  ResponseExtractorType::Field("exists"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::FileSizeBytes,
                        response_extractor:  ResponseExtractorType::Field("file_size_bytes"),
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
            handler:         HandlerFn::local_with_port(
                crate::app_tools::brp_launch_bevy_app::create_launch_bevy_app_handler(),
            ),
            parameters:      vec![
                Parameter::string(
                    ParameterName::AppName,
                    "Name of the Bevy app to launch",
                    true,
                ),
                Parameter::string(
                    ParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                ),
                Parameter::string(
                    ParameterName::Path,
                    "Path to use when multiple apps with the same name exist",
                    false,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Launched Bevy app `{app_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
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
                brp_launch_bevy_example::create_launch_bevy_example_handler(),
            ),
            parameters:      vec![
                Parameter::string(
                    ParameterName::ExampleName,
                    "Name of the Bevy example to launch",
                    true,
                ),
                Parameter::string(
                    ParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                ),
                Parameter::string(
                    ParameterName::Path,
                    "Path to use when multiple examples with the same name exist",
                    false,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Launched Bevy example `{example_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
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
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Found {count} Bevy apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Apps,
                        response_extractor:  ResponseExtractorType::Field("apps"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        response_extractor:  ResponseExtractorType::Count,
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
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Found {count} Bevy examples",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Examples,
                        response_extractor:  ResponseExtractorType::Field("examples"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        response_extractor:  ResponseExtractorType::Count,
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
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Found {count} BRP-enabled apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Apps,
                        response_extractor:  ResponseExtractorType::Field("apps"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        response_extractor:  ResponseExtractorType::Count,
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
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Found {count} active watches",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Watches,
                        response_extractor:  ResponseExtractorType::Field("watches"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        response_extractor:  ResponseExtractorType::Field("count"),
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
            parameters:      vec![Parameter::number(
                ParameterName::WatchId,
                "The watch ID returned from bevy_start_entity_watch or bevy_start_list_watch",
                true,
            )],
            response_format: ResponseSpecification {
                message_template: "Successfully stopped watch",
                response_fields:  vec![],
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
            parameters:      vec![
                Parameter::string(
                    ParameterName::AppName,
                    "Optional filter to list logs for a specific app only",
                    false,
                ),
                Parameter::boolean(
                    ParameterName::Verbose,
                    "Include full details (path, timestamps, size in bytes). Default is false for minimal output",
                    false,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Found {count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Logs,
                        response_extractor:  ResponseExtractorType::Field("logs"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TempDirectory,
                        response_extractor:  ResponseExtractorType::Field("temp_directory"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Count,
                        response_extractor:  ResponseExtractorType::Count,
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
            parameters:      vec![
                Parameter::string(
                    ParameterName::Filename,
                    "The log filename (e.g., bevy_brp_mcp_myapp_1234567890.log)",
                    true,
                ),
                Parameter::string(
                    ParameterName::Keyword,
                    "Optional keyword to filter lines (case-insensitive)",
                    false,
                ),
                Parameter::number(
                    ParameterName::TailLines,
                    "Optional number of lines to read from the end of file",
                    false,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully read log file: {filename}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Filename,
                        response_extractor:  ResponseExtractorType::Field("filename"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::FilePath,
                        response_extractor:  ResponseExtractorType::Field("file_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::SizeBytes,
                        response_extractor:  ResponseExtractorType::Field("size_bytes"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::SizeHuman,
                        response_extractor:  ResponseExtractorType::Field("size_human"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LinesRead,
                        response_extractor:  ResponseExtractorType::Field("lines_read"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Content,
                        response_extractor:  ResponseExtractorType::SplitContentIntoLines,
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::FilteredByKeyword,
                        response_extractor:  ResponseExtractorType::Field("filtered_by_keyword"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TailMode,
                        response_extractor:  ResponseExtractorType::Field("tail_mode"),
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
            parameters:      vec![Parameter::string(
                ParameterName::Level,
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            )],
            response_format: ResponseSpecification {
                message_template: "Tracing level set to '{level}' - diagnostic information will be logged to temp directory",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::TracingLevel,
                        response_extractor:  ResponseExtractorType::Field("level"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::LogFile,
                        response_extractor:  ResponseExtractorType::Field("log_file"),
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
            parameters:      vec![Parameter::string(
                ParameterName::AppName,
                "Name of the process to check for",
                true,
            )],
            response_format: ResponseSpecification {
                message_template: "Status check for `{app_name}` on port {port}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name: ResponseFieldName::AppName,
                        parameter_name:      ParameterName::AppName,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::AppRunning,
                        response_extractor:  ResponseExtractorType::Field("app_running"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::BrpResponsive,
                        response_extractor:  ResponseExtractorType::Field("brp_responsive"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::Pid,
                        response_extractor:  ResponseExtractorType::Field("pid"),
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
            parameters:      vec![Parameter::string(
                ParameterName::AppName,
                "Name of the Bevy app to shutdown",
                true,
            )],
            response_format: ResponseSpecification {
                message_template: "{shutdown_message}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::ShutdownMethod,
                        response_extractor:  ResponseExtractorType::Field("shutdown_method"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: ResponseFieldName::AppName,
                        response_extractor:  ResponseExtractorType::Field("app_name"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponseNullableWithPlacement {
                        response_field_name: ResponseFieldName::Pid,
                        response_extractor:  ResponseExtractorType::Field("pid"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
    ]
}
