//! Parameter definitions for our MCP tools

use rmcp::Error as McpError;
use serde_json::Value;
use strum::{Display, EnumString};

/// Common interface for parameter definitions
/// Parameter names are generated automatically by strum's Display trait
pub trait ParameterDefinition {
    /// Get the parameter name as string
    fn name(&self) -> &str;

    /// Check if the parameter is required
    fn required(&self) -> bool;

    /// Get the parameter description
    fn description(&self) -> &'static str;

    /// Get the parameter type (we need to import `ParamType`)
    fn param_type(&self) -> &ParamType;
}

/// Types of parameters that can be defined
#[derive(Clone)]
pub enum ParamType {
    /// Any JSON value (object, array, etc.)
    Any,
    /// A boolean parameter
    Boolean,
    /// Dynamic parameters for `brp_execute` - the value becomes the BRP method parameters directly
    DynamicParams,
    /// A numeric parameter (typically entity IDs or ports)
    Number,
    /// An array of numbers
    NumberArray,
    /// A string parameter
    String,
    /// An array of strings
    StringArray,
}

/// Extracted parameter values with their types
#[derive(Debug, Clone)]
pub enum ExtractedValue {
    /// String value
    String(String),
    /// Numeric value (u64)
    Number(u64),
    /// Boolean value
    Boolean(bool),
    /// Array of strings
    StringArray(Vec<String>),
    /// Array of numbers
    NumberArray(Vec<u64>),
    /// Any JSON value
    Any(Value),
}

impl ExtractedValue {
    /// Convert to string, returning error if wrong type
    pub fn into_string(self) -> Result<String, McpError> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err(McpError::invalid_params("Expected string value", None)),
        }
    }

    /// Convert to u64, returning error if wrong type
    pub fn into_u64(self) -> Result<u64, McpError> {
        match self {
            Self::Number(n) => Ok(n),
            _ => Err(McpError::invalid_params("Expected number value", None)),
        }
    }

    /// Convert to u32, returning error if wrong type or out of range
    pub fn into_u32(self) -> Result<u32, McpError> {
        match self {
            Self::Number(n) => u32::try_from(n)
                .map_err(|_| McpError::invalid_params("Number value too large for u32", None)),
            _ => Err(McpError::invalid_params("Expected number value", None)),
        }
    }

    /// Convert to bool, returning error if wrong type
    pub fn into_bool(self) -> Result<bool, McpError> {
        match self {
            Self::Boolean(b) => Ok(b),
            _ => Err(McpError::invalid_params("Expected boolean value", None)),
        }
    }

    /// Convert to string array, returning error if wrong type
    pub fn into_string_array(self) -> Result<Vec<String>, McpError> {
        match self {
            Self::StringArray(arr) => Ok(arr),
            _ => Err(McpError::invalid_params(
                "Expected string array value",
                None,
            )),
        }
    }

    /// Convert to number array, returning error if wrong type
    pub fn into_number_array(self) -> Result<Vec<u64>, McpError> {
        match self {
            Self::NumberArray(arr) => Ok(arr),
            _ => Err(McpError::invalid_params(
                "Expected number array value",
                None,
            )),
        }
    }

    /// Convert to any JSON value, returning error if wrong type
    pub fn into_any(self) -> Result<Value, McpError> {
        match self {
            Self::Any(v) => Ok(v),
            _ => Err(McpError::invalid_params("Expected any JSON value", None)),
        }
    }
}

// Implement Into<ExtractedValue> for common types to support defaults
impl From<String> for ExtractedValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ExtractedValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<u64> for ExtractedValue {
    fn from(n: u64) -> Self {
        Self::Number(n)
    }
}

impl From<u32> for ExtractedValue {
    fn from(n: u32) -> Self {
        Self::Number(u64::from(n))
    }
}

impl From<u16> for ExtractedValue {
    fn from(n: u16) -> Self {
        Self::Number(u64::from(n))
    }
}

impl From<bool> for ExtractedValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<Vec<String>> for ExtractedValue {
    fn from(arr: Vec<String>) -> Self {
        Self::StringArray(arr)
    }
}

impl From<Vec<u64>> for ExtractedValue {
    fn from(arr: Vec<u64>) -> Self {
        Self::NumberArray(arr)
    }
}

impl From<Value> for ExtractedValue {
    fn from(v: Value) -> Self {
        Self::Any(v)
    }
}

/// Unified parameter names combining all BRP and local tool parameters
/// Entries are alphabetically sorted for easy maintenance
/// serialized into parameter names provided to the rcmp mcp tool framework
#[derive(Display, EnumString, Clone, Copy, Debug, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ParameterName {
    /// Application name
    AppName,
    /// Component type for mutations
    Component,
    /// Components parameter for operations
    Components,
    /// Data parameter for queries
    Data,
    /// Duration in milliseconds
    DurationMs,
    /// Boolean enabled flag
    Enabled,
    /// Multiple entities for batch operations
    Entities,
    /// Entity ID parameter
    Entity,
    /// Example name
    ExampleName,
    /// Log filename
    Filename,
    /// Filter parameter for queries
    Filter,
    /// Keys array for input simulation
    Keys,
    /// Keyword for filtering
    Keyword,
    /// Tracing level
    Level,
    /// Method name for dynamic execution
    Method,
    /// Age threshold in seconds
    OlderThanSeconds,
    /// Parameters for dynamic method execution
    Params,
    /// Parent entity for reparenting
    Parent,
    /// Path for field mutations or file paths
    Path,
    /// Build profile (debug/release)
    Profile,
    /// Resource type name parameter
    Resource,
    /// Strict mode flag for queries
    Strict,
    /// Number of lines to tail
    TailLines,
    /// Types parameter for discovery
    Types,
    /// Value for mutations and inserts
    Value,
    /// Verbose output flag
    Verbose,
    /// Watch ID for stopping watches
    WatchId,
    /// Include specific crates in schema
    WithCrates,
    /// Exclude specific crates from schema
    WithoutCrates,
    /// Include specific reflect types
    WithTypes,
    /// Exclude specific reflect types
    WithoutTypes,
}

impl ParameterName {
    /// Get the expected parameter type for this parameter
    pub const fn param_type(self) -> ParamType {
        match self {
            // String parameters
            Self::AppName
            | Self::Component
            | Self::ExampleName
            | Self::Filename
            | Self::Keyword
            | Self::Level
            | Self::Method
            | Self::Path
            | Self::Profile
            | Self::Resource => ParamType::String,

            // Number parameters
            Self::DurationMs
            | Self::Entity
            | Self::OlderThanSeconds
            | Self::Parent
            | Self::TailLines
            | Self::WatchId => ParamType::Number,

            // Boolean parameters
            Self::Enabled | Self::Strict | Self::Verbose => ParamType::Boolean,

            // String array parameters
            Self::Components
            | Self::Keys
            | Self::Types
            | Self::WithCrates
            | Self::WithoutCrates
            | Self::WithTypes
            | Self::WithoutTypes => ParamType::StringArray,

            // Number array parameters
            Self::Entities => ParamType::NumberArray,

            // Any type parameters
            Self::Data | Self::Filter | Self::Value => ParamType::Any,

            // Special case
            Self::Params => ParamType::DynamicParams,
        }
    }
}

/// Simple parameter definition using the unified `ParameterName` enum
#[derive(Clone)]
pub struct Parameter {
    /// Parameter name as enum variant
    name:        ParameterName,
    /// Description of the parameter
    description: &'static str,
    /// Whether this parameter is required
    required:    bool,
    /// Type of the parameter
    param_type:  ParamType,
}

/// Specifies whether a tool requires a port parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortParameter {
    /// Tool requires a port parameter
    Required,
    /// Tool does not use a port parameter
    NotUsed,
}

/// Implementation for the simple Parameter struct
impl Parameter {
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

/// Parameter constructor methods using the unified `ParameterName` enum
impl Parameter {
    /// Entity ID parameter with custom description
    pub const fn entity(description: &'static str, required: bool) -> Self {
        Self {
            name: ParameterName::Entity,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Resource name parameter
    pub const fn resource(description: &'static str) -> Self {
        Self {
            name: ParameterName::Resource,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Components parameter
    pub const fn components(description: &'static str, required: bool) -> Self {
        Self {
            name: ParameterName::Components,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Path parameter for mutations
    pub const fn path(description: &'static str) -> Self {
        Self {
            name: ParameterName::Path,
            description,
            required: true,
            param_type: ParamType::String,
        }
    }

    /// Value parameter
    pub const fn value(description: &'static str, required: bool) -> Self {
        Self {
            name: ParameterName::Value,
            description,
            required,
            param_type: ParamType::Any,
        }
    }

    /// Boolean parameter helper
    pub const fn boolean(name: ParameterName, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Boolean,
        }
    }

    /// Generic string parameter
    pub const fn string(name: ParameterName, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::String,
        }
    }

    /// String array parameter
    pub const fn string_array(
        name: ParameterName,
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
        name: ParameterName,
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
    pub const fn number(name: ParameterName, description: &'static str, required: bool) -> Self {
        Self {
            name,
            description,
            required,
            param_type: ParamType::Number,
        }
    }

    /// Any type parameter
    pub const fn any(name: ParameterName, description: &'static str, required: bool) -> Self {
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
            ParameterName::Strict,
            "If true, returns error on unknown component types (default: false)",
            false,
        )
    }

    /// Component parameter for mutations
    pub const fn component(description: &'static str) -> Self {
        Self::string(ParameterName::Component, description, true)
    }

    /// Data parameter for queries
    pub const fn data() -> Self {
        Self::any(
            ParameterName::Data,
            "Object specifying what component data to retrieve. Properties: components (array), option (array), has (array)",
            true,
        )
    }

    /// Filter parameter for queries
    pub const fn filter() -> Self {
        Self::any(
            ParameterName::Filter,
            "Object specifying which entities to query. Properties: with (array), without (array)",
            false,
        )
    }

    /// Entities parameter for batch operations
    pub const fn entities(description: &'static str) -> Self {
        Self::number_array(ParameterName::Entities, description, true)
    }

    /// Parent parameter for reparenting
    pub const fn parent() -> Self {
        Self::number(
            ParameterName::Parent,
            "The new parent entity ID (omit to remove parent)",
            false,
        )
    }

    /// `With_crates` parameter for schema filtering
    pub const fn with_crates() -> Self {
        Self::string_array(
            ParameterName::WithCrates,
            "Include only types from these crates (e.g., [\"bevy_transform\", \"my_game\"])",
            false,
        )
    }

    /// `Without_crates` parameter for schema filtering
    pub const fn without_crates() -> Self {
        Self::string_array(
            ParameterName::WithoutCrates,
            "Exclude types from these crates (e.g., [\"bevy_render\", \"bevy_pbr\"])",
            false,
        )
    }

    /// `With_types` parameter for schema filtering
    pub const fn with_types() -> Self {
        Self::string_array(
            ParameterName::WithTypes,
            "Include only types with these reflect traits (e.g., [\"Component\", \"Resource\"])",
            false,
        )
    }

    /// `Without_types` parameter for schema filtering
    pub const fn without_types() -> Self {
        Self::string_array(
            ParameterName::WithoutTypes,
            "Exclude types with these reflect traits (e.g., [\"RenderResource\"])",
            false,
        )
    }

    /// Enabled parameter for boolean operations
    pub const fn enabled() -> Self {
        Self::boolean(
            ParameterName::Enabled,
            "Enable or disable debug mode for bevy_brp_extras plugin",
            true,
        )
    }

    /// Keys parameter for input simulation
    pub const fn keys() -> Self {
        Self::string_array(ParameterName::Keys, "Array of key code names to send", true)
    }

    /// `Duration_ms` parameter for timing
    pub const fn duration_ms() -> Self {
        Self::number(
            ParameterName::DurationMs,
            "Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1)",
            false,
        )
    }

    /// Types parameter for discovery operations
    pub const fn types(description: &'static str, required: bool) -> Self {
        Self::string_array(ParameterName::Types, description, required)
    }

    /// Params parameter for dynamic execution
    pub const fn dynamic_params(description: &'static str, required: bool) -> Self {
        Self {
            name: ParameterName::Params,
            description,
            required,
            param_type: ParamType::DynamicParams,
        }
    }
}

impl ParameterDefinition for Parameter {
    fn name(&self) -> &str {
        self.name()
    }

    fn required(&self) -> bool {
        self.required()
    }

    fn description(&self) -> &'static str {
        self.description()
    }

    fn param_type(&self) -> &ParamType {
        self.param_type()
    }
}
