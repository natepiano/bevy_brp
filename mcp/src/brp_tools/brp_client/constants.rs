// JSON-RPC 2.0 constants
pub const JSONRPC_VERSION: &str = "2.0";
pub const JSONRPC_DEFAULT_ID: u64 = 1;
pub const JSONRPC_FIELD: &str = "jsonrpc";
pub const JSONRPC_FIELD_ID: &str = "id";
pub const JSONRPC_FIELD_METHOD: &str = "method";
pub const JSONRPC_FIELD_PARAMS: &str = "params";

// Network constants for BRP client
/// JSON-RPC path for BRP requests
pub const BRP_JSONRPC_PATH: &str = "/jsonrpc";

/// Default host for BRP connections
/// Using IPv4 address directly to avoid IPv6 connection issues
pub const BRP_DEFAULT_HOST: &str = "127.0.0.1";

/// HTTP protocol for BRP connections
pub const BRP_HTTP_PROTOCOL: &str = "http";

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
