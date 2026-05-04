// brp network constants
/// Default host for BRP connections
/// Using IPv4 address directly to avoid IPv6 connection issues
pub(super) const BRP_DEFAULT_HOST: &str = "127.0.0.1";
/// `bevy_brp_extras` prefix
pub(super) const BRP_EXTRAS_PREFIX: &str = "brp_extras/";
/// HTTP protocol for BRP connections
pub(super) const BRP_HTTP_PROTOCOL: &str = "http";
/// JSON-RPC path for BRP requests
pub(super) const BRP_JSONRPC_PATH: &str = "/jsonrpc";
/// Maximum characters of the request body to include in error reports
pub(super) const ERROR_BODY_PREVIEW_CHARS: usize = 500;
/// Timeout for standard (non-streaming) HTTP requests
pub(super) const HTTP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

// error constants
/// Basically we're trying to to access a field of a struct or a resource with the wrong path - here
/// is an example of what would be returned with -23501 when incorrectly trying to modify
/// `ClearColor`   "Error accessing element with .red access(offset 3): Expected variant field
/// access to access Struct variant, found a Tuple variant instead."
pub(super) const BRP_ERROR_ACCESS_ERROR: i32 = -23_501;
/// BRP error code for invalid request - can occur under multiple circumstances including
/// The underlying error is generally something like "Unknown component type" which our code will
/// turn into one of the following depending on what is happening:
/// - "Component '...' is registered but lacks Serialize and Deserialize traits required for spawn
///   operations..."
/// - "The struct accessed doesn't have a '...' field"
pub(super) const BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE: i32 = -23_402;
/// "Internal error" (JSON-RPC standard)
pub(super) const JSON_RPC_ERROR_INTERNAL_ERROR: i32 = -32_603;
/// "invalid type: ... expected ..." (parameter validation errors)
pub(super) const JSON_RPC_ERROR_INVALID_PARAMS: i32 = -32_602;
/// "Method '...' not found. This method requires the `bevy_brp_extras` crate to be added to your
/// Bevy app with the `BrpExtrasPlugin`"
pub const JSON_RPC_ERROR_METHOD_NOT_FOUND: i32 = -32_601;

// json-rpc constants
pub(super) const JSONRPC_DEFAULT_ID: u64 = 1;
pub(super) const JSONRPC_FIELD: &str = "jsonrpc";
pub(super) const JSONRPC_FIELD_ID: &str = "id";
pub(super) const JSONRPC_FIELD_METHOD: &str = "method";
pub(super) const JSONRPC_FIELD_PARAMS: &str = "params";
pub(super) const JSONRPC_VERSION: &str = "2.0";
