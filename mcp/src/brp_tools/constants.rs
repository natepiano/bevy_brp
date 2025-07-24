use std::ops::RangeInclusive;

use serde::{Deserialize, Deserializer};
use strum_macros::{Display, EnumString};

/// Format correction field names enum for type-safe field access
///
/// This enum provides compile-time safety for format correction field names
/// used throughout the BRP tools handler, response builder, and components.
/// Using strum's `IntoStaticStr` derive allows `.into()` to get string representation.
/// Using strum's `AsRefStr` derive allows `.as_ref()` to get string representation.
#[derive(Display, EnumString, Clone, Copy, Debug, strum::IntoStaticStr, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum FormatCorrectionField {
    /// Available mutation paths
    AvailablePaths,
    /// Error code
    Code,
    /// Component type being corrected
    Component,
    /// Corrected format to use instead
    CorrectedFormat,
    /// Error data in error responses
    ErrorData,
    /// Examples of valid usage
    Examples,
    /// Status indicating if format correction was applied
    FormatCorrected,
    /// Array of format corrections that were applied
    FormatCorrections,
    /// Human-readable hint for using the corrected format
    Hint,
    /// Available mutation paths for this component
    MutationPaths,
    /// Original error message when enhanced
    OriginalError,
    /// Original format that was incorrect
    OriginalFormat,
    /// Path for mutation operations
    Path,
    /// Operations supported by this component type
    SupportedOperations,
    /// Category of the component type (e.g., "Component", "Resource")
    TypeCategory,
    /// Valid values for enum fields
    ValidValues,
    /// Value for mutation operations
    Value,
}

/// Idle timeout for connection pool in seconds
pub const POOLE_IDLE_TIMEOUT: u64 = 300;

/// Maximum idle connections per host
pub const POOL_MAX_IDLE_PER_HOST: usize = 5;

/// default watch timeout
pub const DEFAULT_WATCH_TIMEOUT: u64 = 30;

/// Connection timeout in seconds
pub const CONNECTION_TIMEOUT: u64 = 30;

// ============================================================================
// NETWORK CONSTANTS
// ============================================================================

/// JSON-RPC path for BRP requests
pub const BRP_JSONRPC_PATH: &str = "/jsonrpc";

/// Default host for BRP connections
/// Using IPv4 address directly to avoid IPv6 connection issues
pub const BRP_DEFAULT_HOST: &str = "127.0.0.1";

/// HTTP protocol for BRP connections
pub const BRP_HTTP_PROTOCOL: &str = "http";

/// Network/Port Constants
pub const DEFAULT_BRP_PORT: u16 = 15702;

/// Environment variable name for BRP port
pub const BRP_PORT_ENV_VAR: &str = "BRP_PORT";

/// valid ports
pub const MIN_VALID_PORT: u16 = 1024; // Non-privileged ports start here
pub const MAX_VALID_PORT: u16 = 65534; // Leave room for calculations
pub const VALID_PORT_RANGE: RangeInclusive<u16> = MIN_VALID_PORT..=MAX_VALID_PORT;

/// Returns the default BRP port for serde default attribute
pub const fn default_port() -> u16 {
    DEFAULT_BRP_PORT
}

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

/// `bevy_brp_extras` prefix
pub const BRP_EXTRAS_PREFIX: &str = "brp_extras/";

// ============================================================================
// ERROR CONSTANTS
// ============================================================================

/// BRP error code for invalid request - can occur under multiple circumstances including
/// The underlying error is generally something like "Unknown component type" which our code will
/// turn into one of the following depending on what is happening:
/// - "Component '...' is registered but lacks Serialize and Deserialize traits required for spawn
///   operations..."
/// - "The struct accessed doesn't have a '...' field"
pub const BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE: i32 = -23402;
/// Basically we're trying to to access a field of a struct or a resource with the wrong path - here
/// is an example of what would be returned with -23501 when incorrectly trying to modify
/// `ClearColor`   "Error accessing element with .red access(offset 3): Expected variant field
/// access to access Struct variant, found a Tuple variant instead."
pub const BRP_ERROR_ACCESS_ERROR: i32 = -23501;
/// "Method '...' not found. This method requires the `bevy_brp_extras` crate to be added to your
/// Bevy app with the `BrpExtrasPlugin`"
pub const JSON_RPC_ERROR_METHOD_NOT_FOUND: i32 = -32601;
/// "invalid type: ... expected ..." (parameter validation errors)
pub const JSON_RPC_ERROR_INVALID_PARAMS: i32 = -32602;
/// "Internal error" (JSON-RPC standard)
pub const JSON_RPC_ERROR_INTERNAL_ERROR: i32 = -32603;

// ============================================================================
// JSON-RPC CONSTANTS
// ============================================================================

/// JSON-RPC protocol constants
pub const JSONRPC_VERSION: &str = "2.0";
pub const JSONRPC_DEFAULT_ID: u64 = 1;
pub const JSONRPC_FIELD: &str = "jsonrpc";
pub const JSONRPC_FIELD_ID: &str = "id";
pub const JSONRPC_FIELD_METHOD: &str = "method";
pub const JSONRPC_FIELD_PARAMS: &str = "params";
