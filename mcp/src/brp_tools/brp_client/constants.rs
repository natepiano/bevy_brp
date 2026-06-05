// brp network constants
use std::time::Duration;
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
/// MIME type sent in the `Content-Type` header for BRP JSON-RPC requests
pub(super) const HTTP_CONTENT_TYPE_JSON: &str = "application/json";
/// HTTP header name carrying the request content type
pub(super) const HTTP_HEADER_CONTENT_TYPE: &str = "Content-Type";
/// Timeout for standard (non-streaming) HTTP requests
pub(super) const HTTP_REQUEST_TIMEOUT: Duration = std::time::Duration::from_secs(30);

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

// error parsing
pub(super) const ERROR_PATTERNS: &[&str] = &[
    r"Unknown component type: `([^`]+)`",
    r"([a-zA-Z0-9_:]+) is invalid:",
];

// format error details
pub(super) const FORMAT_ERROR_HELP_FIELD: &str = "help";
pub(super) const FORMAT_ERROR_HELP_MESSAGE: &str = "Unable to determine specific types that failed. Use the brp_type_guide tool to get spawn/insert/mutation information for the types you're working with.";
pub(super) const FORMAT_ERROR_ORIGINAL_ERROR_FIELD: &str = "original_error";
pub(super) const FORMAT_ERROR_SUGGESTED_ACTION: &str =
    "Check your BRP method parameters and ensure they match expected structure";
pub(super) const FORMAT_ERROR_SUGGESTED_ACTION_FIELD: &str = "suggested_action";
pub(super) const FORMAT_ERROR_TYPE_GUIDE_FIELD: &str = "type_guide";

// json-rpc constants
pub(super) const JSONRPC_DEFAULT_ID: u64 = 1;
pub(super) const JSONRPC_FIELD: &str = "jsonrpc";
pub(super) const JSONRPC_FIELD_ID: &str = "id";
pub(super) const JSONRPC_FIELD_METHOD: &str = "method";
pub(super) const JSONRPC_FIELD_PARAMS: &str = "params";
pub(super) const JSONRPC_VERSION: &str = "2.0";
