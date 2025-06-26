//! Declarative tool definitions for BRP (Bevy Remote Protocol) tools.
//!
//! Provides a declarative approach to defining BRP tools that eliminates code duplication.
//! Tools are defined as data structures describing parameters, extractors, and response formatting.
//!
//! # Tool Categories
//!
//! - **Standard Tools**: CRUD operations following predictable patterns (destroy, get, insert, etc.)
//! - **Special Tools**: Tools requiring custom extractors or response handling (query, spawn, execute)
//! - **Local Tools**: Execute within MCP server (log management, app lifecycle)
//!
//! # Handler Types
//!
//! - `HandlerType::Brp`: Execute remote BRP method calls over network
//! - `HandlerType::Local`: Execute local functions within MCP server
//!
//! # Adding New Tools
//!
//! For standard BRP tools, add to `get_standard_tools()` with `HandlerType::Brp`.
//! For local tools, add to `get_log_tools()` or `get_app_tools()` with `HandlerType::Local`.
//! For complex tools needing custom behavior, add to `get_special_tools()`.
//!
//! Use `FormatterDef::default()` for simple responses, custom formatters for structured output.

use crate::brp_tools::constants::{
    DESC_PORT, JSON_FIELD_COMPONENT, JSON_FIELD_COMPONENTS, JSON_FIELD_COUNT, JSON_FIELD_DATA,
    JSON_FIELD_DESTROYED_ENTITY, JSON_FIELD_ENTITY, JSON_FIELD_METADATA, JSON_FIELD_PATH,
    JSON_FIELD_PORT, JSON_FIELD_RESOURCE, JSON_FIELD_RESOURCES, JSON_FIELD_VALUE,
    PARAM_COMPONENT_COUNT, PARAM_DATA, PARAM_ENTITIES, PARAM_ENTITY_COUNT, PARAM_FILTER,
    PARAM_FORMATS, PARAM_METHOD, PARAM_PARAMS, PARAM_PARENT, PARAM_QUERY_PARAMS, PARAM_RESULT,
    PARAM_SPAWNED_ENTITY, PARAM_STRICT, PARAM_TYPES, PARAM_WITH_CRATES, PARAM_WITH_TYPES,
    PARAM_WITHOUT_CRATES, PARAM_WITHOUT_TYPES,
};
use crate::constants::PARAM_WORKSPACE;
use crate::tools::{
    BRP_METHOD_DESTROY, BRP_METHOD_EXTRAS_DISCOVER_FORMAT, BRP_METHOD_EXTRAS_SCREENSHOT,
    BRP_METHOD_EXTRAS_SEND_KEYS, BRP_METHOD_EXTRAS_SET_DEBUG_MODE, BRP_METHOD_GET,
    BRP_METHOD_GET_RESOURCE, BRP_METHOD_INSERT, BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST,
    BRP_METHOD_LIST_RESOURCES, BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE,
    BRP_METHOD_REMOVE, BRP_METHOD_REMOVE_RESOURCE, BRP_METHOD_RPC_DISCOVER, DESC_BEVY_DESTROY,
    DESC_BEVY_GET, DESC_BEVY_GET_RESOURCE, DESC_BEVY_INSERT, DESC_BEVY_INSERT_RESOURCE,
    DESC_BEVY_LIST, DESC_BEVY_LIST_RESOURCES, DESC_BEVY_MUTATE_COMPONENT,
    DESC_BEVY_MUTATE_RESOURCE, DESC_BEVY_REMOVE, DESC_BEVY_REMOVE_RESOURCE, DESC_BEVY_RPC_DISCOVER,
    DESC_BRP_EXTRAS_DISCOVER_FORMAT, DESC_BRP_EXTRAS_SCREENSHOT, DESC_BRP_EXTRAS_SEND_KEYS,
    DESC_BRP_EXTRAS_SET_DEBUG_MODE, TOOL_BEVY_DESTROY, TOOL_BEVY_GET, TOOL_BEVY_GET_RESOURCE,
    TOOL_BEVY_INSERT, TOOL_BEVY_INSERT_RESOURCE, TOOL_BEVY_LIST, TOOL_BEVY_LIST_RESOURCES,
    TOOL_BEVY_MUTATE_COMPONENT, TOOL_BEVY_MUTATE_RESOURCE, TOOL_BEVY_REMOVE,
    TOOL_BEVY_REMOVE_RESOURCE, TOOL_BEVY_RPC_DISCOVER, TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
    TOOL_BRP_EXTRAS_SCREENSHOT, TOOL_BRP_EXTRAS_SEND_KEYS, TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
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

/// Defines how to format the response for a tool
#[derive(Clone)]
pub struct FormatterDef {
    /// Type of formatter to use
    pub formatter_type:  FormatterType,
    /// Template for success messages
    pub template:        &'static str,
    /// Fields to include in the response
    pub response_fields: Vec<ResponseField>,
}

impl FormatterDef {
    /// Creates a default formatter for local tools that don't need special formatting
    pub const fn default() -> Self {
        Self {
            formatter_type:  FormatterType::Simple,
            template:        "",
            response_fields: vec![],
        }
    }
}

/// Types of formatters available
#[derive(Clone)]
pub enum FormatterType {
    /// Entity operation formatter
    EntityOperation(&'static str),
    /// Resource operation formatter
    ResourceOperation,
    /// Simple formatter (no special formatting)
    Simple,
}

/// Defines a field to include in the response
#[derive(Clone)]
pub struct ResponseField {
    /// Name of the field in the response
    pub name:      &'static str,
    /// Type of extractor to use
    pub extractor: ExtractorType,
}

/// Types of extractors for response fields
#[derive(Clone)]
pub enum ExtractorType {
    /// Extract entity from params
    EntityFromParams,
    /// Extract resource from params
    ResourceFromParams,
    /// Pass through data from BRP response
    PassThroughData,
    /// Pass through entire result
    PassThroughResult,
    /// Extract entity count from data
    EntityCountFromData,
    /// Extract component count from data
    ComponentCountFromData,
    /// Extract entity from response data (for spawn operation)
    EntityFromResponse,
    /// Extract total component count from nested query results
    QueryComponentCount,
    /// Extract query parameters from request context
    QueryParamsFromContext,
    /// Extract specific parameter from request context
    ParamFromContext(&'static str),
}

/// Type of parameter extractor to use
#[derive(Clone)]
pub enum ParamExtractorType {
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

/// Type of handler for the tool
#[derive(Clone)]
pub enum HandlerType {
    /// BRP handler - calls a BRP method
    Brp {
        /// BRP method to call (e.g., "bevy/destroy")
        method: &'static str,
    },
    /// Local handler - executes local logic
    Local {
        /// Handler function name (e.g., "`list_logs`", "`read_log`")
        handler: &'static str,
    },
}

/// Complete definition of a BRP tool
#[derive(Clone)]
pub struct BrpToolDef {
    /// Tool name (e.g., "`bevy_destroy`")
    pub name:            &'static str,
    /// Tool description
    pub description:     &'static str,
    /// Handler type (BRP or Local)
    pub handler:         HandlerType,
    /// Parameters for the tool
    pub params:          Vec<ParamDef>,
    /// Parameter extractor type
    pub param_extractor: ParamExtractorType,
    /// Response formatter definition
    pub formatter:       FormatterDef,
}

/// Get all standard tool definitions
#[allow(clippy::too_many_lines)]
pub fn get_standard_tools() -> Vec<BrpToolDef> {
    vec![
        // bevy_destroy
        BrpToolDef {
            name:            TOOL_BEVY_DESTROY,
            description:     DESC_BEVY_DESTROY,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_DESTROY,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_ENTITY,
                    description: "The entity ID to destroy",
                    required:    true,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Entity { required: true },
            formatter:       FormatterDef {
                formatter_type:  FormatterType::EntityOperation(JSON_FIELD_DESTROYED_ENTITY),
                template:        "Successfully destroyed entity {entity}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_DESTROYED_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                }],
            },
        },
        // bevy_get
        BrpToolDef {
            name:            TOOL_BEVY_GET,
            description:     DESC_BEVY_GET,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_GET,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_ENTITY,
                    description: "The entity ID to get component data from",
                    required:    true,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_COMPONENTS,
                    description: "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::EntityOperation(JSON_FIELD_ENTITY),
                template:        "Retrieved component data from entity {entity}",
                response_fields: vec![
                    ResponseField {
                        name:      JSON_FIELD_ENTITY,
                        extractor: ExtractorType::EntityFromParams,
                    },
                    ResponseField {
                        name:      JSON_FIELD_COMPONENTS,
                        extractor: ExtractorType::PassThroughData,
                    },
                ],
            },
        },
        // bevy_list
        BrpToolDef {
            name:            TOOL_BEVY_LIST,
            description:     DESC_BEVY_LIST,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_LIST,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_ENTITY,
                    description: "Optional entity ID to list components for",
                    required:    false,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Entity { required: false },
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Listed {count} components",
                response_fields: vec![
                    ResponseField {
                        name:      JSON_FIELD_COMPONENTS,
                        extractor: ExtractorType::PassThroughData,
                    },
                    ResponseField {
                        name:      JSON_FIELD_COUNT,
                        extractor: ExtractorType::ComponentCountFromData,
                    },
                ],
            },
        },
        // bevy_remove
        BrpToolDef {
            name:            TOOL_BEVY_REMOVE,
            description:     DESC_BEVY_REMOVE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_REMOVE,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_ENTITY,
                    description: "The entity ID to remove components from",
                    required:    true,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_COMPONENTS,
                    description: "Array of component type names to remove",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::EntityOperation(JSON_FIELD_ENTITY),
                template:        "Successfully removed components from entity {entity}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                }],
            },
        },
        // bevy_insert
        BrpToolDef {
            name:            TOOL_BEVY_INSERT,
            description:     DESC_BEVY_INSERT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_INSERT,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_ENTITY,
                    description: "The entity ID to insert components into",
                    required:    true,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_COMPONENTS,
                    description: "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::EntityOperation(JSON_FIELD_ENTITY),
                template:        "Successfully inserted components into entity {entity}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                }],
            },
        },
        // bevy_get_resource
        BrpToolDef {
            name:            TOOL_BEVY_GET_RESOURCE,
            description:     DESC_BEVY_GET_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_GET_RESOURCE,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_RESOURCE,
                    description: "The fully-qualified type name of the resource to get",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Resource,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Retrieved resource: {resource}",
                response_fields: vec![
                    ResponseField {
                        name:      JSON_FIELD_RESOURCE,
                        extractor: ExtractorType::ResourceFromParams,
                    },
                    ResponseField {
                        name:      JSON_FIELD_DATA,
                        extractor: ExtractorType::PassThroughData,
                    },
                ],
            },
        },
        // bevy_insert_resource
        BrpToolDef {
            name:            TOOL_BEVY_INSERT_RESOURCE,
            description:     DESC_BEVY_INSERT_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_INSERT_RESOURCE,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_RESOURCE,
                    description: "The fully-qualified type name of the resource to insert or update",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_VALUE,
                    description: "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Successfully inserted/updated resource: {resource}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_RESOURCE,
                    extractor: ExtractorType::ResourceFromParams,
                }],
            },
        },
        // bevy_remove_resource
        BrpToolDef {
            name:            TOOL_BEVY_REMOVE_RESOURCE,
            description:     DESC_BEVY_REMOVE_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_REMOVE_RESOURCE,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_RESOURCE,
                    description: "The fully-qualified type name of the resource to remove",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Resource,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Successfully removed resource: {resource}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_RESOURCE,
                    extractor: ExtractorType::ResourceFromParams,
                }],
            },
        },
        // bevy_mutate_component
        BrpToolDef {
            name:            TOOL_BEVY_MUTATE_COMPONENT,
            description:     DESC_BEVY_MUTATE_COMPONENT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_MUTATE_COMPONENT,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_ENTITY,
                    description: "The entity ID containing the component to mutate",
                    required:    true,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_COMPONENT,
                    description: "The fully-qualified type name of the component to mutate",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_PATH,
                    description: "The path to the field within the component (e.g., 'translation.x')",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_VALUE,
                    description: "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::EntityOperation(JSON_FIELD_ENTITY),
                template:        "Successfully mutated component on entity {entity}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_ENTITY,
                    extractor: ExtractorType::EntityFromParams,
                }],
            },
        },
        // bevy_mutate_resource
        BrpToolDef {
            name:            TOOL_BEVY_MUTATE_RESOURCE,
            description:     DESC_BEVY_MUTATE_RESOURCE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_MUTATE_RESOURCE,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_RESOURCE,
                    description: "The fully-qualified type name of the resource to mutate",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_PATH,
                    description: "The path to the field within the resource (e.g., 'settings.volume')",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_VALUE,
                    description: "The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::ResourceOperation,
                template:        "Successfully mutated resource: {resource}",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_RESOURCE,
                    extractor: ExtractorType::ResourceFromParams,
                }],
            },
        },
        // bevy_list_resources
        BrpToolDef {
            name:            TOOL_BEVY_LIST_RESOURCES,
            description:     DESC_BEVY_LIST_RESOURCES,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_LIST_RESOURCES,
            },
            params:          vec![ParamDef {
                name:        JSON_FIELD_PORT,
                description: DESC_PORT,
                required:    false,
                param_type:  ParamType::Number,
            }],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Listed {count} resources",
                response_fields: vec![
                    ResponseField {
                        name:      JSON_FIELD_RESOURCES,
                        extractor: ExtractorType::PassThroughData,
                    },
                    ResponseField {
                        name:      JSON_FIELD_COUNT,
                        extractor: ExtractorType::ComponentCountFromData,
                    },
                ],
            },
        },
        // bevy_rpc_discover
        BrpToolDef {
            name:            TOOL_BEVY_RPC_DISCOVER,
            description:     DESC_BEVY_RPC_DISCOVER,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_RPC_DISCOVER,
            },
            params:          vec![ParamDef {
                name:        JSON_FIELD_PORT,
                description: DESC_PORT,
                required:    false,
                param_type:  ParamType::Number,
            }],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Retrieved BRP method discovery information",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_METADATA,
                    extractor: ExtractorType::PassThroughResult,
                }],
            },
        },
        // bevy_brp_extras/discover_format
        BrpToolDef {
            name:            TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
            description:     DESC_BRP_EXTRAS_DISCOVER_FORMAT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_DISCOVER_FORMAT,
            },
            params:          vec![
                ParamDef {
                    name:        PARAM_TYPES,
                    description: "Array of fully-qualified component type names to discover formats for",
                    required:    true,
                    param_type:  ParamType::StringArray,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Format discovery completed",
                response_fields: vec![ResponseField {
                    name:      PARAM_FORMATS,
                    extractor: ExtractorType::PassThroughResult,
                }],
            },
        },
        // bevy_screenshot
        BrpToolDef {
            name:            TOOL_BRP_EXTRAS_SCREENSHOT,
            description:     DESC_BRP_EXTRAS_SCREENSHOT,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SCREENSHOT,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_PATH,
                    description: "File path where the screenshot should be saved",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Successfully captured screenshot and saved to {path}",
                response_fields: vec![
                    ResponseField {
                        name:      JSON_FIELD_PATH,
                        extractor: ExtractorType::ParamFromContext(JSON_FIELD_PATH),
                    },
                    ResponseField {
                        name:      JSON_FIELD_PORT,
                        extractor: ExtractorType::ParamFromContext(JSON_FIELD_PORT),
                    },
                ],
            },
        },
        // brp_extras/send_keys
        BrpToolDef {
            name:            TOOL_BRP_EXTRAS_SEND_KEYS,
            description:     DESC_BRP_EXTRAS_SEND_KEYS,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SEND_KEYS,
            },
            params:          vec![
                ParamDef {
                    name:        "keys",
                    description: "Array of key code names to send",
                    required:    true,
                    param_type:  ParamType::StringArray,
                },
                ParamDef {
                    name:        "duration_ms",
                    description: "Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1 minute)",
                    required:    false,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Successfully sent keyboard input",
                response_fields: vec![
                    ResponseField {
                        name:      "keys_sent",
                        extractor: ExtractorType::PassThroughData,
                    },
                    ResponseField {
                        name:      "duration_ms",
                        extractor: ExtractorType::PassThroughData,
                    },
                ],
            },
        },
        BrpToolDef {
            name:            TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
            description:     DESC_BRP_EXTRAS_SET_DEBUG_MODE,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_EXTRAS_SET_DEBUG_MODE,
            },
            params:          vec![
                ParamDef {
                    name:        "enabled",
                    description: "Set to true to enable debug output, false to disable",
                    required:    true,
                    param_type:  ParamType::Boolean,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "{message}",
                response_fields: vec![
                    ResponseField {
                        name:      "message",
                        extractor: ExtractorType::PassThroughData,
                    },
                    ResponseField {
                        name:      "debug_enabled",
                        extractor: ExtractorType::PassThroughData,
                    },
                ],
            },
        },
    ]
}

/// Get tool definitions for tools with special variations
#[allow(clippy::too_many_lines)]
pub fn get_special_tools() -> Vec<BrpToolDef> {
    vec![
        // bevy_query - has custom extractors for component counts
        BrpToolDef {
            name:            crate::tools::TOOL_BEVY_QUERY,
            description:     crate::tools::DESC_BEVY_QUERY,
            handler:         HandlerType::Brp {
                method: crate::tools::BRP_METHOD_QUERY,
            },
            params:          vec![
                ParamDef {
                    name:        PARAM_DATA,
                    description: "Object specifying what component data to retrieve. Properties: components (array), option (array), has (array)",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        PARAM_FILTER,
                    description: "Object specifying which entities to query. Properties: with (array), without (array)",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        PARAM_STRICT,
                    description: "If true, returns error on unknown component types (default: false)",
                    required:    false,
                    param_type:  ParamType::Boolean,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Query completed successfully",
                response_fields: vec![
                    ResponseField {
                        name:      JSON_FIELD_DATA,
                        extractor: ExtractorType::PassThroughData,
                    },
                    ResponseField {
                        name:      PARAM_ENTITY_COUNT,
                        extractor: ExtractorType::EntityCountFromData,
                    },
                    ResponseField {
                        name:      PARAM_COMPONENT_COUNT,
                        extractor: ExtractorType::QueryComponentCount,
                    },
                    ResponseField {
                        name:      PARAM_QUERY_PARAMS,
                        extractor: ExtractorType::QueryParamsFromContext,
                    },
                ],
            },
        },
        // bevy_spawn - has dynamic entity extraction from response
        BrpToolDef {
            name:            crate::tools::TOOL_BEVY_SPAWN,
            description:     crate::tools::DESC_BEVY_SPAWN,
            handler:         HandlerType::Brp {
                method: crate::tools::BRP_METHOD_SPAWN,
            },
            params:          vec![
                ParamDef {
                    name:        JSON_FIELD_COMPONENTS,
                    description: "Object containing component data to spawn with. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    required:    false,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::EntityOperation(PARAM_SPAWNED_ENTITY),
                template:        "Successfully spawned entity",
                response_fields: vec![
                    ResponseField {
                        name:      PARAM_SPAWNED_ENTITY,
                        extractor: ExtractorType::EntityFromResponse,
                    },
                    ResponseField {
                        name:      JSON_FIELD_COMPONENTS,
                        extractor: ExtractorType::ParamFromContext(JSON_FIELD_COMPONENTS),
                    },
                ],
            },
        },
        // brp_execute - has dynamic method selection
        BrpToolDef {
            name:            crate::tools::TOOL_BRP_EXECUTE,
            description:     crate::tools::DESC_BRP_EXECUTE,
            handler:         HandlerType::Brp { method: "" }, // Dynamic method
            params:          vec![
                ParamDef {
                    name:        PARAM_METHOD,
                    description: "The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        PARAM_PARAMS,
                    description: "Optional parameters for the method, as a JSON object or array",
                    required:    false,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::BrpExecute,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Method executed successfully",
                response_fields: vec![ResponseField {
                    name:      PARAM_RESULT,
                    extractor: ExtractorType::PassThroughResult,
                }],
            },
        },
        // bevy_registry_schema - has complex parameter transformation
        BrpToolDef {
            name:            crate::tools::TOOL_BEVY_REGISTRY_SCHEMA,
            description:     crate::tools::DESC_BEVY_REGISTRY_SCHEMA,
            handler:         HandlerType::Brp {
                method: crate::tools::BRP_METHOD_REGISTRY_SCHEMA,
            },
            params:          vec![
                ParamDef {
                    name:        PARAM_WITH_CRATES,
                    description: "Include only types from these crates (e.g., [\"bevy_transform\", \"my_game\"])",
                    required:    false,
                    param_type:  ParamType::StringArray,
                },
                ParamDef {
                    name:        PARAM_WITHOUT_CRATES,
                    description: "Exclude types from these crates (e.g., [\"bevy_render\", \"bevy_pbr\"])",
                    required:    false,
                    param_type:  ParamType::StringArray,
                },
                ParamDef {
                    name:        PARAM_WITH_TYPES,
                    description: "Include only types with these reflect traits (e.g., [\"Component\", \"Resource\"])",
                    required:    false,
                    param_type:  ParamType::StringArray,
                },
                ParamDef {
                    name:        PARAM_WITHOUT_TYPES,
                    description: "Exclude types with these reflect traits (e.g., [\"RenderResource\"])",
                    required:    false,
                    param_type:  ParamType::StringArray,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::RegistrySchema,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Retrieved schema information",
                response_fields: vec![ResponseField {
                    name:      JSON_FIELD_DATA,
                    extractor: ExtractorType::PassThroughData,
                }],
            },
        },
        // bevy_reparent - has array parameter handling
        BrpToolDef {
            name:            crate::tools::TOOL_BEVY_REPARENT,
            description:     crate::tools::DESC_BEVY_REPARENT,
            handler:         HandlerType::Brp {
                method: crate::tools::BRP_METHOD_REPARENT,
            },
            params:          vec![
                ParamDef {
                    name:        PARAM_ENTITIES,
                    description: "Array of entity IDs to reparent",
                    required:    true,
                    param_type:  ParamType::Any,
                },
                ParamDef {
                    name:        PARAM_PARENT,
                    description: "The new parent entity ID (omit to remove parent)",
                    required:    false,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        JSON_FIELD_PORT,
                    description: DESC_PORT,
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Successfully reparented entities",
                response_fields: vec![
                    ResponseField {
                        name:      PARAM_ENTITIES,
                        extractor: ExtractorType::ParamFromContext(PARAM_ENTITIES),
                    },
                    ResponseField {
                        name:      PARAM_PARENT,
                        extractor: ExtractorType::ParamFromContext(PARAM_PARENT),
                    },
                ],
            },
        },
    ]
}

/// Get log tool definitions
pub fn get_log_tools() -> Vec<BrpToolDef> {
    vec![
        // list_logs
        BrpToolDef {
            name:            crate::tools::TOOL_LIST_LOGS,
            description:     crate::tools::DESC_LIST_LOGS,
            handler:         HandlerType::Local {
                handler: "list_logs",
            },
            params:          vec![ParamDef {
                name:        "app_name",
                description: "Optional filter to list logs for a specific app only",
                required:    false,
                param_type:  ParamType::String,
            }],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // read_log
        BrpToolDef {
            name:            crate::tools::TOOL_READ_LOG,
            description:     crate::tools::DESC_READ_LOG,
            handler:         HandlerType::Local {
                handler: "read_log",
            },
            params:          vec![
                ParamDef {
                    name:        "filename",
                    description: "The log filename (e.g., bevy_brp_mcp_myapp_1234567890.log)",
                    required:    true,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        "keyword",
                    description: "Optional keyword to filter lines (case-insensitive)",
                    required:    false,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        "tail_lines",
                    description: "Optional number of lines to read from the end of file",
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // cleanup_logs
        BrpToolDef {
            name:            crate::tools::TOOL_CLEANUP_LOGS,
            description:     crate::tools::DESC_CLEANUP_LOGS,
            handler:         HandlerType::Local {
                handler: "cleanup_logs",
            },
            params:          vec![
                ParamDef {
                    name:        "app_name",
                    description: "Optional filter to delete logs for a specific app only",
                    required:    false,
                    param_type:  ParamType::String,
                },
                ParamDef {
                    name:        "older_than_seconds",
                    description: "Optional filter to delete logs older than N seconds",
                    required:    false,
                    param_type:  ParamType::Number,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
    ]
}

/// Get app tool definitions
pub fn get_app_tools() -> Vec<BrpToolDef> {
    vec![
        // list_bevy_apps
        BrpToolDef {
            name:            crate::tools::TOOL_LIST_BEVY_APPS,
            description:     crate::tools::DESC_LIST_BEVY_APPS,
            handler:         HandlerType::Local {
                handler: "list_bevy_apps",
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // list_brp_apps
        BrpToolDef {
            name:            crate::tools::TOOL_LIST_BRP_APPS,
            description:     crate::tools::DESC_LIST_BRP_APPS,
            handler:         HandlerType::Local {
                handler: "list_brp_apps",
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // list_bevy_examples
        BrpToolDef {
            name:            crate::tools::TOOL_LIST_BEVY_EXAMPLES,
            description:     crate::tools::DESC_LIST_BEVY_EXAMPLES,
            handler:         HandlerType::Local {
                handler: "list_bevy_examples",
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // launch_bevy_app
        BrpToolDef {
            name:            crate::tools::TOOL_LAUNCH_BEVY_APP,
            description:     crate::tools::DESC_LAUNCH_BEVY_APP,
            handler:         HandlerType::Local {
                handler: "launch_bevy_app",
            },
            params:          create_launch_params("app_name", "Name of the Bevy app to launch"),
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // launch_bevy_example
        BrpToolDef {
            name:            crate::tools::TOOL_LAUNCH_BEVY_EXAMPLE,
            description:     crate::tools::DESC_LAUNCH_BEVY_EXAMPLE,
            handler:         HandlerType::Local {
                handler: "launch_bevy_example",
            },
            params:          create_launch_params("example_name", "Name of the Bevy example to launch"),
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
    ]
}

/// Create standard launch tool parameters (profile, workspace, port)
fn create_launch_params(name_param: &'static str, name_desc: &'static str) -> Vec<ParamDef> {
    vec![
        ParamDef {
            name: name_param,
            description: name_desc,
            required: true,
            param_type: ParamType::String,
        },
        ParamDef {
            name: "profile",
            description: "Build profile to use (debug or release)",
            required: false,
            param_type: ParamType::String,
        },
        ParamDef {
            name: PARAM_WORKSPACE,
            description: "Workspace name to use when multiple apps/examples with the same name exist",
            required: false,
            param_type: ParamType::String,
        },
        ParamDef {
            name: JSON_FIELD_PORT,
            description: "BRP port to use (default: 15702)",
            required: false,
            param_type: ParamType::Number,
        },
    ]
}

/// Get all tool definitions - combines standard, special, log, and app tools
pub fn get_all_tools() -> Vec<BrpToolDef> {
    let mut tools = Vec::new();

    // Add standard tools
    tools.extend(get_standard_tools());

    // Add special tools
    tools.extend(get_special_tools());

    // Add log tools
    tools.extend(get_log_tools());

    // Add app tools
    tools.extend(get_app_tools());

    tools
}
