//! Tool definitions for BRP and local MCP tools.

use super::brp_tool_def::{BrpMethodSource, BrpToolDef};
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
use super::local_tool_def::LocalToolDef;
use super::parameters::{BrpParameter, LocalParameter, LocalParameterName};
use super::tool_definition::ToolDefinition;
use crate::app_tools::brp_launch_bevy_example;
use crate::app_tools::brp_list_bevy_apps::ListBevyApps;
use crate::app_tools::brp_list_bevy_examples::ListBevyExamples;
use crate::app_tools::brp_list_brp_apps::ListBrpApps;
use crate::app_tools::brp_shutdown::Shutdown;
use crate::app_tools::brp_status::Status;
use crate::brp_tools::watch::bevy_get_watch::BevyGetWatch;
use crate::brp_tools::watch::bevy_list_watch::BevyListWatch;
use crate::brp_tools::watch::brp_list_active::BrpListActiveWatches;
use crate::brp_tools::watch::brp_stop_watch::BrpStopWatch;
use crate::constants::{
    JSON_FIELD_APP_NAME, JSON_FIELD_APPS, JSON_FIELD_COMPONENT_COUNT, JSON_FIELD_COMPONENTS,
    JSON_FIELD_COUNT, JSON_FIELD_ENTITIES, JSON_FIELD_ENTITY, JSON_FIELD_ENTITY_COUNT,
    JSON_FIELD_LOG_PATH, JSON_FIELD_PARENT, JSON_FIELD_PATH, JSON_FIELD_PID, JSON_FIELD_RESOURCE,
    JSON_FIELD_SHUTDOWN_METHOD, PARAM_APP_NAME, PARAM_ENTITIES, PARAM_PARENT, PARAM_PATH,
    PARAM_RESOURCE,
};
use crate::log_tools::cleanup_logs::CleanupLogs;
use crate::log_tools::get_trace_log_path::GetTraceLogPath;
use crate::log_tools::list_logs::ListLogs;
use crate::log_tools::read_log::ReadLog;
use crate::log_tools::set_tracing_level::SetTracingLevel;
use crate::response::{
    FieldPlacement, ResponseExtractorType, ResponseField, ResponseSpecification,
};
use crate::tool::HandlerFn;
use crate::tool::constants::{
    DESC_LIST_ACTIVE_WATCHES, DESC_STOP_WATCH, TOOL_LIST_ACTIVE_WATCHES, TOOL_STOP_WATCH,
};

/// Get all tool definitions for registration with the MCP service
#[allow(clippy::too_many_lines)]
pub fn get_all_tool_definitions() -> Vec<Box<dyn ToolDefinition>> {
    vec![
        // BrpToolDef/bevy_destroy
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_DESTROY,
            description:     DESC_BEVY_DESTROY,
            method_source:   BrpMethodSource::Static(BRP_METHOD_DESTROY),
            parameters:      vec![BrpParameter::entity("The entity ID to destroy", true)],
            response_format: ResponseSpecification {
                message_template: "Successfully destroyed entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: JSON_FIELD_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/bevy_get
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_GET,
            description:     DESC_BEVY_GET,
            method_source:   BrpMethodSource::Static(BRP_METHOD_GET),
            parameters:      vec![
                BrpParameter::entity("The entity ID to get component data from", true),
                BrpParameter::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Retrieved component data from entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: JSON_FIELD_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COMPONENTS,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_COMPONENTS),
                        placement:           FieldPlacement::Result,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_get_resource
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_GET_RESOURCE,
            description:     DESC_BEVY_GET_RESOURCE,
            method_source:   BrpMethodSource::Static(BRP_METHOD_GET_RESOURCE),
            parameters:      vec![BrpParameter::resource(
                "The fully-qualified type name of the resource to get",
            )],
            response_format: ResponseSpecification {
                message_template: "Retrieved resource: {resource}",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        }),
        // BrpToolDef/bevy_insert
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_INSERT,
            description:     DESC_BEVY_INSERT,
            method_source:   BrpMethodSource::Static(BRP_METHOD_INSERT),
            parameters:      vec![
                BrpParameter::entity("The entity ID to insert components into", true),
                BrpParameter::components(
                    "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully inserted components into entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: JSON_FIELD_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_COMPONENTS,
                        parameter_field_name: JSON_FIELD_COMPONENTS,
                        placement:            FieldPlacement::Result,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_insert_resource
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_INSERT_RESOURCE,
            description:     DESC_BEVY_INSERT_RESOURCE,
            method_source:   BrpMethodSource::Static(BRP_METHOD_INSERT_RESOURCE),
            parameters:      vec![
                BrpParameter::resource(
                    "The fully-qualified type name of the resource to insert or update",
                ),
                BrpParameter::value(
                    "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully inserted/updated resource: {resource}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/bevy_list
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_LIST,
            description:     DESC_BEVY_LIST,
            method_source:   BrpMethodSource::Static(BRP_METHOD_LIST),
            parameters:      vec![BrpParameter::entity(
                "Optional entity ID to list components for",
                false,
            )],
            response_format: ResponseSpecification {
                message_template: "Listed {count} components",
                response_fields:  vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_list_resources
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_LIST_RESOURCES,
            description:     DESC_BEVY_LIST_RESOURCES,
            method_source:   BrpMethodSource::Static(BRP_METHOD_LIST_RESOURCES),
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Listed {count} resources",
                response_fields:  vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_mutate_component
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_MUTATE_COMPONENT,
            description:     DESC_BEVY_MUTATE_COMPONENT,
            method_source:   BrpMethodSource::Static(BRP_METHOD_MUTATE_COMPONENT),
            parameters:      vec![
                BrpParameter::entity("The entity ID containing the component to mutate", true),
                BrpParameter::component("The fully-qualified type name of the component to mutate"),
                BrpParameter::path(
                    "The path to the field within the component (e.g., 'translation.x')",
                ),
                BrpParameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully mutated component on entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: JSON_FIELD_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/bevy_mutate_resource
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_MUTATE_RESOURCE,
            description:     DESC_BEVY_MUTATE_RESOURCE,
            method_source:   BrpMethodSource::Static(BRP_METHOD_MUTATE_RESOURCE),
            parameters:      vec![
                BrpParameter::resource("The fully-qualified type name of the resource to mutate"),
                BrpParameter::path(
                    "The path to the field within the resource (e.g., 'settings.volume')",
                ),
                BrpParameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully mutated resource: `{resource}`",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/bevy_query
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_QUERY,
            description:     DESC_BEVY_QUERY,
            method_source:   BrpMethodSource::Static(BRP_METHOD_QUERY),
            parameters:      vec![
                BrpParameter::data(),
                BrpParameter::filter(),
                BrpParameter::strict(),
            ],
            response_format: ResponseSpecification {
                message_template: "Query completed successfully",
                response_fields:  vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_ENTITY_COUNT,
                        response_extractor:  ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COMPONENT_COUNT,
                        response_extractor:  ResponseExtractorType::QueryComponentCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_registry_schema
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_REGISTRY_SCHEMA,
            description:     DESC_BEVY_REGISTRY_SCHEMA,
            method_source:   BrpMethodSource::Static(BRP_METHOD_REGISTRY_SCHEMA),
            parameters:      vec![
                BrpParameter::with_crates(),
                BrpParameter::without_crates(),
                BrpParameter::with_types(),
                BrpParameter::without_types(),
            ],
            response_format: ResponseSpecification {
                message_template: "Retrieved schema information",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        }),
        // BrpToolDef/bevy_remove
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_REMOVE,
            description:     DESC_BEVY_REMOVE,
            method_source:   BrpMethodSource::Static(BRP_METHOD_REMOVE),
            parameters:      vec![
                BrpParameter::entity("The entity ID to remove components from", true),
                BrpParameter::components("Array of component type names to remove", true),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully removed components from entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: JSON_FIELD_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_COMPONENTS,
                        parameter_field_name: JSON_FIELD_COMPONENTS,
                        placement:            FieldPlacement::Result,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_remove_resource
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_REMOVE_RESOURCE,
            description:     DESC_BEVY_REMOVE_RESOURCE,
            method_source:   BrpMethodSource::Static(BRP_METHOD_REMOVE_RESOURCE),
            parameters:      vec![BrpParameter::resource(
                "The fully-qualified type name of the resource to remove",
            )],
            response_format: ResponseSpecification {
                message_template: "Successfully removed resource",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/bevy_reparent
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_REPARENT,
            description:     DESC_BEVY_REPARENT,
            method_source:   BrpMethodSource::Static(BRP_METHOD_REPARENT),
            parameters:      vec![
                BrpParameter::entities("Array of entity IDs to reparent"),
                BrpParameter::parent(),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully reparented entities",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITIES,
                        parameter_field_name: PARAM_ENTITIES,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_PARENT,
                        parameter_field_name: PARAM_PARENT,
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_rpc_discover
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_RPC_DISCOVER,
            description:     DESC_BEVY_RPC_DISCOVER,
            method_source:   BrpMethodSource::Static(BRP_METHOD_RPC_DISCOVER),
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Retrieved BRP method discovery information",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        }),
        // BrpToolDef/bevy_spawn
        // todo: (later) make this match curl
        Box::new(BrpToolDef {
            name:            TOOL_BEVY_SPAWN,
            description:     DESC_BEVY_SPAWN,
            method_source:   BrpMethodSource::Static(BRP_METHOD_SPAWN),
            parameters:      vec![BrpParameter::components(
                "Object containing component data to spawn with. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                false,
            )],
            response_format: ResponseSpecification {
                message_template: "Successfully spawned entity",
                response_fields:  vec![ResponseField::FromResponse {
                    response_field_name: JSON_FIELD_ENTITY,
                    response_extractor:  ResponseExtractorType::Field(JSON_FIELD_ENTITY),
                    placement:           FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/brp_execute
        // this is the one brp tool that uses BrpMethodSource::Dynamic
        // as the user can dynamically pass in the method
        // we use this enum to make sure we don't accidentally create
        // a method param on any other BrpToolDef as it's a special case
        Box::new(BrpToolDef {
            name:            TOOL_BRP_EXECUTE,
            description:     DESC_BRP_EXECUTE,
            method_source:   BrpMethodSource::Dynamic,
            parameters:      vec![BrpParameter::params(
                "Optional parameters for the method, as a JSON object or array",
                false,
            )],
            response_format: ResponseSpecification {
                message_template: "Method executed successfully",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        }),
        // BrpToolDef/brp_extras_discover_format
        Box::new(BrpToolDef {
            name:            TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
            description:     DESC_BRP_EXTRAS_DISCOVER_FORMAT,
            method_source:   BrpMethodSource::Static(BRP_METHOD_EXTRAS_DISCOVER_FORMAT),
            parameters:      vec![BrpParameter::types(
                "Array of fully-qualified component type names to discover formats for",
                true,
            )],
            response_format: ResponseSpecification {
                message_template: "Format discovery completed",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        }),
        // BrpToolDef/brp_extras_screenshot
        Box::new(BrpToolDef {
            name:            TOOL_BRP_EXTRAS_SCREENSHOT,
            description:     DESC_BRP_EXTRAS_SCREENSHOT,
            method_source:   BrpMethodSource::Static(BRP_METHOD_EXTRAS_SCREENSHOT),
            parameters:      vec![BrpParameter::path(
                "File path where the screenshot should be saved",
            )],
            response_format: ResponseSpecification {
                message_template: "Successfully captured screenshot",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_PATH,
                    parameter_field_name: PARAM_PATH,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        // BrpToolDef/brp_extras_send_keys
        Box::new(BrpToolDef {
            name:            TOOL_BRP_EXTRAS_SEND_KEYS,
            description:     DESC_BRP_EXTRAS_SEND_KEYS,
            method_source:   BrpMethodSource::Static(BRP_METHOD_EXTRAS_SEND_KEYS),
            parameters:      vec![BrpParameter::keys(), BrpParameter::duration_ms()],
            response_format: ResponseSpecification {
                message_template: "Successfully sent keyboard input",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "keys_sent",
                        response_extractor:  ResponseExtractorType::Field("keys_sent"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "duration_ms",
                        response_extractor:  ResponseExtractorType::Field("duration_ms"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // BrpToolDef/brp_extras_set_debug_mode
        Box::new(BrpToolDef {
            name:            TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
            description:     DESC_BRP_EXTRAS_SET_DEBUG_MODE,
            method_source:   BrpMethodSource::Static(BRP_METHOD_EXTRAS_SET_DEBUG_MODE),
            parameters:      vec![BrpParameter::enabled()],
            response_format: ResponseSpecification {
                message_template: "Debug mode updated successfully",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "debug_enabled",
                        response_extractor:  ResponseExtractorType::Field("debug_enabled"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "details",
                        response_extractor:  ResponseExtractorType::Field("message"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // BevyGetWatch and BevyListWatch are unusual in that
        // ultimately we do call bevy/get+watch and bevy/list+watch
        // but we need the local tool in order to set up the watch to stream
        // the results and log them to a file

        // LocalToolDef/bevy_get_watch
        Box::new(LocalToolDef {
            name:        TOOL_BEVY_GET_WATCH,
            description: DESC_BEVY_GET_WATCH,
            handler:     HandlerFn::local_with_port(BevyGetWatch),
            parameters:  vec![
                LocalParameter::number(
                    LocalParameterName::Entity,
                    "The entity ID to watch for component changes",
                    true,
                ),
                LocalParameter::string_array(
                    LocalParameterName::Components,
                    "Required array of component types to watch. Must contain at least one component. Without this, the watch will not detect any changes.",
                    true,
                ),
            ],
            formatter:   ResponseSpecification {
                message_template: "Started entity watch for entity {entity}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "watch_id",
                        response_extractor:  ResponseExtractorType::Field("watch_id"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_LOG_PATH,
                        response_extractor:  ResponseExtractorType::Field("log_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: "entity",
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/bevy_list_watch
        Box::new(LocalToolDef {
            name:        TOOL_BEVY_LIST_WATCH,
            description: DESC_BEVY_LIST_WATCH,
            handler:     HandlerFn::local_with_port(BevyListWatch),
            parameters:  vec![LocalParameter::number(
                LocalParameterName::Entity,
                "The entity ID to watch for component list changes",
                true,
            )],
            formatter:   ResponseSpecification {
                message_template: "Started list watch for entity {entity}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "watch_id",
                        response_extractor:  ResponseExtractorType::Field("watch_id"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_LOG_PATH,
                        response_extractor:  ResponseExtractorType::Field("log_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: "entity",
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/cleanup_logs
        Box::new(LocalToolDef {
            name:        TOOL_CLEANUP_LOGS,
            description: DESC_CLEANUP_LOGS,
            handler:     HandlerFn::local(CleanupLogs),
            parameters:  vec![
                LocalParameter::string(
                    LocalParameterName::AppName,
                    "Optional filter to delete logs for a specific app only",
                    false,
                ),
                LocalParameter::number(
                    LocalParameterName::OlderThanSeconds,
                    "Optional filter to delete logs older than N seconds",
                    false,
                ),
            ],
            formatter:   ResponseSpecification {
                message_template: "Deleted {deleted_count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "deleted_count",
                        response_extractor:  ResponseExtractorType::Field("deleted_count"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "deleted_files",
                        response_extractor:  ResponseExtractorType::Field("deleted_files"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "app_name_filter",
                        response_extractor:  ResponseExtractorType::Field("app_name_filter"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "older_than_seconds",
                        response_extractor:  ResponseExtractorType::Field("older_than_seconds"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/get_trace_log_path
        Box::new(LocalToolDef {
            name:        TOOL_GET_TRACE_LOG_PATH,
            description: DESC_GET_TRACE_LOG_PATH,
            handler:     HandlerFn::local(GetTraceLogPath),
            parameters:  vec![],
            formatter:   ResponseSpecification {
                message_template: "Trace log found",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_LOG_PATH,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_LOG_PATH),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "exists",
                        response_extractor:  ResponseExtractorType::Field("exists"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "file_size_bytes",
                        response_extractor:  ResponseExtractorType::Field("file_size_bytes"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/launch_bevy_app
        Box::new(LocalToolDef {
            name:        TOOL_LAUNCH_BEVY_APP,
            description: DESC_LAUNCH_BEVY_APP,
            handler:     HandlerFn::local_with_port(
                crate::app_tools::brp_launch_bevy_app::create_launch_bevy_app_handler(),
            ),
            parameters:  vec![
                LocalParameter::string(
                    LocalParameterName::AppName,
                    "Name of the Bevy app to launch",
                    true,
                ),
                LocalParameter::string(
                    LocalParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                ),
                LocalParameter::string(
                    LocalParameterName::Path,
                    "Path to use when multiple apps with the same name exist",
                    false,
                ),
            ],
            formatter:   ResponseSpecification {
                message_template: "Launched Bevy app `{app_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        }),
        // LocalToolDef/launch_bevy_example
        Box::new(LocalToolDef {
            name:        TOOL_LAUNCH_BEVY_EXAMPLE,
            description: DESC_LAUNCH_BEVY_EXAMPLE,
            handler:     HandlerFn::local_with_port(
                brp_launch_bevy_example::create_launch_bevy_example_handler(),
            ),
            parameters:  vec![
                LocalParameter::string(
                    LocalParameterName::ExampleName,
                    "Name of the Bevy example to launch",
                    true,
                ),
                LocalParameter::string(
                    LocalParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                ),
                LocalParameter::string(
                    LocalParameterName::Path,
                    "Path to use when multiple examples with the same name exist",
                    false,
                ),
            ],
            formatter:   ResponseSpecification {
                message_template: "Launched Bevy example `{example_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        }),
        // LocalToolDef/list_bevy_apps
        Box::new(LocalToolDef {
            name:        TOOL_LIST_BEVY_APPS,
            description: DESC_LIST_BEVY_APPS,
            handler:     HandlerFn::local(ListBevyApps),
            parameters:  vec![],
            formatter:   ResponseSpecification {
                message_template: "Found {count} Bevy apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_APPS,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_APPS),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/list_bevy_examples
        Box::new(LocalToolDef {
            name:        TOOL_LIST_BEVY_EXAMPLES,
            description: DESC_LIST_BEVY_EXAMPLES,
            handler:     HandlerFn::local(ListBevyExamples),
            parameters:  vec![],
            formatter:   ResponseSpecification {
                message_template: "Found {count} Bevy examples",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "examples",
                        response_extractor:  ResponseExtractorType::Field("examples"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/list_brp_apps
        Box::new(LocalToolDef {
            name:        TOOL_LIST_BRP_APPS,
            description: DESC_LIST_BRP_APPS,
            handler:     HandlerFn::local(ListBrpApps),
            parameters:  vec![],
            formatter:   ResponseSpecification {
                message_template: "Found {count} BRP-enabled apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_APPS,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_APPS),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/brp_list_active_watches
        Box::new(LocalToolDef {
            name:        TOOL_LIST_ACTIVE_WATCHES,
            description: DESC_LIST_ACTIVE_WATCHES,
            handler:     HandlerFn::local(BrpListActiveWatches),
            parameters:  vec![],
            formatter:   ResponseSpecification {
                message_template: "Found {count} active watches",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "watches",
                        response_extractor:  ResponseExtractorType::Field("watches"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_COUNT),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/brp_stop_watch
        Box::new(LocalToolDef {
            name:        TOOL_STOP_WATCH,
            description: DESC_STOP_WATCH,
            handler:     HandlerFn::local(BrpStopWatch),
            parameters:  vec![LocalParameter::number(
                LocalParameterName::WatchId,
                "The watch ID returned from bevy_start_entity_watch or bevy_start_list_watch",
                true,
            )],
            formatter:   ResponseSpecification {
                message_template: "Successfully stopped watch",
                response_fields:  vec![],
            },
        }),
        // LocalToolDef/list_logs
        Box::new(LocalToolDef {
            name:        TOOL_LIST_LOGS,
            description: DESC_LIST_LOGS,
            handler:     HandlerFn::local(ListLogs),
            parameters:  vec![
                LocalParameter::string(
                    LocalParameterName::AppName,
                    "Optional filter to list logs for a specific app only",
                    false,
                ),
                LocalParameter::boolean(
                    LocalParameterName::Verbose,
                    "Include full details (path, timestamps, size in bytes). Default is false for minimal output",
                    false,
                ),
            ],
            formatter:   ResponseSpecification {
                message_template: "Found {count} log files",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "logs",
                        response_extractor:  ResponseExtractorType::Field("logs"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "temp_directory",
                        response_extractor:  ResponseExtractorType::Field("temp_directory"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/read_log
        Box::new(LocalToolDef {
            name:        TOOL_READ_LOG,
            description: DESC_READ_LOG,
            handler:     HandlerFn::local(ReadLog),
            parameters:  vec![
                LocalParameter::string(
                    LocalParameterName::Filename,
                    "The log filename (e.g., bevy_brp_mcp_myapp_1234567890.log)",
                    true,
                ),
                LocalParameter::string(
                    LocalParameterName::Keyword,
                    "Optional keyword to filter lines (case-insensitive)",
                    false,
                ),
                LocalParameter::number(
                    LocalParameterName::TailLines,
                    "Optional number of lines to read from the end of file",
                    false,
                ),
            ],
            formatter:   ResponseSpecification {
                message_template: "Successfully read log file: {filename}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "filename",
                        response_extractor:  ResponseExtractorType::Field("filename"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "file_path",
                        response_extractor:  ResponseExtractorType::Field("file_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "size_bytes",
                        response_extractor:  ResponseExtractorType::Field("size_bytes"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "size_human",
                        response_extractor:  ResponseExtractorType::Field("size_human"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "lines_read",
                        response_extractor:  ResponseExtractorType::Field("lines_read"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "content",
                        response_extractor:  ResponseExtractorType::SplitContentIntoLines,
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "filtered_by_keyword",
                        response_extractor:  ResponseExtractorType::Field("filtered_by_keyword"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "tail_mode",
                        response_extractor:  ResponseExtractorType::Field("tail_mode"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/set_tracing_level
        Box::new(LocalToolDef {
            name:        TOOL_SET_TRACING_LEVEL,
            description: DESC_SET_TRACING_LEVEL,
            handler:     HandlerFn::local(SetTracingLevel),
            parameters:  vec![LocalParameter::string(
                LocalParameterName::Level,
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            )],
            formatter:   ResponseSpecification {
                message_template: "Tracing level set to '{level}' - diagnostic information will be logged to temp directory",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "tracing_level",
                        response_extractor:  ResponseExtractorType::Field("level"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "log_file",
                        response_extractor:  ResponseExtractorType::Field("log_file"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/status
        Box::new(LocalToolDef {
            name:        TOOL_STATUS,
            description: DESC_STATUS,
            handler:     HandlerFn::local_with_port(Status),
            parameters:  vec![LocalParameter::string(
                LocalParameterName::AppName,
                "Name of the process to check for",
                true,
            )],
            formatter:   ResponseSpecification {
                message_template: "Status check for `{app_name}` on port {port}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_APP_NAME,
                        parameter_field_name: PARAM_APP_NAME,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "app_running",
                        response_extractor:  ResponseExtractorType::Field("app_running"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "brp_responsive",
                        response_extractor:  ResponseExtractorType::Field("brp_responsive"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_PID,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_PID),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/shutdown
        Box::new(LocalToolDef {
            name:        TOOL_SHUTDOWN,
            description: DESC_SHUTDOWN,
            handler:     HandlerFn::local_with_port(Shutdown),
            parameters:  vec![LocalParameter::string(
                LocalParameterName::AppName,
                "Name of the Bevy app to shutdown",
                true,
            )],
            formatter:   ResponseSpecification {
                message_template: "{shutdown_message}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_SHUTDOWN_METHOD,
                        response_extractor:  ResponseExtractorType::Field(
                            JSON_FIELD_SHUTDOWN_METHOD,
                        ),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_APP_NAME,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_APP_NAME),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponseNullableWithPlacement {
                        response_field_name: JSON_FIELD_PID,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_PID),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
    ]
}
