//! Declarative tool definitions for BRP and local MCP tools.
//!
//! Defines tools as data structures with parameters, extractors, and response formatting.
//! Eliminates code duplication through declarative configuration.
//!
//! # Handler Types
//!
//! - **`HandlerType::Brp`**: Remote BRP method calls
//! - **`HandlerType::Local`**: Local functions within MCP server
//!
//! # Key Formatter Types
//!
//! - **`LocalPassthrough`**: Preserves pre-structured responses (status, control operations)
//! - **`LocalStandard`**: Standard formatting for simple operations
//! - **`EntityOperation`/`ResourceOperation`**: BRP operations with field extraction

use super::HandlerType;
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
    DESC_BRP_LIST_ACTIVE_WATCHES, DESC_BRP_STOP_WATCH, DESC_CLEANUP_LOGS, DESC_GET_TRACE_LOG_PATH,
    DESC_LAUNCH_BEVY_APP, DESC_LAUNCH_BEVY_EXAMPLE, DESC_LIST_BEVY_APPS, DESC_LIST_BEVY_EXAMPLES,
    DESC_LIST_BRP_APPS, DESC_LIST_LOGS, DESC_READ_LOG, DESC_SET_TRACING_LEVEL, DESC_SHUTDOWN,
    DESC_STATUS, TOOL_BEVY_DESTROY, TOOL_BEVY_GET, TOOL_BEVY_GET_RESOURCE, TOOL_BEVY_GET_WATCH,
    TOOL_BEVY_INSERT, TOOL_BEVY_INSERT_RESOURCE, TOOL_BEVY_LIST, TOOL_BEVY_LIST_RESOURCES,
    TOOL_BEVY_LIST_WATCH, TOOL_BEVY_MUTATE_COMPONENT, TOOL_BEVY_MUTATE_RESOURCE, TOOL_BEVY_QUERY,
    TOOL_BEVY_REGISTRY_SCHEMA, TOOL_BEVY_REMOVE, TOOL_BEVY_REMOVE_RESOURCE, TOOL_BEVY_REPARENT,
    TOOL_BEVY_RPC_DISCOVER, TOOL_BEVY_SPAWN, TOOL_BRP_EXECUTE, TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
    TOOL_BRP_EXTRAS_SCREENSHOT, TOOL_BRP_EXTRAS_SEND_KEYS, TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
    TOOL_BRP_LIST_ACTIVE_WATCHES, TOOL_BRP_STOP_WATCH, TOOL_CLEANUP_LOGS, TOOL_GET_TRACE_LOG_PATH,
    TOOL_LAUNCH_BEVY_APP, TOOL_LAUNCH_BEVY_EXAMPLE, TOOL_LIST_BEVY_APPS, TOOL_LIST_BEVY_EXAMPLES,
    TOOL_LIST_BRP_APPS, TOOL_LIST_LOGS, TOOL_READ_LOG, TOOL_SET_TRACING_LEVEL, TOOL_SHUTDOWN,
    TOOL_STATUS,
};
use super::parameters::Parameter;
use crate::app_tools::brp_status::Status;
use crate::constants::{
    JSON_FIELD_APP_NAME, JSON_FIELD_COMPONENT_COUNT, JSON_FIELD_COMPONENTS, JSON_FIELD_COUNT,
    JSON_FIELD_ENTITIES, JSON_FIELD_ENTITY, JSON_FIELD_ENTITY_COUNT, JSON_FIELD_LOG_PATH,
    JSON_FIELD_PARENT, JSON_FIELD_PATH, JSON_FIELD_PORT, JSON_FIELD_RESOURCE, PARAM_APP_NAME,
    PARAM_COMPONENT, PARAM_COMPONENTS, PARAM_DATA, PARAM_ENTITIES, PARAM_ENTITY, PARAM_FILTER,
    PARAM_PARAMS, PARAM_PARENT, PARAM_PATH, PARAM_PORT, PARAM_RESOURCE, PARAM_TYPES,
    PARAM_WITH_CRATES, PARAM_WITH_TYPES, PARAM_WITHOUT_CRATES, PARAM_WITHOUT_TYPES,
};
use crate::log_tools::get_trace_log_path::GetTraceLogPath;
use crate::log_tools::list_logs::ListLogs;
use crate::response::{
    FieldPlacement, ResponseExtractorType, ResponseField, ResponseSpecification,
};

/// Complete definition of a BRP tool
#[derive(Clone)]
pub struct McpToolDef {
    /// Tool name (e.g., "`bevy_destroy`")
    pub name:                &'static str,
    /// Tool description
    pub description:         &'static str,
    /// Handler type (BRP or Local)
    pub handler:             HandlerType,
    /// Parameters for the tool
    pub parameters:          Vec<Parameter>,
    /// Parameter extractor type
    pub parameter_extractor: BrpMethodParamCategory,
    /// Response formatter definition
    pub formatter:           ResponseSpecification,
}

/// Type of parameter extractor to use
#[derive(Clone)]
pub enum BrpMethodParamCategory {
    /// Pass through all parameters
    Passthrough,
    /// Extract entity parameter
    Entity { required: bool },
    /// Extract resource parameter
    Resource,
    /// Extract empty params
    EmptyParams,
    /// Custom extractor for BRP execute (dynamic method)
    BrpExecute,
    /// Custom extractor for registry schema (parameter transformation)
    RegistrySchema,
}

/// Get all standard tool definitions
#[allow(clippy::too_many_lines)]
fn get_standard_tools() -> Vec<McpToolDef> {
    vec![
        // bevy_destroy
        McpToolDef {
            name:                TOOL_BEVY_DESTROY,
            description:         DESC_BEVY_DESTROY,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_DESTROY,
            },
            parameters:          [
                Parameter::entity("The entity ID to destroy", true),
                Parameter::port(),
            ]
            .to_vec(),
            parameter_extractor: BrpMethodParamCategory::Entity { required: true },
            formatter:           ResponseSpecification {
                message_template:        "Successfully destroyed entity {entity}",
                response_fields: vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: JSON_FIELD_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // bevy_get
        McpToolDef {
            name:                TOOL_BEVY_GET,
            description:         DESC_BEVY_GET,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_GET,
            },
            parameters:         [
                Parameter::entity("The entity ID to get component data from", true),
                Parameter::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                ),
                Parameter::port(),
            ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Retrieved component data from entity {entity}",
                response_fields: vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: PARAM_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COMPONENTS,
                        response_extractor:           ResponseExtractorType::Field(JSON_FIELD_COMPONENTS),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // bevy_list
        McpToolDef {
            name:                TOOL_BEVY_LIST,
            description:         DESC_BEVY_LIST,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_LIST,
            },
            parameters:          [
                Parameter::entity("Optional entity ID to list components for", false),
                Parameter::port(),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Entity { required: false },
            formatter:           ResponseSpecification {
                message_template:        "Listed {count} components",
                response_fields: vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:           ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // bevy_remove
        McpToolDef {
            name:                TOOL_BEVY_REMOVE,
            description:         DESC_BEVY_REMOVE,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_REMOVE,
            },
            parameters:          vec![
                Parameter::entity("The entity ID to remove components from", true),
                Parameter::components("Array of component type names to remove", true),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully removed components from entity {entity}",
                response_fields: vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: PARAM_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_COMPONENTS,
                        parameter_field_name: PARAM_COMPONENTS,
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // bevy_insert
        McpToolDef {
            name:                TOOL_BEVY_INSERT,
            description:         DESC_BEVY_INSERT,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_INSERT,
            },
            parameters:          vec![
                Parameter::entity("The entity ID to insert components into", true),
                Parameter::components(
                    "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully inserted components into entity {entity}",
                response_fields: vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: PARAM_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_COMPONENTS,
                        parameter_field_name: PARAM_COMPONENTS,
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // bevy_get_resource
        McpToolDef {
            name:                TOOL_BEVY_GET_RESOURCE,
            description:         DESC_BEVY_GET_RESOURCE,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_GET_RESOURCE,
            },
            parameters: vec![Parameter::resource( "The fully-qualified type name of the resource to get"), Parameter::port()],
            parameter_extractor: BrpMethodParamCategory::Resource,
            formatter:           ResponseSpecification {
                message_template: "Retrieved resource: {resource}",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // bevy_insert_resource
        McpToolDef {
            name:                TOOL_BEVY_INSERT_RESOURCE,
            description:         DESC_BEVY_INSERT_RESOURCE,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_INSERT_RESOURCE,
            },
            parameters:          vec![
                Parameter::resource(
                    "The fully-qualified type name of the resource to insert or update",
                ),
                Parameter::value(
                    "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully inserted/updated resource: {resource}",
                response_fields: vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // bevy_remove_resource
        McpToolDef {
            name:                TOOL_BEVY_REMOVE_RESOURCE,
            description:         DESC_BEVY_REMOVE_RESOURCE,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_REMOVE_RESOURCE,
            },
            parameters:  [Parameter::resource( "The fully-qualified type name of the resource to remove"), Parameter::port()].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Resource,
            formatter:           ResponseSpecification {
                message_template:        "Successfully removed resource",
                response_fields: vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // bevy_mutate_component
        McpToolDef {
            name:                TOOL_BEVY_MUTATE_COMPONENT,
            description:         DESC_BEVY_MUTATE_COMPONENT,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_MUTATE_COMPONENT,
            },
            parameters:          vec![
                Parameter::entity("The entity ID containing the component to mutate", true),
                Parameter::string(
                    PARAM_COMPONENT,
                    "The fully-qualified type name of the component to mutate",
                    true,
                ),
                Parameter::path(
                    "The path to the field within the component (e.g., 'translation.x')",
                ),
                Parameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully mutated component on entity {entity}",
                response_fields: vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: PARAM_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // bevy_mutate_resource
        McpToolDef {
            name:                TOOL_BEVY_MUTATE_RESOURCE,
            description:         DESC_BEVY_MUTATE_RESOURCE,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_MUTATE_RESOURCE,
            },
            parameters:           [
                Parameter::resource("The fully-qualified type name of the resource to mutate"),
                Parameter::path("The path to the field within the resource (e.g., 'settings.volume')"),
                Parameter::value(
                    "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                Parameter::port(),
            ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully mutated resource: `{resource}`",
                response_fields: vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_RESOURCE,
                    parameter_field_name: PARAM_RESOURCE,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        },
        // bevy_list_resources
        McpToolDef {
            name:                TOOL_BEVY_LIST_RESOURCES,
            description:         DESC_BEVY_LIST_RESOURCES,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_LIST_RESOURCES,
            },
            parameters:          vec![Parameter::port()],
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template:        "Listed {count} resources",
                response_fields: vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:           ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // bevy_rpc_discover
        McpToolDef {
            name:                TOOL_BEVY_RPC_DISCOVER,
            description:         DESC_BEVY_RPC_DISCOVER,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_RPC_DISCOVER,
            },
            parameters:          vec![Parameter::port()],
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template: "Retrieved BRP method discovery information",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // bevy_brp_extras/discover_format
        McpToolDef {
            name:                TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
            description:         DESC_BRP_EXTRAS_DISCOVER_FORMAT,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
            },
            parameters:          vec![
                Parameter::string_array(
                    PARAM_TYPES,
                    "Array of fully-qualified component type names to discover formats for",
                    true,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Format discovery completed",
                response_fields: vec![ResponseField::DirectToResult],
            },
        },
        // bevy_screenshot
        McpToolDef {
            name:                TOOL_BRP_EXTRAS_SCREENSHOT,
            description:         DESC_BRP_EXTRAS_SCREENSHOT,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SCREENSHOT,
            },
            parameters:          vec![
                Parameter::path("File path where the screenshot should be saved"),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully captured screenshot",
                response_fields: vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_PATH,
                        parameter_field_name: PARAM_PATH,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_PORT,
                        parameter_field_name: PARAM_PORT,
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // brp_extras/send_keys
        McpToolDef {
            name:                TOOL_BRP_EXTRAS_SEND_KEYS,
            description:         DESC_BRP_EXTRAS_SEND_KEYS,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SEND_KEYS,
            },
            parameters:          vec![
                Parameter::string_array("keys", "Array of key code names to send", true),
                Parameter::number(
                    "duration_ms",
                    "Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1 minute)",
                    false,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully sent keyboard input",
                response_fields: vec![
                    ResponseField::FromResponse {
                        response_field_name: "keys_sent",
                        response_extractor:           ResponseExtractorType::Field("keys_sent"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "duration_ms",
                        response_extractor:           ResponseExtractorType::Field("duration_ms"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // brp_extras/set_debug_mode
        McpToolDef {
            name:                TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
            description:         DESC_BRP_EXTRAS_SET_DEBUG_MODE,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
            },
            parameters:          vec![
                Parameter::boolean(
                    "enabled",
                    "Enable or disable debug mode for bevy_brp_extras plugin",
                    true,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template: "Debug mode updated successfully",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "debug_enabled",
                        response_extractor:           ResponseExtractorType::Field("debug_enabled"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "details",
                        response_extractor:           ResponseExtractorType::Field("message"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
    ]
}

/// Get tool definitions for tools with special variations
#[allow(clippy::too_many_lines)]
fn get_special_tools() -> Vec<McpToolDef> {
    vec![
        // bevy_query - has custom extractors for component counts
        McpToolDef {
            name:                TOOL_BEVY_QUERY,
            description:         DESC_BEVY_QUERY,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_QUERY,
            },
            parameters:          vec![
                Parameter::any(
                    PARAM_DATA,
                    "Object specifying what component data to retrieve. Properties: components (array), option (array), has (array)",
                    true,
                ),
                Parameter::any(
                    PARAM_FILTER,
                    "Object specifying which entities to query. Properties: with (array), without (array)",
                    true,
                ),
                Parameter::strict(),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Query completed successfully",
                response_fields: vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_ENTITY_COUNT,
                        response_extractor:           ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COMPONENT_COUNT,
                        response_extractor:           ResponseExtractorType::QueryComponentCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // bevy_spawn - has dynamic entity extraction from response
        McpToolDef {
            name:                TOOL_BEVY_SPAWN,
            description:         DESC_BEVY_SPAWN,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_SPAWN,
            },
            parameters:          vec![
                Parameter::components(
                    "Object containing component data to spawn with. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    false,
                ),
                Parameter::port(),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully spawned entity",
                response_fields: vec![
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_ENTITY,
                        response_extractor:           ResponseExtractorType::Field(JSON_FIELD_ENTITY),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_COMPONENTS,
                        parameter_field_name: PARAM_COMPONENTS,
                        placement:            FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // brp_execute - has dynamic method selection
        McpToolDef {
            name:                TOOL_BRP_EXECUTE,
            description:         DESC_BRP_EXECUTE,
            handler:             HandlerType::Brp { method: "" }, // Dynamic method
            parameters:          [
                Parameter::method(),
                Parameter::any(
                    PARAM_PARAMS,
                    "Optional parameters for the method, as a JSON object or array",
                    false,
                ),
                Parameter::port(),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::BrpExecute,
            formatter:           ResponseSpecification {
                message_template: "Method executed successfully",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // bevy_registry_schema - has complex parameter transformation
        McpToolDef {
            name:                TOOL_BEVY_REGISTRY_SCHEMA,
            description:         DESC_BEVY_REGISTRY_SCHEMA,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_REGISTRY_SCHEMA,
            },
            parameters:          [
                Parameter::string_array(
                    PARAM_WITH_CRATES,
                    "Include only types from these crates (e.g., [\"bevy_transform\", \"my_game\"])",
                    false,
                ),
                Parameter::string_array(
                    PARAM_WITHOUT_CRATES,
                    "Exclude types from these crates (e.g., [\"bevy_render\", \"bevy_pbr\"])",
                    false,
                ),
                Parameter::string_array(
                    PARAM_WITH_TYPES,
                    "Include only types with these reflect traits (e.g., [\"Component\", \"Resource\"])",
                    false,
                ),
                Parameter::string_array(
                    PARAM_WITHOUT_TYPES,
                    "Exclude types with these reflect traits (e.g., [\"RenderResource\"])",
                    false,
                ),
                Parameter::port(),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::RegistrySchema,
            formatter:           ResponseSpecification {
                message_template: "Retrieved schema information",
                response_fields:  vec![ResponseField::DirectToResult],
            },
        },
        // bevy_reparent - has array parameter handling
        McpToolDef {
            name:                TOOL_BEVY_REPARENT,
            description:         DESC_BEVY_REPARENT,
            handler:             HandlerType::Brp {
                method: BRP_METHOD_REPARENT,
            },
            parameters:          [
                Parameter::any(PARAM_ENTITIES, "Array of entity IDs to reparent", true),
                Parameter::number(
                    PARAM_PARENT,
                    "The new parent entity ID (omit to remove parent)",
                    false,
                ),
                Parameter::port(),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully reparented entities",
                response_fields: vec![
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
    ]
}

/// Get log tool definitions
#[allow(clippy::too_many_lines)]
fn get_log_tools() -> Vec<McpToolDef> {
    vec![
        // list_logs
        McpToolDef {
            name:                TOOL_LIST_LOGS,
            description:         DESC_LIST_LOGS,
            handler:             HandlerType::Local {
                handler: Box::new(ListLogs),
            },
            parameters:          [
                Parameter::string(
                    "app_name",
                    "Optional filter to list logs for a specific app only",
                    false,
                ),
                Parameter::boolean(
                    "verbose",
                    "Include full details (path, timestamps, size in bytes). Default is false for minimal output",
                    false,
                ),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Found {count} log files",
                response_fields: vec![
                    ResponseField::FromResponse {
                        response_field_name: "logs",
                        response_extractor:           ResponseExtractorType::Field("logs"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "temp_directory",
                        response_extractor:           ResponseExtractorType::Field("temp_directory"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:           ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // read_log
        McpToolDef {
            name:                TOOL_READ_LOG,
            description:         DESC_READ_LOG,
            handler:             HandlerType::Local {
                handler: Box::new(crate::log_tools::read_log::ReadLog),
            },
            parameters:          [
                Parameter::string(
                    "filename",
                    "The log filename (e.g., bevy_brp_mcp_myapp_1234567890.log)",
                    true,
                ),
                Parameter::string(
                    "keyword",
                    "Optional keyword to filter lines (case-insensitive)",
                    false,
                ),
                Parameter::number(
                    "tail_lines",
                    "Optional number of lines to read from the end of file",
                    false,
                ),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Successfully read log file: {filename}",
                response_fields: [
                    ResponseField::FromResponse {
                        response_field_name: "filename",
                        response_extractor:           ResponseExtractorType::Field("filename"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "file_path",
                        response_extractor:           ResponseExtractorType::Field("file_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "size_bytes",
                        response_extractor:           ResponseExtractorType::Field("size_bytes"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "size_human",
                        response_extractor:           ResponseExtractorType::Field("size_human"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "lines_read",
                        response_extractor:           ResponseExtractorType::Field("lines_read"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "content",
                        response_extractor:           ResponseExtractorType::SplitContentIntoLines,
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "filtered_by_keyword",
                        response_extractor:           ResponseExtractorType::Field("filtered_by_keyword"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "tail_mode",
                        response_extractor:           ResponseExtractorType::Field("tail_mode"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ].to_vec(),
            },
        },
        // cleanup_logs
        McpToolDef {
            name:                TOOL_CLEANUP_LOGS,
            description:         DESC_CLEANUP_LOGS,
            handler:             HandlerType::Local {
                handler: Box::new(crate::log_tools::cleanup_logs::CleanupLogs),
            },
            parameters:          vec![
                Parameter::string(
                    "app_name",
                    "Optional filter to delete logs for a specific app only",
                    false,
                ),
                Parameter::number(
                    "older_than_seconds",
                    "Optional filter to delete logs older than N seconds",
                    false,
                ),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Deleted {deleted_count} log files",
                response_fields: vec![
                    ResponseField::FromResponse {
                        response_field_name: "deleted_count",
                        response_extractor:           ResponseExtractorType::Field("deleted_count"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "deleted_files",
                        response_extractor:           ResponseExtractorType::Field("deleted_files"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "app_name_filter",
                        response_extractor:           ResponseExtractorType::Field("app_name_filter"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "older_than_seconds",
                        response_extractor:           ResponseExtractorType::Field("older_than_seconds"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // brp_get_trace_log_path
        McpToolDef {
            name:                TOOL_GET_TRACE_LOG_PATH,
            description:         DESC_GET_TRACE_LOG_PATH,
            handler:             HandlerType::Local {
                handler: Box::new(GetTraceLogPath),
            },
            parameters:          [].to_vec(),
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template:        "Trace log found",
                response_fields: vec![
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_LOG_PATH,
                        response_extractor:           ResponseExtractorType::Field(JSON_FIELD_LOG_PATH),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "exists",
                        response_extractor:           ResponseExtractorType::Field("exists"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "file_size_bytes",
                        response_extractor:           ResponseExtractorType::Field("file_size_bytes"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // brp_set_tracing_level
        McpToolDef {
            name:                TOOL_SET_TRACING_LEVEL,
            description:         DESC_SET_TRACING_LEVEL,
            handler:             HandlerType::Local {
                handler: Box::new(crate::log_tools::set_tracing_level::SetTracingLevel),
            },
            parameters:          vec![Parameter::string(
                "level",
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            )],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Tracing level set to '{level}' - diagnostic information will be logged to temp directory",
                response_fields: vec![
                    ResponseField::FromResponse {
                        response_field_name: "tracing_level",
                        response_extractor:           ResponseExtractorType::Field("level"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "log_file",
                        response_extractor:           ResponseExtractorType::Field("log_file"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
    ]
}

/// Get app tool definitions
#[allow(clippy::too_many_lines)]
fn get_app_tools() -> Vec<McpToolDef> {
    vec![
        // list_bevy_apps
        McpToolDef {
            name:                TOOL_LIST_BEVY_APPS,
            description:         DESC_LIST_BEVY_APPS,
            handler:             HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_list_bevy_apps::ListBevyApps),
            },
            parameters:          [].to_vec(),
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template: "Found {count} Bevy apps",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "apps",
                        response_extractor:  ResponseExtractorType::Field("apps"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "count",
                        response_extractor:  ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // list_brp_apps
        McpToolDef {
            name:                TOOL_LIST_BRP_APPS,
            description:         DESC_LIST_BRP_APPS,
            handler:             HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_list_brp_apps::ListBrpApps),
            },
            parameters:          [].to_vec(),
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template: "Found {count} BRP-enabled apps",
                response_fields:  [
                    ResponseField::FromResponse {
                        response_field_name: "apps",
                        response_extractor:  ResponseExtractorType::Field("apps"),
                        placement:           FieldPlacement::Result,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COUNT,
                        response_extractor:  ResponseExtractorType::Count,
                        placement:           FieldPlacement::Metadata,
                    },
                ]
                .to_vec(),
            },
        },
        // list_bevy_examples
        McpToolDef {
            name:                TOOL_LIST_BEVY_EXAMPLES,
            description:         DESC_LIST_BEVY_EXAMPLES,
            handler:             HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_list_bevy_examples::ListBevyExamples),
            },
            parameters:          [].to_vec(),
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template: "Found {count} Bevy examples",
                response_fields:  [
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
                ]
                .to_vec(),
            },
        },
        // launch_bevy_app
        McpToolDef {
            name:                TOOL_LAUNCH_BEVY_APP,
            description:         DESC_LAUNCH_BEVY_APP,
            handler:             HandlerType::Local {
                handler: Box::new(
                    crate::app_tools::brp_launch_bevy_app::create_launch_bevy_app_handler(),
                ),
            },
            parameters:          create_launch_params("app_name", "Name of the Bevy app to launch"),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template: "Launched Bevy app `{app_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        },
        // launch_bevy_example
        McpToolDef {
            name:                TOOL_LAUNCH_BEVY_EXAMPLE,
            description:         DESC_LAUNCH_BEVY_EXAMPLE,
            handler:             HandlerType::Local {
                handler: Box::new(
                    crate::app_tools::brp_launch_bevy_example::create_launch_bevy_example_handler(),
                ),
            },
            parameters:          create_launch_params(
                "example_name",
                "Name of the Bevy example to launch",
            ),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template: "Launched Bevy example `{example_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        },
        // brp_extras_shutdown
        McpToolDef {
            name:                TOOL_SHUTDOWN,
            description:         DESC_SHUTDOWN,
            handler:             HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_shutdown::Shutdown),
            },
            parameters:          [
                Parameter::string("app_name", "Name of the Bevy app to shutdown", true),
                Parameter::number(
                    JSON_FIELD_PORT,
                    "BRP port to connect to (default: 15702)",
                    false,
                ),
            ]
            .to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template: "{shutdown_message}",
                response_fields:  vec![
                    ResponseField::FromResponse {
                        response_field_name: "method",
                        response_extractor:  ResponseExtractorType::Field("shutdown_method"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "app_name",
                        response_extractor:  ResponseExtractorType::Field("app_name"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: "port",
                        response_extractor:  ResponseExtractorType::Field("port"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponseNullableWithPlacement {
                        response_field_name: "pid",
                        response_extractor:  ResponseExtractorType::Field("pid"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
        // brp_status
        McpToolDef {
            name:                TOOL_STATUS,
            description:         DESC_STATUS,
            handler:             HandlerType::Local {
                handler: Box::new(Status),
            },
            parameters:          vec![
                Parameter::string("app_name", "Name of the process to check for", true),
                Parameter::number(
                    JSON_FIELD_PORT,
                    "Port to check for BRP (default: 15702)",
                    false,
                ),
            ],
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template: "Status check for `{app_name}` on port {port}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_APP_NAME,
                        parameter_field_name: PARAM_APP_NAME,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_PORT,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_PORT),
                        placement:           FieldPlacement::Metadata,
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
                        response_field_name: "app_pid",
                        response_extractor:  ResponseExtractorType::Field("app_pid"),
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        },
    ]
}

/// Get watch tool definitions
fn get_watch_tools() -> Vec<McpToolDef> {
    vec![
        // bevy_get_watch
        McpToolDef {
            name:                TOOL_BEVY_GET_WATCH,
            description:         DESC_BEVY_GET_WATCH,
            handler:             HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::bevy_get_watch::BevyGetWatch),
            },
            parameters:          [
                Parameter::entity("The entity ID to watch for component changes", true),
                Parameter::components(
                    "Required array of component types to watch. Must contain at least one component. Without this, the watch will not detect any changes.",
                    true,
                ),
                Parameter::port(),
                ].to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:"Started entity watch for entity {entity}",
                response_fields: [
                    ResponseField::FromResponse {
                        response_field_name: "watch_id",
                        response_extractor:           ResponseExtractorType::Field("watch_id"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_LOG_PATH,
                        response_extractor:           ResponseExtractorType::Field("log_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: PARAM_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ].to_vec(),
            },
        },
        // bevy_list_watch
        McpToolDef {
            name:                TOOL_BEVY_LIST_WATCH,
            description:         DESC_BEVY_LIST_WATCH,
            handler:             HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::bevy_list_watch::BevyListWatch),
            },
            parameters:          [
                Parameter::entity("The entity ID to watch for component list changes", true),
                Parameter::port(),
            ]
            .to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template:        "Started list watch for entity {entity}",
                response_fields: [
                    ResponseField::FromResponse {
                        response_field_name: "watch_id",
                        response_extractor:           ResponseExtractorType::Field("watch_id"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_LOG_PATH,
                        response_extractor:           ResponseExtractorType::Field("log_path"),
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: JSON_FIELD_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ].to_vec(),
            },
        },
    ]
}

/// Get BRP watch management tool definitions
fn get_brp_tools() -> Vec<McpToolDef> {
    vec![
        // brp_stop_watch
        McpToolDef {
            name:                TOOL_BRP_STOP_WATCH,
            description:         DESC_BRP_STOP_WATCH,
            handler:             HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::brp_stop_watch::BrpStopWatch),
            },
            parameters:          [Parameter::number(
                "watch_id",
                "The watch ID returned from bevy_start_entity_watch or bevy_start_list_watch",
                true,
            )]
            .to_vec(),
            parameter_extractor: BrpMethodParamCategory::Passthrough,
            formatter:           ResponseSpecification {
                message_template: "Successfully stopped watch",
                response_fields:  [].to_vec(),
            },
        },
        // brp_list_active_watches
        McpToolDef {
            name:                TOOL_BRP_LIST_ACTIVE_WATCHES,
            description:         DESC_BRP_LIST_ACTIVE_WATCHES,
            handler:             HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::brp_list_active::BrpListActiveWatches),
            },
            parameters:          [].to_vec(),
            parameter_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:           ResponseSpecification {
                message_template: "Found {count} active watches",
                response_fields:  [
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
                ]
                .to_vec(),
            },
        },
    ]
}

/// Create standard launch tool parameters (profile, path, port)
fn create_launch_params(name_param: &'static str, name_desc: &'static str) -> Vec<Parameter> {
    [
        Parameter::string(name_param, name_desc, true),
        Parameter::string("profile", "Build profile to use (debug or release)", false),
        Parameter::string(
            PARAM_PATH,
            "Path to use when multiple apps/examples with the same name exist",
            false,
        ),
        Parameter::number(JSON_FIELD_PORT, "BRP port to use (default: 15702)", false),
    ]
    .to_vec()
}

/// Get all tool definitions - combines standard, special, log, and app tools
pub fn get_all_tool_definitions() -> Vec<McpToolDef> {
    let mut tools = Vec::new();

    // Add standard tools
    tools.extend(get_standard_tools());

    // Add special tools
    tools.extend(get_special_tools());

    // Add log tools
    tools.extend(get_log_tools());

    // Add app tools
    tools.extend(get_app_tools());

    // Add watch tools
    tools.extend(get_watch_tools());

    // Add BRP watch management tools
    tools.extend(get_brp_tools());

    tools
}
