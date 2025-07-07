//! Declarative tool definitions for BRP (Bevy Remote Protocol) tools.
//!
//! Provides a declarative approach to defining BRP tools that eliminates code duplication.
//! Tools are defined as data structures describing parameters, extractors, and response formatting.
//!
//! # Tool Categories
//!
//! - **Standard Tools**: CRUD operations following predictable patterns (destroy, get, insert,
//!   etc.)
//! - **Special Tools**: Tools requiring custom extractors or response handling (query, spawn,
//!   execute)
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
use crate::constants::PARAM_BINARY_PATH;
use crate::tools::{
    BRP_METHOD_DESTROY, BRP_METHOD_EXTRAS_DISCOVER_FORMAT, BRP_METHOD_EXTRAS_SCREENSHOT,
    BRP_METHOD_EXTRAS_SEND_KEYS, BRP_METHOD_EXTRAS_SET_DEBUG_MODE, BRP_METHOD_GET,
    BRP_METHOD_GET_RESOURCE, BRP_METHOD_INSERT, BRP_METHOD_INSERT_RESOURCE, BRP_METHOD_LIST,
    BRP_METHOD_LIST_RESOURCES, BRP_METHOD_MUTATE_COMPONENT, BRP_METHOD_MUTATE_RESOURCE,
    BRP_METHOD_QUERY, BRP_METHOD_REGISTRY_SCHEMA, BRP_METHOD_REMOVE, BRP_METHOD_REMOVE_RESOURCE,
    BRP_METHOD_REPARENT, BRP_METHOD_RPC_DISCOVER, BRP_METHOD_SPAWN, DESC_BEVY_DESTROY,
    DESC_BEVY_GET, DESC_BEVY_GET_RESOURCE, DESC_BEVY_INSERT, DESC_BEVY_INSERT_RESOURCE,
    DESC_BEVY_LIST, DESC_BEVY_LIST_RESOURCES, DESC_BEVY_MUTATE_COMPONENT,
    DESC_BEVY_MUTATE_RESOURCE, DESC_BEVY_QUERY, DESC_BEVY_REGISTRY_SCHEMA, DESC_BEVY_REMOVE,
    DESC_BEVY_REMOVE_RESOURCE, DESC_BEVY_REPARENT, DESC_BEVY_RPC_DISCOVER, DESC_BEVY_SPAWN,
    DESC_BRP_EXECUTE, DESC_BRP_EXTRAS_DISCOVER_FORMAT, DESC_BRP_EXTRAS_SCREENSHOT,
    DESC_BRP_EXTRAS_SEND_KEYS, DESC_BRP_EXTRAS_SET_DEBUG_MODE, DESC_CLEANUP_LOGS,
    DESC_GET_TRACE_LOG_PATH, DESC_LAUNCH_BEVY_APP, DESC_LAUNCH_BEVY_EXAMPLE, DESC_LIST_BEVY_APPS,
    DESC_LIST_BEVY_EXAMPLES, DESC_LIST_BRP_APPS, DESC_LIST_LOGS, DESC_READ_LOG,
    DESC_SET_TRACING_LEVEL, DESC_SHUTDOWN, DESC_STATUS, HANDLER_CLEANUP_LOGS,
    HANDLER_GET_TRACE_LOG_PATH, HANDLER_LAUNCH_BEVY_APP, HANDLER_LAUNCH_BEVY_EXAMPLE,
    HANDLER_LIST_BEVY_APPS, HANDLER_LIST_BEVY_EXAMPLES, HANDLER_LIST_BRP_APPS, HANDLER_LIST_LOGS,
    HANDLER_READ_LOG, HANDLER_SET_TRACING_LEVEL, HANDLER_SHUTDOWN, HANDLER_STATUS,
    TOOL_BEVY_DESTROY, TOOL_BEVY_GET, TOOL_BEVY_GET_RESOURCE, TOOL_BEVY_INSERT,
    TOOL_BEVY_INSERT_RESOURCE, TOOL_BEVY_LIST, TOOL_BEVY_LIST_RESOURCES,
    TOOL_BEVY_MUTATE_COMPONENT, TOOL_BEVY_MUTATE_RESOURCE, TOOL_BEVY_QUERY,
    TOOL_BEVY_REGISTRY_SCHEMA, TOOL_BEVY_REMOVE, TOOL_BEVY_REMOVE_RESOURCE, TOOL_BEVY_REPARENT,
    TOOL_BEVY_RPC_DISCOVER, TOOL_BEVY_SPAWN, TOOL_BRP_EXECUTE, TOOL_BRP_EXTRAS_DISCOVER_FORMAT,
    TOOL_BRP_EXTRAS_SCREENSHOT, TOOL_BRP_EXTRAS_SEND_KEYS, TOOL_BRP_EXTRAS_SET_DEBUG_MODE,
    TOOL_CLEANUP_LOGS, TOOL_GET_TRACE_LOG_PATH, TOOL_LAUNCH_BEVY_APP, TOOL_LAUNCH_BEVY_EXAMPLE,
    TOOL_LIST_BEVY_APPS, TOOL_LIST_BEVY_EXAMPLES, TOOL_LIST_BRP_APPS, TOOL_LIST_LOGS,
    TOOL_READ_LOG, TOOL_SET_TRACING_LEVEL, TOOL_SHUTDOWN, TOOL_STATUS,
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
fn get_standard_tools() -> Vec<BrpToolDef> {
    vec![
        // bevy_destroy
        BrpToolDef {
            name:            TOOL_BEVY_DESTROY,
            description:     DESC_BEVY_DESTROY,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_DESTROY,
            },
            params:          ParamDef::entity_with_port("The entity ID to destroy").to_vec(),
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
                ParamDef::entity("The entity ID to get component data from", true),
                ParamDef::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                ),
                ParamDef::port(),
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
                ParamDef::entity("Optional entity ID to list components for", false),
                ParamDef::port(),
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
                ParamDef::entity("The entity ID to remove components from", true),
                ParamDef::components("Array of component type names to remove", true),
                ParamDef::port(),
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
                ParamDef::entity("The entity ID to insert components into", true),
                ParamDef::components(
                    "Object containing component data to insert. Keys are component types, values are component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                ParamDef::port(),
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
            params:          ParamDef::resource_with_port(
                "The fully-qualified type name of the resource to get",
            )
            .to_vec(),
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
                ParamDef::resource(
                    "The fully-qualified type name of the resource to insert or update",
                ),
                ParamDef::value(
                    "The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.",
                    true,
                ),
                ParamDef::port(),
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
            params:          ParamDef::resource_with_port(
                "The fully-qualified type name of the resource to remove",
            )
            .to_vec(),
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
            params:          ParamDef::component_mutation_params().to_vec(),
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
            params:          ParamDef::resource_mutation_params().to_vec(),
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
            params:          vec![ParamDef::port()],
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
            params:          vec![ParamDef::port()],
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
                ParamDef::string_array(
                    PARAM_TYPES,
                    "Array of fully-qualified component type names to discover formats for",
                    true,
                ),
                ParamDef::port(),
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
                ParamDef::path("File path where the screenshot should be saved"),
                ParamDef::port(),
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
                ParamDef::string_array("keys", "Array of key code names to send", true),
                ParamDef::number(
                    "duration_ms",
                    "Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1 minute)",
                    false,
                ),
                ParamDef::port(),
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
        // brp_extras/set_debug_mode
        BrpToolDef {
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
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Debug mode updated successfully",
                response_fields: vec![ResponseField {
                    name:      "debug_enabled",
                    extractor: ExtractorType::PassThroughData,
                }],
            },
        },
    ]
}

/// Get tool definitions for tools with special variations
#[allow(clippy::too_many_lines)]
fn get_special_tools() -> Vec<BrpToolDef> {
    vec![
        // bevy_query - has custom extractors for component counts
        BrpToolDef {
            name:            TOOL_BEVY_QUERY,
            description:     DESC_BEVY_QUERY,
            handler:         HandlerType::Brp {
                method: BRP_METHOD_QUERY,
            },
            params:          ParamDef::query_params().to_vec(),
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
            name:            TOOL_BRP_EXECUTE,
            description:     DESC_BRP_EXECUTE,
            handler:         HandlerType::Brp { method: "" }, // Dynamic method
            params:          vec![
                ParamDef::method(),
                ParamDef::optional_params(),
                ParamDef::port(),
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
fn get_log_tools() -> Vec<BrpToolDef> {
    vec![
        // list_logs
        BrpToolDef {
            name:            TOOL_LIST_LOGS,
            description:     DESC_LIST_LOGS,
            handler:         HandlerType::Local {
                handler: HANDLER_LIST_LOGS,
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
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // read_log
        BrpToolDef {
            name:            TOOL_READ_LOG,
            description:     DESC_READ_LOG,
            handler:         HandlerType::Local {
                handler: HANDLER_READ_LOG,
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
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // cleanup_logs
        BrpToolDef {
            name:            TOOL_CLEANUP_LOGS,
            description:     DESC_CLEANUP_LOGS,
            handler:         HandlerType::Local {
                handler: HANDLER_CLEANUP_LOGS,
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
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // brp_get_trace_log_path
        BrpToolDef {
            name:            TOOL_GET_TRACE_LOG_PATH,
            description:     DESC_GET_TRACE_LOG_PATH,
            handler:         HandlerType::Local {
                handler: HANDLER_GET_TRACE_LOG_PATH,
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // brp_set_tracing_level
        BrpToolDef {
            name:            TOOL_SET_TRACING_LEVEL,
            description:     DESC_SET_TRACING_LEVEL,
            handler:         HandlerType::Local {
                handler: HANDLER_SET_TRACING_LEVEL,
            },
            params:          vec![ParamDef::string(
                "level",
                "Tracing level to set (error, warn, info, debug, trace)",
                true,
            )],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
    ]
}

/// Get app tool definitions
fn get_app_tools() -> Vec<BrpToolDef> {
    vec![
        // list_bevy_apps
        BrpToolDef {
            name:            TOOL_LIST_BEVY_APPS,
            description:     DESC_LIST_BEVY_APPS,
            handler:         HandlerType::Local {
                handler: HANDLER_LIST_BEVY_APPS,
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // list_brp_apps
        BrpToolDef {
            name:            TOOL_LIST_BRP_APPS,
            description:     DESC_LIST_BRP_APPS,
            handler:         HandlerType::Local {
                handler: HANDLER_LIST_BRP_APPS,
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // list_bevy_examples
        BrpToolDef {
            name:            TOOL_LIST_BEVY_EXAMPLES,
            description:     DESC_LIST_BEVY_EXAMPLES,
            handler:         HandlerType::Local {
                handler: HANDLER_LIST_BEVY_EXAMPLES,
            },
            params:          vec![],
            param_extractor: ParamExtractorType::EmptyParams,
            formatter:       FormatterDef::default(),
        },
        // launch_bevy_app
        BrpToolDef {
            name:            TOOL_LAUNCH_BEVY_APP,
            description:     DESC_LAUNCH_BEVY_APP,
            handler:         HandlerType::Local {
                handler: HANDLER_LAUNCH_BEVY_APP,
            },
            params:          create_launch_params("app_name", "Name of the Bevy app to launch"),
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // launch_bevy_example
        BrpToolDef {
            name:            TOOL_LAUNCH_BEVY_EXAMPLE,
            description:     DESC_LAUNCH_BEVY_EXAMPLE,
            handler:         HandlerType::Local {
                handler: HANDLER_LAUNCH_BEVY_EXAMPLE,
            },
            params:          create_launch_params(
                "example_name",
                "Name of the Bevy example to launch",
            ),
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // brp_extras_shutdown
        BrpToolDef {
            name:            TOOL_SHUTDOWN,
            description:     DESC_SHUTDOWN,
            handler:         HandlerType::Local {
                handler: HANDLER_SHUTDOWN,
            },
            params:          vec![
                ParamDef::string("app_name", "Name of the Bevy app to shutdown", true),
                ParamDef::number(
                    JSON_FIELD_PORT,
                    "BRP port to connect to (default: 15702)",
                    false,
                ),
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
        },
        // brp_status
        BrpToolDef {
            name:            TOOL_STATUS,
            description:     DESC_STATUS,
            handler:         HandlerType::Local {
                handler: HANDLER_STATUS,
            },
            params:          vec![
                ParamDef::string("app_name", "Name of the process to check for", true),
                ParamDef::number(
                    JSON_FIELD_PORT,
                    "Port to check for BRP (default: 15702)",
                    false,
                ),
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef::default(),
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
