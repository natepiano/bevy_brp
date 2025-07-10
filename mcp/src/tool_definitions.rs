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

use crate::brp_tools::constants::{
    DESC_PORT, JSON_FIELD_COMPONENT, JSON_FIELD_COMPONENTS, JSON_FIELD_COUNT, JSON_FIELD_DATA,
    JSON_FIELD_DESTROYED_ENTITY, JSON_FIELD_ENTITY, JSON_FIELD_LOG_PATH, JSON_FIELD_PATH,
    JSON_FIELD_PORT, JSON_FIELD_RESOURCE, JSON_FIELD_RESOURCES, JSON_FIELD_VALUE,
    PARAM_COMPONENT_COUNT, PARAM_DATA, PARAM_ENTITIES, PARAM_ENTITY_COUNT, PARAM_FILTER,
    PARAM_FORMATS, PARAM_METHOD, PARAM_PARAMS, PARAM_PARENT, PARAM_QUERY_PARAMS, PARAM_RESULT,
    PARAM_SPAWNED_ENTITY, PARAM_STRICT, PARAM_TYPES, PARAM_WITH_CRATES, PARAM_WITH_TYPES,
    PARAM_WITHOUT_CRATES, PARAM_WITHOUT_TYPES,
};
use crate::constants::PARAM_BINARY_PATH;
use crate::extractors::ExtractorType;
use crate::handler::HandlerType;
use crate::log_tools::get_trace_log_path::GetTraceLogPath;
use crate::log_tools::list_logs::ListLogs;
use crate::response::{
    FormatterType, ResponseExtractorType, ResponseField, ResponseFieldCompat, ResponseFieldV2,
    ResponseSpecification,
};
use crate::tools::{
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

/// Represents a parameter definition for a BRP tool
#[derive(Clone)]
pub struct ParamDef {
    /// Parameter name as it appears in the API
    pub name:        &'static str,
    /// Description of the parameter
    pub description: &'static str,
    /// Whether this parameter is required
    pub required:    bool,
    /// Type of the parameter
    pub param_type:  ParamType,
}

impl ParamDef {
    /// Standard port parameter (appears in 21+ tools)
    pub const fn port() -> Self {
        Self {
            name:        JSON_FIELD_PORT,
            description: DESC_PORT,
            required:    false,
            param_type:  ParamType::Number,
        }
    }

    /// Entity ID parameter with custom description
    pub const fn entity(description: &'static str, required: bool) -> Self {
        Self {
            name: JSON_FIELD_ENTITY,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Resource name parameter
    pub const fn resource(description: &'static str) -> Self {
        Self {
            name: JSON_FIELD_RESOURCE,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Components parameter
    pub const fn components(description: &'static str, required: bool) -> Self {
        Self {
            name: JSON_FIELD_COMPONENTS,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Path parameter for mutations
    pub const fn path(description: &'static str) -> Self {
        Self {
            name: JSON_FIELD_PATH,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Value parameter
    pub const fn value(description: &'static str, required: bool) -> Self {
        Self {
            name: JSON_FIELD_VALUE,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Entity + port (used in destroy, get, insert, remove, `mutate_component`)
    pub const fn entity_with_port(entity_desc: &'static str) -> [Self; 2] {
        [Self::entity(entity_desc, true), Self::port()]
    }

    /// Resource + port (used in `get_resource`, `insert_resource`, `remove_resource`)
    pub const fn resource_with_port(resource_desc: &'static str) -> [Self; 2] {
        [Self::resource(resource_desc), Self::port()]
    }

    /// Resource + path + value + port (used in `mutate_resource`)
    pub const fn resource_mutation_params() -> [Self; 4] {
        [
            Self::resource("The fully-qualified type name of the resource to mutate"),
            Self::path("The path to the field within the resource (e.g., 'settings.volume')"),
            Self::value(
                "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                true,
            ),
            Self::port(),
        ]
    }

    /// Boolean parameter helper (for enabled, strict, etc.)
    pub const fn boolean(name: &'static str, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Boolean,
        }
    }

    /// Generic string parameter
    pub const fn string(name: &'static str, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::String,
        }
    }

    /// String array parameter (for keys, filters, etc.)
    pub const fn string_array(
        name: &'static str,
        description: &'static str,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::StringArray,
        }
    }

    /// Generic number parameter (for `duration_ms`, etc.)
    pub const fn number(name: &'static str, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Any type parameter (for flexible data)
    pub const fn any(name: &'static str, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Method parameter (used in `brp_execute`)
    pub const fn method() -> Self {
        Self::string(
            PARAM_METHOD,
            "The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')",
            true,
        )
    }

    /// Optional params parameter (used in `brp_execute`)
    pub const fn optional_params() -> Self {
        Self::any(
            PARAM_PARAMS,
            "Optional parameters for the method, as a JSON object or array",
            false,
        )
    }

    /// Strict parameter (used in query)
    pub const fn strict() -> Self {
        Self::boolean(
            PARAM_STRICT,
            "If true, returns error on unknown component types (default: false)",
            false,
        )
    }

    /// Entity + component + path + value + port (for `mutate_component`)
    pub const fn component_mutation_params() -> [Self; 5] {
        [
            Self::entity("The entity ID containing the component to mutate", true),
            Self::string(
                JSON_FIELD_COMPONENT,
                "The fully-qualified type name of the component to mutate",
                true,
            ),
            Self::path("The path to the field within the component (e.g., 'translation.x')"),
            Self::value(
                "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                true,
            ),
            Self::port(),
        ]
    }

    /// Data + filter + strict + port (for query)
    pub const fn query_params() -> [Self; 4] {
        [
            Self::any(
                PARAM_DATA,
                "Object specifying what component data to retrieve. Properties: components (array), option (array), has (array)",
                true,
            ),
            Self::any(
                PARAM_FILTER,
                "Object specifying which entities to query. Properties: with (array), without (array)",
                true,
            ),
            Self::strict(),
            Self::port(),
        ]
    }
}

/// Types of parameters that can be defined
#[derive(Clone)]
pub enum ParamType {
    /// A numeric parameter (typically entity IDs or ports)
    Number,
    /// A string parameter
    String,
    /// A boolean parameter
    Boolean,
    /// An array of strings
    StringArray,
    /// Any JSON value (object, array, etc.)
    Any,
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

/// Complete definition of a BRP tool
pub struct McpToolDef {
    /// Tool name (e.g., "`bevy_destroy`")
    pub name:            &'static str,
    /// Tool description
    pub description:     &'static str,
    /// Handler type (BRP or Local)
    pub handler:         HandlerType,
    /// Parameters for the tool
    pub params:          Vec<ParamDef>,
    /// Parameter extractor type
    pub param_extractor: BrpMethodParamCategory,
    /// Response formatter definition
    pub formatter:       ResponseSpecification,
}

/// Get all standard tool definitions
#[allow(clippy::too_many_lines)]
fn get_standard_tools() -> Vec<McpToolDef> {
    vec![
        // bevy_destroy
        McpToolDef {
            name:            TOOL_BEVY_DESTROY,
            description:     DESC_BEVY_DESTROY,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_DESTROY,
            },
            params:          ParamDef::entity_with_port("The entity ID to destroy").to_vec(),
            param_extractor: BrpMethodParamCategory::Entity { required: true },
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::EntityOperation,
                template:        "Successfully destroyed entity {entity}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_DESTROYED_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                })],
            },
        },
        // bevy_get
        McpToolDef {
            name:            TOOL_BEVY_GET,
            description:     DESC_BEVY_GET,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_GET,
            },
            params:          vec![
                ParamDef::entity("The entity ID to get component data from", true),
                ParamDef::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::EntityOperation,
                template:        "Retrieved component data from entity {entity}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_ENTITY,
                        extractor: ExtractorType::EntityFromParams,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_COMPONENTS,
                        extractor: ExtractorType::PassThroughData,
                    }),
                ],
            },
        },
        // bevy_list
        McpToolDef {
            name:            TOOL_BEVY_LIST,
            description:     DESC_BEVY_LIST,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_LIST,
            },
            params:          vec![
                ParamDef::entity("Optional entity ID to list components for", false),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Entity { required: false },
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Listed {count} components",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_COMPONENTS,
                        extractor: ExtractorType::PassThroughData,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_COUNT,
                        extractor: ExtractorType::ComponentCountFromData,
                    }),
                ],
            },
        },
        // bevy_remove
        McpToolDef {
            name:            TOOL_BEVY_REMOVE,
            description:     DESC_BEVY_REMOVE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_REMOVE,
            },
            params:          vec![
                ParamDef::entity("The entity ID to remove components from", true),
                ParamDef::components("Array of component type names to remove", true),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::EntityOperation,
                template:        "Successfully removed components from entity {entity}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                })],
            },
        },
        // bevy_insert
        McpToolDef {
            name:            TOOL_BEVY_INSERT,
            description:     DESC_BEVY_INSERT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_INSERT,
            },
            params:          vec![
                ParamDef::entity("The entity ID to insert components into", true),
                ParamDef::components(
                    "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::EntityOperation,
                template:        "Successfully inserted components into entity {entity}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                })],
            },
        },
        // bevy_get_resource
        McpToolDef {
            name:            TOOL_BEVY_GET_RESOURCE,
            description:     DESC_BEVY_GET_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_GET_RESOURCE,
            },
            params:          ParamDef::resource_with_port(
                "The fully-qualified type name of the resource to get",
            )
            .to_vec(),
            param_extractor: BrpMethodParamCategory::Resource,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Retrieved resource: {resource}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_RESOURCE,
                        extractor: ExtractorType::ResourceFromParams,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_DATA,
                        extractor: ExtractorType::PassThroughData,
                    }),
                ],
            },
        },
        // bevy_insert_resource
        McpToolDef {
            name:            TOOL_BEVY_INSERT_RESOURCE,
            description:     DESC_BEVY_INSERT_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_INSERT_RESOURCE,
            },
            params:          vec![
                ParamDef::resource(
                    "The fully-qualified type name of the resource to insert or update",
                ),
                ParamDef::value(
                    "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Successfully inserted/updated resource: {resource}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_RESOURCE,
                    extractor: ExtractorType::ResourceFromParams,
                })],
            },
        },
        // bevy_remove_resource
        McpToolDef {
            name:            TOOL_BEVY_REMOVE_RESOURCE,
            description:     DESC_BEVY_REMOVE_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_REMOVE_RESOURCE,
            },
            params:          ParamDef::resource_with_port(
                "The fully-qualified type name of the resource to remove",
            )
            .to_vec(),
            param_extractor: BrpMethodParamCategory::Resource,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Successfully removed resource: {resource}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_RESOURCE,
                    extractor: ExtractorType::ResourceFromParams,
                })],
            },
        },
        // bevy_mutate_component
        McpToolDef {
            name:            TOOL_BEVY_MUTATE_COMPONENT,
            description:     DESC_BEVY_MUTATE_COMPONENT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_MUTATE_COMPONENT,
            },
            params:          ParamDef::component_mutation_params().to_vec(),
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::EntityOperation,
                template:        "Successfully mutated component on entity {entity}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                })],
            },
        },
        // bevy_mutate_resource
        McpToolDef {
            name:            TOOL_BEVY_MUTATE_RESOURCE,
            description:     DESC_BEVY_MUTATE_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_MUTATE_RESOURCE,
            },
            params:          ParamDef::resource_mutation_params().to_vec(),
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Successfully mutated resource: {resource}",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_RESOURCE,
                    extractor: ExtractorType::ResourceFromParams,
                })],
            },
        },
        // bevy_list_resources
        McpToolDef {
            name:            TOOL_BEVY_LIST_RESOURCES,
            description:     DESC_BEVY_LIST_RESOURCES,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_LIST_RESOURCES,
            },
            params:          vec![ParamDef::port()],
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Listed {count} resources",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_RESOURCES,
                        extractor: ExtractorType::PassThroughData,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_COUNT,
                        extractor: ExtractorType::ComponentCountFromData,
                    }),
                ],
            },
        },
        // bevy_rpc_discover
        McpToolDef {
            name:            TOOL_BEVY_RPC_DISCOVER,
            description:     DESC_BEVY_RPC_DISCOVER,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_RPC_DISCOVER,
            },
            params:          vec![ParamDef::port()],
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Retrieved BRP method discovery information",
                response_fields: vec![ResponseFieldCompat::V2(ResponseFieldV2::FromResponse {
                    name:      PARAM_RESULT,
                    extractor: ResponseExtractorType::PassThroughRaw,
                })],
            },
        },
        // bevy_brp_extras/discover_format
        McpToolDef {
            name:            TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
            description:     DESC_BRP_EXTRAS_DISCOVER_FORMAT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
            },
            params:          vec![
                ParamDef::string_array(
                    PARAM_TYPES,
                    "Array of fully-qualified component type names to discover formats for",
                    true,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Format discovery completed",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      PARAM_FORMATS,
                    extractor: ExtractorType::PassThroughResult,
                })],
            },
        },
        // bevy_screenshot
        McpToolDef {
            name:            TOOL_BRP_EXTRAS_SCREENSHOT,
            description:     DESC_BRP_EXTRAS_SCREENSHOT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SCREENSHOT,
            },
            params:          vec![
                ParamDef::path("File path where the screenshot should be saved"),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Successfully captured screenshot and saved to {path}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_PATH,
                        extractor: ExtractorType::ParamFromContext(JSON_FIELD_PATH),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_PORT,
                        extractor: ExtractorType::ParamFromContext(JSON_FIELD_PORT),
                    }),
                ],
            },
        },
        // brp_extras/send_keys
        McpToolDef {
            name:            TOOL_BRP_EXTRAS_SEND_KEYS,
            description:     DESC_BRP_EXTRAS_SEND_KEYS,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SEND_KEYS,
            },
            params:          vec![
                ParamDef::string_array("keys", "Array of key code names to send", true),
                ParamDef::number(
                    "duration_ms",
                    "Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1 minute)",
                    false,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Successfully sent keyboard input",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "keys_sent",
                        extractor: ExtractorType::PassThroughData,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "duration_ms",
                        extractor: ExtractorType::PassThroughData,
                    }),
                ],
            },
        },
        // brp_extras/set_debug_mode
        McpToolDef {
            name:            TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
            description:     DESC_BRP_EXTRAS_SET_DEBUG_MODE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
            },
            params:          vec![
                ParamDef::boolean(
                    "enabled",
                    "Enable or disable debug mode for bevy_brp_extras plugin",
                    true,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Debug mode updated successfully",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      "debug_enabled",
                    extractor: ExtractorType::PassThroughData,
                })],
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
            name:            TOOL_BEVY_QUERY,
            description:     DESC_BEVY_QUERY,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_QUERY,
            },
            params:          ParamDef::query_params().to_vec(),
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Query completed successfully",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_DATA,
                        extractor: ExtractorType::PassThroughData,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      PARAM_ENTITY_COUNT,
                        extractor: ExtractorType::EntityCountFromData,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      PARAM_COMPONENT_COUNT,
                        extractor: ExtractorType::QueryComponentCount,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      PARAM_QUERY_PARAMS,
                        extractor: ExtractorType::QueryParamsFromContext,
                    }),
                ],
            },
        },
        // bevy_spawn - has dynamic entity extraction from response
        McpToolDef {
            name:            TOOL_BEVY_SPAWN,
            description:     DESC_BEVY_SPAWN,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_SPAWN,
            },
            params:          vec![
                ParamDef::components(
                    "Object containing component data to spawn with. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    false,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::EntityOperation,
                template:        "Successfully spawned entity",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      PARAM_SPAWNED_ENTITY,
                        extractor: ExtractorType::EntityFromResponse,
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_COMPONENTS,
                        extractor: ExtractorType::ParamFromContext(JSON_FIELD_COMPONENTS),
                    }),
                ],
            },
        },
        // brp_execute - has dynamic method selection
        McpToolDef {
            name:            TOOL_BRP_EXECUTE,
            description:     DESC_BRP_EXECUTE,
            handler:         HandlerType::Brp { method: "" }, // Dynamic method
            params:          vec![
                ParamDef::method(),
                ParamDef::optional_params(),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::BrpExecute,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Method executed successfully",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      PARAM_RESULT,
                    extractor: ExtractorType::PassThroughResult,
                })],
            },
        },
        // bevy_registry_schema - has complex parameter transformation
        McpToolDef {
            name:            TOOL_BEVY_REGISTRY_SCHEMA,
            description:     DESC_BEVY_REGISTRY_SCHEMA,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_REGISTRY_SCHEMA,
            },
            params:          vec![
                ParamDef::string_array(
                    PARAM_WITH_CRATES,
                    "Include only types from these crates (e.g., [\"bevy_transform\", \"my_game\"])",
                    false,
                ),
                ParamDef::string_array(
                    PARAM_WITHOUT_CRATES,
                    "Exclude types from these crates (e.g., [\"bevy_render\", \"bevy_pbr\"])",
                    false,
                ),
                ParamDef::string_array(
                    PARAM_WITH_TYPES,
                    "Include only types with these reflect traits (e.g., [\"Component\", \"Resource\"])",
                    false,
                ),
                ParamDef::string_array(
                    PARAM_WITHOUT_TYPES,
                    "Exclude types with these reflect traits (e.g., [\"RenderResource\"])",
                    false,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::RegistrySchema,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Retrieved schema information",
                response_fields: vec![ResponseFieldCompat::V1(ResponseField {
                    name:      JSON_FIELD_DATA,
                    extractor: ExtractorType::PassThroughData,
                })],
            },
        },
        // bevy_reparent - has array parameter handling
        McpToolDef {
            name:            TOOL_BEVY_REPARENT,
            description:     DESC_BEVY_REPARENT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_REPARENT,
            },
            params:          vec![
                ParamDef::any(PARAM_ENTITIES, "Array of entity IDs to reparent", true),
                ParamDef::number(
                    PARAM_PARENT,
                    "The new parent entity ID (omit to remove parent)",
                    false,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Simple,
                template:        "Successfully reparented entities",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      PARAM_ENTITIES,
                        extractor: ExtractorType::ParamFromContext(PARAM_ENTITIES),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      PARAM_PARENT,
                        extractor: ExtractorType::ParamFromContext(PARAM_PARENT),
                    }),
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
            name:            TOOL_LIST_LOGS,
            description:     DESC_LIST_LOGS,
            handler:         HandlerType::Local {
                handler: Box::new(ListLogs),
            },
            params:          vec![
                ParamDef::string(
                    "app_name",
                    "Optional filter to list logs for a specific app only",
                    false,
                ),
                ParamDef::boolean(
                    "verbose",
                    "Include full details (path, timestamps, size in bytes). Default is false for minimal output",
                    false,
                ),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Found {count} log files",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "logs",
                        extractor: ExtractorType::DataField("logs"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "temp_directory",
                        extractor: ExtractorType::DataField("temp_directory"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "count",
                        extractor: ExtractorType::CountFromData,
                    }),
                ],
            },
        },
        // read_log
        McpToolDef {
            name:            TOOL_READ_LOG,
            description:     DESC_READ_LOG,
            handler:         HandlerType::Local {
                handler: Box::new(crate::log_tools::read_log::ReadLog),
            },
            params:          vec![
                ParamDef::string(
                    "filename",
                    "The log filename (e.g., bevy_brp_mcp_myapp_1234567890.log)",
                    true,
                ),
                ParamDef::string(
                    "keyword",
                    "Optional keyword to filter lines (case-insensitive)",
                    false,
                ),
                ParamDef::number(
                    "tail_lines",
                    "Optional number of lines to read from the end of file",
                    false,
                ),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Successfully read log file: {filename}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "filename",
                        extractor: ExtractorType::DataField("filename"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "file_path",
                        extractor: ExtractorType::DataField("file_path"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "size_bytes",
                        extractor: ExtractorType::DataField("size_bytes"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "size_human",
                        extractor: ExtractorType::DataField("size_human"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "lines_read",
                        extractor: ExtractorType::DataField("lines_read"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "content",
                        extractor: ExtractorType::DataField("content"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "filtered_by_keyword",
                        extractor: ExtractorType::DataField("filtered_by_keyword"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "tail_mode",
                        extractor: ExtractorType::DataField("tail_mode"),
                    }),
                ],
            },
        },
        // cleanup_logs
        McpToolDef {
            name:            TOOL_CLEANUP_LOGS,
            description:     DESC_CLEANUP_LOGS,
            handler:         HandlerType::Local {
                handler: Box::new(crate::log_tools::cleanup_logs::CleanupLogs),
            },
            params:          vec![
                ParamDef::string(
                    "app_name",
                    "Optional filter to delete logs for a specific app only",
                    false,
                ),
                ParamDef::number(
                    "older_than_seconds",
                    "Optional filter to delete logs older than N seconds",
                    false,
                ),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Deleted {deleted_count} log files",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "deleted_count",
                        extractor: ExtractorType::DataField("deleted_count"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "deleted_files",
                        extractor: ExtractorType::DataField("deleted_files"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "app_name_filter",
                        extractor: ExtractorType::DataField("app_name_filter"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "older_than_seconds",
                        extractor: ExtractorType::DataField("older_than_seconds"),
                    }),
                ],
            },
        },
        // brp_get_trace_log_path
        McpToolDef {
            name:            TOOL_GET_TRACE_LOG_PATH,
            description:     DESC_GET_TRACE_LOG_PATH,
            handler:         HandlerType::Local {
                handler: Box::new(GetTraceLogPath),
            },
            params:          vec![],
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Trace log file {exists:found at|not found (will be created when logging starts) at}: {log_path}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      JSON_FIELD_LOG_PATH,
                        extractor: ExtractorType::DataField(JSON_FIELD_LOG_PATH),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "exists",
                        extractor: ExtractorType::DataField("exists"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "file_size_bytes",
                        extractor: ExtractorType::DataField("file_size_bytes"),
                    }),
                ],
            },
        },
        // brp_set_tracing_level
        McpToolDef {
            name:            TOOL_SET_TRACING_LEVEL,
            description:     DESC_SET_TRACING_LEVEL,
            handler:         HandlerType::Local {
                handler: Box::new(crate::log_tools::set_tracing_level::SetTracingLevel),
            },
            params:          vec![ParamDef::string(
                "level",
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            )],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Tracing level set to '{level}' - diagnostic information will be logged to temp directory",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "tracing_level",
                        extractor: ExtractorType::DataField("level"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "log_file",
                        extractor: ExtractorType::DataField("log_file"),
                    }),
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
            name:            TOOL_LIST_BEVY_APPS,
            description:     DESC_LIST_BEVY_APPS,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_list_bevy_apps::ListBevyApps),
            },
            params:          vec![],
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Found {count} Bevy apps",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "apps",
                        extractor: ExtractorType::DataField("apps"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "count",
                        extractor: ExtractorType::CountFromData,
                    }),
                ],
            },
        },
        // list_brp_apps
        McpToolDef {
            name:            TOOL_LIST_BRP_APPS,
            description:     DESC_LIST_BRP_APPS,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_list_brp_apps::ListBrpApps),
            },
            params:          vec![],
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Found {count} BRP-enabled apps",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "apps",
                        extractor: ExtractorType::DataField("apps"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "count",
                        extractor: ExtractorType::CountFromData,
                    }),
                ],
            },
        },
        // list_bevy_examples
        McpToolDef {
            name:            TOOL_LIST_BEVY_EXAMPLES,
            description:     DESC_LIST_BEVY_EXAMPLES,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_list_bevy_examples::ListBevyExamples),
            },
            params:          vec![],
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Found {count} Bevy examples",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "examples",
                        extractor: ExtractorType::DataField("examples"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "count",
                        extractor: ExtractorType::CountFromData,
                    }),
                ],
            },
        },
        // launch_bevy_app
        McpToolDef {
            name:            TOOL_LAUNCH_BEVY_APP,
            description:     DESC_LAUNCH_BEVY_APP,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_launch_bevy_app::LaunchBevyApp),
            },
            params:          create_launch_params("app_name", "Name of the Bevy app to launch"),
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "{message}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "status",
                        extractor: ExtractorType::DataField("status"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "message",
                        extractor: ExtractorType::DataField("message"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "app_name",
                        extractor: ExtractorType::DataField("app_name"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "pid",
                        extractor: ExtractorType::DataField("pid"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "working_directory",
                        extractor: ExtractorType::DataField("working_directory"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "profile",
                        extractor: ExtractorType::DataField("profile"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "log_file",
                        extractor: ExtractorType::DataField("log_file"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "binary_path",
                        extractor: ExtractorType::DataField("binary_path"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "launch_duration_ms",
                        extractor: ExtractorType::DataField("launch_duration_ms"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "launch_timestamp",
                        extractor: ExtractorType::DataField("launch_timestamp"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "workspace",
                        extractor: ExtractorType::DataField("workspace"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "duplicate_paths",
                        extractor: ExtractorType::DataField("duplicate_paths"),
                    }),
                ],
            },
        },
        // launch_bevy_example
        McpToolDef {
            name:            TOOL_LAUNCH_BEVY_EXAMPLE,
            description:     DESC_LAUNCH_BEVY_EXAMPLE,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_launch_bevy_example::LaunchBevyExample),
            },
            params:          create_launch_params(
                "example_name",
                "Name of the Bevy example to launch",
            ),
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::Local,
                template:        "Launched Bevy example {example_name}",
                response_fields: vec![
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "status",
                        extractor: ExtractorType::DataField("status"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "message",
                        extractor: ExtractorType::DataField("message"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "example_name",
                        extractor: ExtractorType::DataField("example_name"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "pid",
                        extractor: ExtractorType::DataField("pid"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "working_directory",
                        extractor: ExtractorType::DataField("working_directory"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "profile",
                        extractor: ExtractorType::DataField("profile"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "log_file",
                        extractor: ExtractorType::DataField("log_file"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "launch_duration_ms",
                        extractor: ExtractorType::DataField("launch_duration_ms"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "launch_timestamp",
                        extractor: ExtractorType::DataField("launch_timestamp"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "workspace",
                        extractor: ExtractorType::DataField("workspace"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "package_name",
                        extractor: ExtractorType::DataField("package_name"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "duplicate_paths",
                        extractor: ExtractorType::DataField("duplicate_paths"),
                    }),
                    ResponseFieldCompat::V1(ResponseField {
                        name:      "note",
                        extractor: ExtractorType::DataField("note"),
                    }),
                ],
            },
        },
        // brp_extras_shutdown
        McpToolDef {
            name:            TOOL_SHUTDOWN,
            description:     DESC_SHUTDOWN,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_shutdown::Shutdown),
            },
            params:          vec![
                ParamDef::string("app_name", "Name of the Bevy app to shutdown", true),
                ParamDef::number(
                    JSON_FIELD_PORT,
                    "BRP port to connect to (default: 15702)",
                    false,
                ),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::LocalPassthrough,
                template:        "",
                response_fields: vec![],
            },
        },
        // brp_status
        McpToolDef {
            name:            TOOL_STATUS,
            description:     DESC_STATUS,
            handler:         HandlerType::Local {
                handler: Box::new(crate::app_tools::brp_status::Status),
            },
            params:          vec![
                ParamDef::string("app_name", "Name of the process to check for", true),
                ParamDef::number(
                    JSON_FIELD_PORT,
                    "Port to check for BRP (default: 15702)",
                    false,
                ),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::LocalPassthrough,
                template:        "",
                response_fields: vec![],
            },
        },
    ]
}

/// Get watch tool definitions
fn get_watch_tools() -> Vec<McpToolDef> {
    vec![
        // bevy_get_watch
        McpToolDef {
            name:            TOOL_BEVY_GET_WATCH,
            description:     DESC_BEVY_GET_WATCH,
            handler:         HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::bevy_get_watch::BevyGetWatch),
            },
            params:          vec![
                ParamDef::entity("The entity ID to watch for component changes", true),
                ParamDef::components(
                    "Required array of component types to watch. Must contain at least one component. Without this, the watch will not detect any changes.",
                    true,
                ),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::LocalPassthrough,
                template:        "",
                response_fields: vec![],
            },
        },
        // bevy_list_watch
        McpToolDef {
            name:            TOOL_BEVY_LIST_WATCH,
            description:     DESC_BEVY_LIST_WATCH,
            handler:         HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::bevy_list_watch::BevyListWatch),
            },
            params:          vec![
                ParamDef::entity("The entity ID to watch for component list changes", true),
                ParamDef::port(),
            ],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::LocalPassthrough,
                template:        "",
                response_fields: vec![],
            },
        },
    ]
}

/// Get BRP watch management tool definitions
fn get_brp_tools() -> Vec<McpToolDef> {
    vec![
        // brp_stop_watch
        McpToolDef {
            name:            TOOL_BRP_STOP_WATCH,
            description:     DESC_BRP_STOP_WATCH,
            handler:         HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::brp_stop_watch::BrpStopWatch),
            },
            params:          vec![ParamDef::number(
                "watch_id",
                "The watch ID returned from bevy_start_entity_watch or bevy_start_list_watch",
                true,
            )],
            param_extractor: BrpMethodParamCategory::Passthrough,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::LocalPassthrough,
                template:        "",
                response_fields: vec![],
            },
        },
        // brp_list_active_watches
        McpToolDef {
            name:            TOOL_BRP_LIST_ACTIVE_WATCHES,
            description:     DESC_BRP_LIST_ACTIVE_WATCHES,
            handler:         HandlerType::Local {
                handler: Box::new(crate::brp_tools::watch::brp_list_active::BrpListActiveWatches),
            },
            params:          vec![], // No parameters
            param_extractor: BrpMethodParamCategory::EmptyParams,
            formatter:       ResponseSpecification {
                formatter_type:  FormatterType::LocalPassthrough,
                template:        "",
                response_fields: vec![],
            },
        },
    ]
}

/// Create standard launch tool parameters (profile, path, port)
fn create_launch_params(name_param: &'static str, name_desc: &'static str) -> Vec<ParamDef> {
    vec![
        ParamDef::string(name_param, name_desc, true),
        ParamDef::string("profile", "Build profile to use (debug or release)", false),
        ParamDef::string(
            PARAM_BINARY_PATH,
            "Path to use when multiple apps/examples with the same name exist",
            false,
        ),
        ParamDef::number(JSON_FIELD_PORT, "BRP port to use (default: 15702)", false),
    ]
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
