//! Parameter definitions for our MCP tools

use serde::{Deserialize, Deserializer};
use strum::{Display, EnumString};

use crate::constants::VALID_PORT_RANGE;

/// Deserialize and validate port numbers
///
/// This function ensures that all port parameters are within the valid range (1024-65534).
/// It's used as a serde `deserialize_with` attribute on port fields.
pub fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let port = u16::deserialize(deserializer)?;

    if VALID_PORT_RANGE.contains(&port) {
        Ok(port)
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid port {}: must be in range {}-{}",
            port,
            VALID_PORT_RANGE.start(),
            VALID_PORT_RANGE.end()
        )))
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
