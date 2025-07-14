//! Parameter definitions for our MCP tools

use crate::constants::{
    PARAM_COMPONENTS, PARAM_ENTITY, PARAM_METHOD, PARAM_PATH, PARAM_PORT, PARAM_RESOURCE,
    PARAM_STRICT, PARAM_VALUE,
};
use crate::error::Error;

/// Represents a parameter definition for a BRP tool
#[derive(Clone)]
pub struct Parameter {
    /// Parameter name as it appears in the API
    name:        &'static str,
    /// Description of the parameter
    description: &'static str,
    /// Whether this parameter is required
    required:    bool,
    /// Type of the parameter
    param_type:  ParamType,
}

impl Parameter {
    /// Get parameter name
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Get parameter description
    pub const fn description(&self) -> &'static str {
        self.description
    }

    /// Get whether parameter is required
    pub const fn required(&self) -> bool {
        self.required
    }

    /// Get parameter type
    pub const fn param_type(&self) -> &ParamType {
        &self.param_type
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
