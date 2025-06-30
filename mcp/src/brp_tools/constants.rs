//! Constants used by BRP tools
//!
//! This module contains constants specific to BRP tool operations,
//! including JSON field names and parameter constants.

// ============================================================================
// JSON FIELD CONSTANTS
// ============================================================================

/// JSON field name constants for BRP responses
pub const JSON_FIELD_CODE: &str = "code";
pub const JSON_FIELD_COMPONENT: &str = "component";
pub const JSON_FIELD_COMPONENTS: &str = "components";
pub const JSON_FIELD_COUNT: &str = "count";
pub const JSON_FIELD_DATA: &str = "data";
pub const JSON_FIELD_DEBUG_INFO: &str = "debug_info";
pub const JSON_FIELD_DESTROYED_ENTITY: &str = "destroyed_entity";
pub const JSON_FIELD_ENTITIES: &str = "entities";
pub const JSON_FIELD_ENTITY: &str = "entity";
pub const JSON_FIELD_ERROR_CODE: &str = "error_code";
pub const JSON_FIELD_FORMAT_CORRECTIONS: &str = "format_corrections";
pub const JSON_FIELD_LOG_PATH: &str = "log_path";
pub const JSON_FIELD_METADATA: &str = "metadata";
pub const JSON_FIELD_METHOD: &str = "method";
pub const JSON_FIELD_ORIGINAL_ERROR: &str = "original_error";
pub const JSON_FIELD_PARENT: &str = "parent";
pub const JSON_FIELD_PATH: &str = "path";
pub const JSON_FIELD_PORT: &str = "port";
pub const JSON_FIELD_RESOURCE: &str = "resource";
pub const JSON_FIELD_RESOURCES: &str = "resources";
pub const JSON_FIELD_STATUS: &str = "status";
pub const JSON_FIELD_TIMEOUT_SECONDS: &str = "timeout_seconds";
pub const JSON_FIELD_VALUE: &str = "value";
pub const JSON_FIELD_WATCH_ID: &str = "watch_id";
pub const JSON_FIELD_WATCHES: &str = "watches";

// ============================================================================
// PARAMETER CONSTANTS
// ============================================================================

/// Parameter name constants for BRP tool inputs
pub const PARAM_TYPES: &str = "types";
pub const PARAM_METHOD: &str = "method";
pub const PARAM_PARAMS: &str = "params";
pub const PARAM_DATA: &str = "data";
pub const PARAM_FILTER: &str = "filter";
pub const PARAM_STRICT: &str = "strict";
pub const PARAM_FORMATS: &str = "formats";
pub const PARAM_WITH_CRATES: &str = "with_crates";
pub const PARAM_WITHOUT_CRATES: &str = "without_crates";
pub const PARAM_WITH_TYPES: &str = "with_types";
pub const PARAM_WITHOUT_TYPES: &str = "without_types";
pub const PARAM_ENTITIES: &str = "entities";
pub const PARAM_PARENT: &str = "parent";
pub const PARAM_RESULT: &str = "result";
pub const PARAM_ENTITY_COUNT: &str = "entity_count";
pub const PARAM_COMPONENT_COUNT: &str = "component_count";
pub const PARAM_QUERY_PARAMS: &str = "query_params";
pub const PARAM_SPAWNED_ENTITY: &str = "spawned_entity";

// ============================================================================
// NETWORK CONSTANTS
// ============================================================================

/// JSON-RPC path for BRP requests
pub const BRP_JSONRPC_PATH: &str = "/jsonrpc";

/// Default host for BRP connections
pub const BRP_DEFAULT_HOST: &str = "localhost";

/// HTTP protocol for BRP connections
pub const BRP_HTTP_PROTOCOL: &str = "http";

/// Documentation/Help Constants
pub const DESC_PORT: &str = "The BRP port (default: 15702)";

/// Network/Port Constants
pub const DEFAULT_BRP_PORT: u16 = 15702;

/// Environment variable name for BRP port
pub const BRP_PORT_ENV_VAR: &str = "BRP_PORT";

// ============================================================================
// ERROR CONSTANTS
// ============================================================================

/// BRP error code for invalid request
pub const BRP_ERROR_CODE_INVALID_REQUEST: i32 = -23402;

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
// Claude code MPC limitation: 25_000 tokens - but we're using heuristics so we buffer
// ============================================================================
