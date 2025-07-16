//! Tool definitions for BRP and local MCP tools.

use super::HandlerFn;
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
use super::parameters::{BrpParameter, LocalParameter, LocalParameterName, UnifiedParameter};
use super::tool_def::ToolDef;
use crate::app_tools::brp_launch_bevy_example;
use crate::app_tools::brp_list_bevy_apps::ListBevyApps;
use crate::app_tools::brp_list_bevy_examples::ListBevyExamples;
use crate::app_tools::brp_list_brp_apps::ListBrpApps;
use crate::app_tools::brp_shutdown::Shutdown;
use crate::app_tools::brp_status::Status;
use crate::brp_tools::request_handler::BrpMethodHandler;
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
use crate::tool::constants::{
    DESC_LIST_ACTIVE_WATCHES, DESC_STOP_WATCH, TOOL_LIST_ACTIVE_WATCHES, TOOL_STOP_WATCH,
};

/// Get all tool definitions for registration with the MCP service
#[allow(clippy::too_many_lines)]
pub fn get_all_tool_definitions() -> Vec<ToolDef> {
    vec![
        // UnifiedToolDef/bevy_destroy
        ToolDef {
            name:            TOOL_BEVY_DESTROY,
            description:     DESC_BEVY_DESTROY,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_DESTROY),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::entity(
                "The entity ID to destroy",
                true,
            ))],
            response_format: ResponseSpecification {
                message_template: "Successfully destroyed entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: JSON_FIELD_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/bevy_get
        ToolDef {
            name:            TOOL_BEVY_GET,
            description:     DESC_BEVY_GET,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_GET),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::entity(
                    "The entity ID to get component data from",
                    true,
                )),
                UnifiedParameter::Brp(BrpParameter::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                )),
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
        },
        // UnifiedToolDef/bevy_get_resource
        ToolDef {
            name:            TOOL_BEVY_GET_RESOURCE,
            description:     DESC_BEVY_GET_RESOURCE,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_GET_RESOURCE),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::resource(
                "The fully-qualified type name of the resource to get",
            ))],
            response_format: ResponseSpecification {
                message_template: "Retrieved resource: {resource}",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // UnifiedToolDef/bevy_insert
        ToolDef {
            name:            TOOL_BEVY_INSERT,
            description:     DESC_BEVY_INSERT,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_INSERT),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::entity(
                    "The entity ID to insert components into",
                    true,
                )),
                UnifiedParameter::Brp(BrpParameter::components(
                    "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                )),
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
        },
        // UnifiedToolDef/bevy_insert_resource
        ToolDef {
            name:            TOOL_BEVY_INSERT_RESOURCE,
            description:     DESC_BEVY_INSERT_RESOURCE,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_INSERT_RESOURCE),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::resource(
                    "The fully-qualified type name of the resource to insert or update",
                )),
                UnifiedParameter::Brp(BrpParameter::value(
                    "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                )),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully inserted/updated resource: {resource}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/bevy_list
        ToolDef {
            name:            TOOL_BEVY_LIST,
            description:     DESC_BEVY_LIST,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_LIST),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::entity(
                "Optional entity ID to list components for",
                false,
            ))],
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
        },
        // UnifiedToolDef/bevy_list_resources
        ToolDef {
            name:            TOOL_BEVY_LIST_RESOURCES,
            description:     DESC_BEVY_LIST_RESOURCES,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_LIST_RESOURCES),
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
        },
        // UnifiedToolDef/bevy_mutate_component
        ToolDef {
            name:            TOOL_BEVY_MUTATE_COMPONENT,
            description:     DESC_BEVY_MUTATE_COMPONENT,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_MUTATE_COMPONENT),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::entity(
                    "The entity ID containing the component to mutate",
                    true,
                )),
                UnifiedParameter::Brp(BrpParameter::component(
                    "The fully-qualified type name of the component to mutate",
                )),
                UnifiedParameter::Brp(BrpParameter::path(
                    "The path to the field within the component (e.g., 'translation.x')",
                )),
                UnifiedParameter::Brp(BrpParameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                )),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully mutated component on entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: JSON_FIELD_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/bevy_mutate_resource
        ToolDef {
            name:            TOOL_BEVY_MUTATE_RESOURCE,
            description:     DESC_BEVY_MUTATE_RESOURCE,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_MUTATE_RESOURCE),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::resource(
                    "The fully-qualified type name of the resource to mutate",
                )),
                UnifiedParameter::Brp(BrpParameter::path(
                    "The path to the field within the resource (e.g., 'settings.volume')",
                )),
                UnifiedParameter::Brp(BrpParameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                )),
            ],
            response_format: ResponseSpecification {
                message_template: "Successfully mutated resource: `{resource}`",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/bevy_query
        ToolDef {
            name:            TOOL_BEVY_QUERY,
            description:     DESC_BEVY_QUERY,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_QUERY),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::data()),
                UnifiedParameter::Brp(BrpParameter::filter()),
                UnifiedParameter::Brp(BrpParameter::strict()),
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
        },
        // UnifiedToolDef/bevy_registry_schema
        ToolDef {
            name:            TOOL_BEVY_REGISTRY_SCHEMA,
            description:     DESC_BEVY_REGISTRY_SCHEMA,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_REGISTRY_SCHEMA),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::with_crates()),
                UnifiedParameter::Brp(BrpParameter::without_crates()),
                UnifiedParameter::Brp(BrpParameter::with_types()),
                UnifiedParameter::Brp(BrpParameter::without_types()),
            ],
            response_format: ResponseSpecification {
                message_template: "Retrieved schema information",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // UnifiedToolDef/bevy_remove
        ToolDef {
            name:            TOOL_BEVY_REMOVE,
            description:     DESC_BEVY_REMOVE,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_REMOVE),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::entity(
                    "The entity ID to remove components from",
                    true,
                )),
                UnifiedParameter::Brp(BrpParameter::components(
                    "Array of component type names to remove",
                    true,
                )),
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
        },
        // UnifiedToolDef/bevy_remove_resource
        ToolDef {
            name:            TOOL_BEVY_REMOVE_RESOURCE,
            description:     DESC_BEVY_REMOVE_RESOURCE,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_REMOVE_RESOURCE),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::resource(
                "The fully-qualified type name of the resource to remove",
            ))],
            response_format: ResponseSpecification {
                message_template: "Successfully removed resource",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/bevy_reparent
        ToolDef {
            name:            TOOL_BEVY_REPARENT,
            description:     DESC_BEVY_REPARENT,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_REPARENT),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::entities("Array of entity IDs to reparent")),
                UnifiedParameter::Brp(BrpParameter::parent()),
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
        },
        // UnifiedToolDef/bevy_rpc_discover
        ToolDef {
            name:            TOOL_BEVY_RPC_DISCOVER,
            description:     DESC_BEVY_RPC_DISCOVER,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_RPC_DISCOVER),
            parameters:      vec![],
            response_format: ResponseSpecification {
                message_template: "Retrieved BRP method discovery information",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // UnifiedToolDef/bevy_spawn
        // todo: (later) make this match curl
        ToolDef {
            name:            TOOL_BEVY_SPAWN,
            description:     DESC_BEVY_SPAWN,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_SPAWN),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::components(
                "Object containing component data to spawn with. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                false,
            ))],
            response_format: ResponseSpecification {
                message_template: "Successfully spawned entity",
                response_fields:  vec![ResponseField::FromResponse {
                    response_field_name: JSON_FIELD_ENTITY,
                    response_extractor:  ResponseExtractorType::Field(JSON_FIELD_ENTITY),
                    placement:           FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/brp_execute
        // this is the one brp tool that uses dynamic method handling
        // as the user can dynamically pass in the method
        ToolDef {
            name:            TOOL_BRP_EXECUTE,
            description:     DESC_BRP_EXECUTE,
            handler:         HandlerFn::brp_dynamic(BrpMethodHandler),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::params(
                "Optional parameters for the method, as a JSON object or array",
                false,
            ))],
            response_format: ResponseSpecification {
                message_template: "Method executed successfully",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // UnifiedToolDef/brp_extras_discover_format
        ToolDef {
            name:            TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
            description:     DESC_BRP_EXTRAS_DISCOVER_FORMAT,
            handler:         HandlerFn::brp_static(
                BrpMethodHandler,
                BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
            ),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::types(
                "Array of fully-qualified component type names to discover formats for",
                true,
            ))],
            response_format: ResponseSpecification {
                message_template: "Format discovery completed",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // UnifiedToolDef/brp_extras_screenshot
        ToolDef {
            name:            TOOL_BRP_EXTRAS_SCREENSHOT,
            description:     DESC_BRP_EXTRAS_SCREENSHOT,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_EXTRAS_SCREENSHOT),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::path(
                "File path where the screenshot should be saved",
            ))],
            response_format: ResponseSpecification {
                message_template: "Successfully captured screenshot",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_PATH,
                    parameter_field_name: PARAM_PATH,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // UnifiedToolDef/brp_extras_send_keys
        ToolDef {
            name:            TOOL_BRP_EXTRAS_SEND_KEYS,
            description:     DESC_BRP_EXTRAS_SEND_KEYS,
            handler:         HandlerFn::brp_static(BrpMethodHandler, BRP_METHOD_EXTRAS_SEND_KEYS),
            parameters:      vec![
                UnifiedParameter::Brp(BrpParameter::keys()),
                UnifiedParameter::Brp(BrpParameter::duration_ms()),
            ],
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
        },
        // UnifiedToolDef/brp_extras_set_debug_mode
        ToolDef {
            name:            TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
            description:     DESC_BRP_EXTRAS_SET_DEBUG_MODE,
            handler:         HandlerFn::brp_static(
                BrpMethodHandler,
                BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
            ),
            parameters:      vec![UnifiedParameter::Brp(BrpParameter::enabled())],
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
        },
        // BevyGetWatch and BevyListWatch are unusual in that
        // ultimately we do call bevy/get+watch and bevy/list+watch
        // but we need the local tool in order to set up the watch to stream
        // the results and log them to a file

        // UnifiedToolDef/bevy_get_watch
        ToolDef {
            name:            TOOL_BEVY_GET_WATCH,
            description:     DESC_BEVY_GET_WATCH,
            handler:         HandlerFn::local_with_port(BevyGetWatch),
            parameters:      vec![
                UnifiedParameter::Local(LocalParameter::number(
                    LocalParameterName::Entity,
                    "The entity ID to watch for component changes",
                    true,
                )),
                UnifiedParameter::Local(LocalParameter::string_array(
                    LocalParameterName::Components,
                    "Required array of component types to watch. Must contain at least one component. Without this, the watch will not detect any changes.",
                    true,
                )),
            ],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/bevy_list_watch
        ToolDef {
            name:            TOOL_BEVY_LIST_WATCH,
            description:     DESC_BEVY_LIST_WATCH,
            handler:         HandlerFn::local_with_port(BevyListWatch),
            parameters:      vec![UnifiedParameter::Local(LocalParameter::number(
                LocalParameterName::Entity,
                "The entity ID to watch for component list changes",
                true,
            ))],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/cleanup_logs
        ToolDef {
            name:            TOOL_CLEANUP_LOGS,
            description:     DESC_CLEANUP_LOGS,
            handler:         HandlerFn::local(CleanupLogs),
            parameters:      vec![
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::AppName,
                    "Optional filter to delete logs for a specific app only",
                    false,
                )),
                UnifiedParameter::Local(LocalParameter::number(
                    LocalParameterName::OlderThanSeconds,
                    "Optional filter to delete logs older than N seconds",
                    false,
                )),
            ],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/get_trace_log_path
        ToolDef {
            name:            TOOL_GET_TRACE_LOG_PATH,
            description:     DESC_GET_TRACE_LOG_PATH,
            handler:         HandlerFn::local(GetTraceLogPath),
            parameters:      vec![],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/launch_bevy_app
        ToolDef {
            name:            TOOL_LAUNCH_BEVY_APP,
            description:     DESC_LAUNCH_BEVY_APP,
            handler:         HandlerFn::local_with_port(
                crate::app_tools::brp_launch_bevy_app::create_launch_bevy_app_handler(),
            ),
            parameters:      vec![
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::AppName,
                    "Name of the Bevy app to launch",
                    true,
                )),
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                )),
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::Path,
                    "Path to use when multiple apps with the same name exist",
                    false,
                )),
            ],
            response_format: ResponseSpecification {
                message_template: "Launched Bevy app `{app_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        },
        // UnifiedToolDef/launch_bevy_example
        ToolDef {
            name:            TOOL_LAUNCH_BEVY_EXAMPLE,
            description:     DESC_LAUNCH_BEVY_EXAMPLE,
            handler:         HandlerFn::local_with_port(
                brp_launch_bevy_example::create_launch_bevy_example_handler(),
            ),
            parameters:      vec![
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::ExampleName,
                    "Name of the Bevy example to launch",
                    true,
                )),
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                )),
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::Path,
                    "Path to use when multiple examples with the same name exist",
                    false,
                )),
            ],
            response_format: ResponseSpecification {
                message_template: "Launched Bevy example `{example_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        },
        // UnifiedToolDef/list_bevy_apps
        ToolDef {
            name:            TOOL_LIST_BEVY_APPS,
            description:     DESC_LIST_BEVY_APPS,
            handler:         HandlerFn::local(ListBevyApps),
            parameters:      vec![],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/list_bevy_examples
        ToolDef {
            name:            TOOL_LIST_BEVY_EXAMPLES,
            description:     DESC_LIST_BEVY_EXAMPLES,
            handler:         HandlerFn::local(ListBevyExamples),
            parameters:      vec![],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/list_brp_apps
        ToolDef {
            name:            TOOL_LIST_BRP_APPS,
            description:     DESC_LIST_BRP_APPS,
            handler:         HandlerFn::local(ListBrpApps),
            parameters:      vec![],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/brp_list_active_watches
        ToolDef {
            name:            TOOL_LIST_ACTIVE_WATCHES,
            description:     DESC_LIST_ACTIVE_WATCHES,
            handler:         HandlerFn::local(BrpListActiveWatches),
            parameters:      vec![],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/brp_stop_watch
        ToolDef {
            name:            TOOL_STOP_WATCH,
            description:     DESC_STOP_WATCH,
            handler:         HandlerFn::local(BrpStopWatch),
            parameters:      vec![UnifiedParameter::Local(LocalParameter::number(
                LocalParameterName::WatchId,
                "The watch ID returned from bevy_start_entity_watch or bevy_start_list_watch",
                true,
            ))],
            response_format: ResponseSpecification {
                message_template: "Successfully stopped watch",
                response_fields:  vec![],
            },
        },
        // LocalToolDef/list_logs
        ToolDef {
            name:            TOOL_LIST_LOGS,
            description:     DESC_LIST_LOGS,
            handler:         HandlerFn::local(ListLogs),
            parameters:      vec![
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::AppName,
                    "Optional filter to list logs for a specific app only",
                    false,
                )),
                UnifiedParameter::Local(LocalParameter::boolean(
                    LocalParameterName::Verbose,
                    "Include full details (path, timestamps, size in bytes). Default is false for minimal output",
                    false,
                )),
            ],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/read_log
        ToolDef {
            name:            TOOL_READ_LOG,
            description:     DESC_READ_LOG,
            handler:         HandlerFn::local(ReadLog),
            parameters:      vec![
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::Filename,
                    "The log filename (e.g., bevy_brp_mcp_myapp_1234567890.log)",
                    true,
                )),
                UnifiedParameter::Local(LocalParameter::string(
                    LocalParameterName::Keyword,
                    "Optional keyword to filter lines (case-insensitive)",
                    false,
                )),
                UnifiedParameter::Local(LocalParameter::number(
                    LocalParameterName::TailLines,
                    "Optional number of lines to read from the end of file",
                    false,
                )),
            ],
            response_format: ResponseSpecification {
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
        },
        // UnifiedToolDef/set_tracing_level
        ToolDef {
            name:            TOOL_SET_TRACING_LEVEL,
            description:     DESC_SET_TRACING_LEVEL,
            handler:         HandlerFn::local(SetTracingLevel),
            parameters:      vec![UnifiedParameter::Local(LocalParameter::string(
                LocalParameterName::Level,
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            ))],
            response_format: ResponseSpecification {
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
        },
        // LocalToolDef/status
        ToolDef {
            name:            TOOL_STATUS,
            description:     DESC_STATUS,
            handler:         HandlerFn::local_with_port(Status),
            parameters:      vec![UnifiedParameter::Local(LocalParameter::string(
                LocalParameterName::AppName,
                "Name of the process to check for",
                true,
            ))],
            response_format: ResponseSpecification {
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
        },
        // LocalToolDef/shutdown
        ToolDef {
            name:            TOOL_SHUTDOWN,
            description:     DESC_SHUTDOWN,
            handler:         HandlerFn::local_with_port(Shutdown),
            parameters:      vec![UnifiedParameter::Local(LocalParameter::string(
                LocalParameterName::AppName,
                "Name of the Bevy app to shutdown",
                true,
            ))],
            response_format: ResponseSpecification {
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
        },
    ]
}
