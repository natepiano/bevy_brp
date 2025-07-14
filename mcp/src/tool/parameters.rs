//! Parameter definitions for our MCP tools

use strum::{Display, EnumString};

// Note: Parameter names are now generated automatically by strum's Display trait
use crate::tool::tool_definition::ParameterDefinition;

/// Parameter names for BRP tools (excludes port parameter)
#[derive(Display, EnumString, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum BrpParameterName {
    /// Entity ID parameter
    Entity,
    /// Components parameter for operations
    Components,
    /// Resource type name parameter
    Resource,
    /// Path for field mutations
    Path,
    /// Value for mutations and inserts
    Value,
    /// Method name for dynamic execution
    Method,
    /// Strict mode flag for queries
    Strict,
    /// Component type for mutations
    Component,
    /// Data parameter for queries
    Data,
    /// Filter parameter for queries
    Filter,
    /// Parameters for dynamic method execution
    Params,
    /// Parent entity for reparenting
    Parent,
    /// Multiple entities for batch operations
    Entities,
    /// Types parameter for discovery
    Types,
    /// Include specific crates in schema
    WithCrates,
    /// Exclude specific crates from schema
    WithoutCrates,
    /// Include specific reflect types
    WithTypes,
    /// Exclude specific reflect types
    WithoutTypes,
    /// Boolean enabled flag
    Enabled,
    /// Keys array for input simulation
    Keys,
    /// Duration in milliseconds
    DurationMs,
    /// Watch ID for stopping watches
    WatchId,
    /// Log filename
    Filename,
    /// Keyword for filtering
    Keyword,
    /// Number of lines to tail
    TailLines,
    /// Age threshold in seconds
    OlderThanSeconds,
    /// Tracing level
    Level,
    /// Verbose output flag
    Verbose,
}

/// Parameter names for local tools (includes all parameters)
#[derive(Display, EnumString, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum LocalParameterName {
    /// Entity ID parameter
    Entity,
    /// Components parameter for operations
    Components,
    /// Resource type name parameter
    Resource,
    /// Path for field mutations or file paths
    Path,
    /// Value for mutations and inserts
    Value,
    /// Method name for dynamic execution
    Method,
    /// Port number for BRP connection
    Port,
    /// Build profile (debug/release)
    Profile,
    /// Application name
    AppName,
    /// Example name
    ExampleName,
    /// Component type for mutations
    Component,
    /// Data parameter for queries
    Data,
    /// Filter parameter for queries
    Filter,
    /// Parameters for dynamic method execution
    Params,
    /// Parent entity for reparenting
    Parent,
    /// Multiple entities for batch operations
    Entities,
    /// Types parameter for discovery
    Types,
    /// Include specific crates in schema
    WithCrates,
    /// Exclude specific crates from schema
    WithoutCrates,
    /// Include specific reflect types
    WithTypes,
    /// Exclude specific reflect types
    WithoutTypes,
    /// Boolean enabled flag
    Enabled,
    /// Keys array for input simulation
    Keys,
    /// Duration in milliseconds
    DurationMs,
    /// Watch ID for stopping watches
    WatchId,
    /// Log filename
    Filename,
    /// Keyword for filtering
    Keyword,
    /// Number of lines to tail
    TailLines,
    /// Age threshold in seconds
    OlderThanSeconds,
    /// Tracing level
    Level,
    /// Verbose output flag
    Verbose,
    /// Strict mode flag for queries
    Strict,
}

/// Generic parameter definition that works for both BRP and Local tools
#[derive(Clone)]
pub struct Parameter<N> {
    /// Parameter name as enum variant
    name:        N,
    /// Description of the parameter
    description: &'static str,
    /// Whether this parameter is required
    required:    bool,
    /// Type of the parameter
    param_type:  ParamType,
}

/// Type alias for BRP tool parameters
pub type BrpParameter = Parameter<BrpParameterName>;

/// Type alias for local tool parameters
pub type LocalParameter = Parameter<LocalParameterName>;

// Implement common methods for any Parameter<N> where N can convert to &str
impl<N> Parameter<N>
where
    N: Into<&'static str> + Copy,
{
    /// Get parameter name as string
    pub fn name(&self) -> &str {
        self.name.into()
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
}

// BRP-specific constructors
impl BrpParameter {
    /// Entity ID parameter with custom description
    pub const fn entity(description: &'static str, required: bool) -> Self {
        Self {
            name: BrpParameterName::Entity,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Resource name parameter
    pub const fn resource(description: &'static str) -> Self {
        Self {
            name: BrpParameterName::Resource,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Components parameter
    pub const fn components(description: &'static str, required: bool) -> Self {
        Self {
            name: BrpParameterName::Components,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Path parameter for mutations
    pub const fn path(description: &'static str) -> Self {
        Self {
            name: BrpParameterName::Path,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Value parameter
    pub const fn value(description: &'static str, required: bool) -> Self {
        Self {
            name: BrpParameterName::Value,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Boolean parameter helper
    pub const fn boolean(
        name: BrpParameterName,
        description: &'static str,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Boolean,
        }
    }

    /// Generic string parameter
    pub const fn string(name: BrpParameterName, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::String,
        }
    }

    /// String array parameter
    pub const fn string_array(
        name: BrpParameterName,
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

    /// Number array parameter
    pub const fn number_array(
        name: BrpParameterName,
        description: &'static str,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::NumberArray,
        }
    }

    /// Generic number parameter
    pub const fn number(name: BrpParameterName, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Any type parameter
    pub const fn any(name: BrpParameterName, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Strict parameter
    pub const fn strict() -> Self {
        Self::boolean(
            BrpParameterName::Strict,
            "If true, returns error on unknown component types (default: false)",
            false,
        )
    }

    /// Component parameter for mutations
    pub const fn component(description: &'static str) -> Self {
        Self::string(BrpParameterName::Component, description, true)
    }

    /// Data parameter for queries
    pub const fn data(description: &'static str) -> Self {
        Self::any(BrpParameterName::Data, description, true)
    }

    /// Filter parameter for queries
    pub const fn filter(description: &'static str) -> Self {
        Self::any(BrpParameterName::Filter, description, true)
    }

    /// Entities parameter for batch operations
    pub const fn entities(description: &'static str) -> Self {
        Self::number_array(BrpParameterName::Entities, description, true)
    }

    /// Parent parameter for reparenting
    pub const fn parent(description: &'static str, required: bool) -> Self {
        Self::number(BrpParameterName::Parent, description, required)
    }

    /// `With_crates` parameter for schema filtering
    pub const fn with_crates(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::WithCrates, description, required)
    }

    /// `Without_crates` parameter for schema filtering
    pub const fn without_crates(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::WithoutCrates, description, required)
    }

    /// `With_types` parameter for schema filtering
    pub const fn with_types(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::WithTypes, description, required)
    }

    /// `Without_types` parameter for schema filtering
    pub const fn without_types(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::WithoutTypes, description, required)
    }

    /// Enabled parameter for boolean operations
    pub const fn enabled(description: &'static str) -> Self {
        Self::boolean(BrpParameterName::Enabled, description, true)
    }

    /// Keys parameter for input simulation
    pub const fn keys(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::Keys, description, required)
    }

    /// `Duration_ms` parameter for timing
    pub const fn duration_ms(description: &'static str, required: bool) -> Self {
        Self::number(BrpParameterName::DurationMs, description, required)
    }

    /// Types parameter for discovery operations
    pub const fn types(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::Types, description, required)
    }

    /// Method parameter for dynamic execution
    pub const fn method(description: &'static str) -> Self {
        Self::string(BrpParameterName::Method, description, true)
    }

    /// Params parameter for dynamic execution
    pub const fn params(description: &'static str, required: bool) -> Self {
        Self::any(BrpParameterName::Params, description, required)
    }
}

impl ParameterDefinition for BrpParameter {
    fn name(&self) -> &str {
        self.name()
    }

    fn required(&self) -> bool {
        self.required()
    }

    fn description(&self) -> &'static str {
        self.description()
    }

    fn param_type(&self) -> &crate::tool::ParamType {
        self.param_type()
    }
}

// Local-specific constructors
impl LocalParameter {
    /// Port parameter
    pub const fn port() -> Self {
        Self {
            name:        LocalParameterName::Port,
            description: "The BRP port (default: 15702)",
            required:    false,
            param_type:  ParamType::Number,
        }
    }

    /// Generic string parameter
    pub const fn string(
        name: LocalParameterName,
        description: &'static str,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::String,
        }
    }

    /// Generic number parameter
    pub const fn number(
        name: LocalParameterName,
        description: &'static str,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Boolean parameter helper
    pub const fn boolean(
        name: LocalParameterName,
        description: &'static str,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Boolean,
        }
    }

    /// String array parameter
    pub const fn string_array(
        name: LocalParameterName,
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
}

impl ParameterDefinition for LocalParameter {
    fn name(&self) -> &str {
        self.name()
    }

    fn required(&self) -> bool {
        self.required()
    }

    fn description(&self) -> &'static str {
        self.description()
    }

    fn param_type(&self) -> &crate::tool::ParamType {
        self.param_type()
    }
}

/// Types of parameters that can be defined
#[derive(Clone)]
pub enum ParamType {
    /// Any JSON value (object, array, etc.)
    Any,
    /// A boolean parameter
    Boolean,
    /// A numeric parameter (typically entity IDs or ports)
    Number,
    /// An array of numbers
    NumberArray,
    /// A string parameter
    String,
    /// An array of strings
    StringArray,
}
