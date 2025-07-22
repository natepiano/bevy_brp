//! Constants for Local Tools, Brp, Parameters and more
//!
//! This module contains constants specific to BRP tool operations,
//! including JSON field names and parameter constants.
use std::ops::RangeInclusive;

// ============================================================================
// JSON FIELD CONSTANTS
// ============================================================================

/// JSON field name constants for BRP responses
pub const RESPONSE_DEBUG_INFO: &str = "debug_info";
// These are now replaced by FormatCorrectionField enum usage directly
// pub const RESPONSE_FORMAT_CORRECTIONS: &str = "format_corrections";
// pub const RESPONSE_FORMAT_CORRECTED: &str = "format_corrected";
pub const RESPONSE_METADATA: &str = "metadata";

// ============================================================================
// TOOL PARAMETER CONSTANTS
// ============================================================================

/// Parameter name constants for BRP tool inputs
pub const PARAM_PARAMS: &str = "params";

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

// ============================================================================
// LARGE RESPONSE TOKEN CALCULATION CONSTANTS
// ============================================================================

/// Estimated characters per token for response size calculation
pub const CHARS_PER_TOKEN: usize = 4;

/// Default maximum tokens before saving to file (Claude Code MCP limitation)
/// Using 10,000 as a conservative buffer below the 25,000 hard limit
/// (MCP seems to count tokens differently than our 4 chars/token estimate)
pub const DEFAULT_MAX_RESPONSE_TOKENS: usize = 9_000;
