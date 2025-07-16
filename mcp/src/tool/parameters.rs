//! Parameter definitions for our MCP tools

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
    /// A numeric parameter (typically entity IDs or ports)
    Number,
    /// An array of numbers
    NumberArray,
    /// A string parameter
    String,
    /// An array of strings
    StringArray,
}

/// Parameter names for BRP tools (excludes port parameter)
/// serialized into parameter names provided to the rcmp mcp tool framework
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
/// serialized into parameter names provided to the rcmp mcp tool framework
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
/// allows a common parameter pattern but ensures that each type can only work
/// with the parameters defined for it
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
/// only usable on `UnifiedToolDef` with BRP handlers
pub type BrpParameter = Parameter<BrpParameterName>;

/// Type alias for local tool parameters
/// only usable on `UnifiedToolDef` with local handlers
pub type LocalParameter = Parameter<LocalParameterName>;

/// Unified parameter enum that can hold either BRP or Local parameters
#[derive(Clone)]
pub enum UnifiedParameter {
    /// BRP parameter
    Brp(BrpParameter),
    /// Local parameter
    Local(LocalParameter),
}

/// Implement common methods for any Parameter<N> where N can convert to &str
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

/// BRP-specific constructors
/// if a description param is provided, it's because
/// these parameters are used in multiple tools and can have different descriptions
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
    pub const fn data() -> Self {
        Self::any(
            BrpParameterName::Data,
            "Object specifying what component data to retrieve. Properties: components (array), option (array), has (array)",
            true,
        )
    }

    /// Filter parameter for queries
    pub const fn filter() -> Self {
        Self::any(
            BrpParameterName::Filter,
            "Object specifying which entities to query. Properties: with (array), without (array)",
            false,
        )
    }

    /// Entities parameter for batch operations
    pub const fn entities(description: &'static str) -> Self {
        Self::number_array(BrpParameterName::Entities, description, true)
    }

    /// Parent parameter for reparenting
    pub const fn parent() -> Self {
        Self::number(
            BrpParameterName::Parent,
            "The new parent entity ID (omit to remove parent)",
            false,
        )
    }

    /// `With_crates` parameter for schema filtering
    pub const fn with_crates() -> Self {
        Self::string_array(
            BrpParameterName::WithCrates,
            "Include only types from these crates (e.g., [\"bevy_transform\", \"my_game\"])",
            false,
        )
    }

    /// `Without_crates` parameter for schema filtering
    pub const fn without_crates() -> Self {
        Self::string_array(
            BrpParameterName::WithoutCrates,
            "Exclude types from these crates (e.g., [\"bevy_render\", \"bevy_pbr\"])",
            false,
        )
    }

    /// `With_types` parameter for schema filtering
    pub const fn with_types() -> Self {
        Self::string_array(
            BrpParameterName::WithTypes,
            "Include only types with these reflect traits (e.g., [\"Component\", \"Resource\"])",
            false,
        )
    }

    /// `Without_types` parameter for schema filtering
    pub const fn without_types() -> Self {
        Self::string_array(
            BrpParameterName::WithoutTypes,
            "Exclude types with these reflect traits (e.g., [\"RenderResource\"])",
            false,
        )
    }

    /// Enabled parameter for boolean operations
    pub const fn enabled() -> Self {
        Self::boolean(
            BrpParameterName::Enabled,
            "Enable or disable debug mode for bevy_brp_extras plugin",
            true,
        )
    }

    /// Keys parameter for input simulation
    pub const fn keys() -> Self {
        Self::string_array(
            BrpParameterName::Keys,
            "Array of key code names to send",
            true,
        )
    }

    /// `Duration_ms` parameter for timing
    pub const fn duration_ms() -> Self {
        Self::number(
            BrpParameterName::DurationMs,
            "Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1)",
            false,
        )
    }

    /// Types parameter for discovery operations
    pub const fn types(description: &'static str, required: bool) -> Self {
        Self::string_array(BrpParameterName::Types, description, required)
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

    fn param_type(&self) -> &ParamType {
        self.param_type()
    }
}

// Local-specific constructors
impl LocalParameter {
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

    fn param_type(&self) -> &ParamType {
        self.param_type()
    }
}

impl ParameterDefinition for UnifiedParameter {
    fn name(&self) -> &str {
        match self {
            Self::Brp(param) => param.name(),
            Self::Local(param) => param.name(),
        }
    }

    fn required(&self) -> bool {
        match self {
            Self::Brp(param) => param.required(),
            Self::Local(param) => param.required(),
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Brp(param) => param.description(),
            Self::Local(param) => param.description(),
        }
    }

    fn param_type(&self) -> &ParamType {
        match self {
            Self::Brp(param) => param.param_type(),
            Self::Local(param) => param.param_type(),
        }
    }
}
