//! Parameter definitions for our MCP tools

use strum::{Display, EnumString};

use super::extraction::{FieldSpec, ParameterFieldType};

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
    pub const fn param_type(self) -> ParameterFieldType {
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
            | Self::Resource => ParameterFieldType::String,

            // Number parameters
            Self::DurationMs
            | Self::Entity
            | Self::OlderThanSeconds
            | Self::Parent
            | Self::TailLines
            | Self::WatchId => ParameterFieldType::Number,

            // Boolean parameters
            Self::Enabled | Self::Strict | Self::Verbose => ParameterFieldType::Boolean,

            // String array parameters
            Self::Keys
            | Self::Types
            | Self::WithCrates
            | Self::WithoutCrates
            | Self::WithTypes
            | Self::WithoutTypes => ParameterFieldType::StringArray,

            // Number array parameters
            Self::Entities => ParameterFieldType::NumberArray,

            // Any type parameters
            Self::Components | Self::Data | Self::Filter | Self::Value => ParameterFieldType::Any,

            // Special case
            Self::Params => ParameterFieldType::DynamicParams,
        }
    }
}

// Implement FieldSpec for ParameterName to enable new extraction system
impl FieldSpec<ParameterFieldType> for ParameterName {
    fn field_name(&self) -> &str {
        self.into() // strum converts to snake_case
    }

    fn field_type(&self) -> ParameterFieldType {
        // Now that ParamType is gone, just return the field type directly
        self.param_type()
    }
}
