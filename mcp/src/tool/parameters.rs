//! Parameter definitions for our MCP tools

use crate::constants::{
    DESC_PORT, PARAM_COMPONENTS, PARAM_ENTITY, PARAM_METHOD, PARAM_PATH, PARAM_PORT,
    PARAM_RESOURCE, PARAM_STRICT, PARAM_VALUE,
};

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

/// Represents a parameter definition for a BRP tool
#[derive(Clone)]
pub struct Parameter {
    /// Parameter name as it appears in the API
    pub name:        &'static str,
    /// Description of the parameter
    pub description: &'static str,
    /// Whether this parameter is required
    pub required:    bool,
    /// Type of the parameter
    pub param_type:  ParamType,
}

impl Parameter {
    /// Standard port parameter (appears in 21+ tools)
    pub const fn port() -> Self {
        Self {
            name:        PARAM_PORT,
            description: DESC_PORT,
            required:    false,
            param_type:  ParamType::Number,
        }
    }

    /// Entity ID parameter with custom description
    pub const fn entity(description: &'static str, required: bool) -> Self {
        Self {
            name: PARAM_ENTITY,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Resource name parameter
    pub const fn resource(description: &'static str) -> Self {
        Self {
            name: PARAM_RESOURCE,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Components parameter
    pub const fn components(description: &'static str, required: bool) -> Self {
        Self {
            name: PARAM_COMPONENTS,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Path parameter for mutations
    pub const fn path(description: &'static str) -> Self {
        Self {
            name: PARAM_PATH,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Value parameter
    pub const fn value(description: &'static str, required: bool) -> Self {
        Self {
            name: PARAM_VALUE,
            description,
            required,
            param_type: ParamType::Any,
        }
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

    /// Strict parameter (used in query)
    pub const fn strict() -> Self {
        Self::boolean(
            PARAM_STRICT,
            "If true, returns error on unknown component types (default: false)",
            false,
        )
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
