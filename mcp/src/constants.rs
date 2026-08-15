// error message prefixes
pub(crate) const MSG_FAILED_TO_PREFIX: &str = "Failed to";
pub(crate) const MSG_INVALID_PREFIX: &str = "Invalid";
pub(crate) const MSG_MISSING_PREFIX: &str = "Missing";

// json schema constants
/// JSON Schema reference prefix for type definitions.
pub(crate) const SCHEMA_REF_PREFIX: &str = "#/$defs/";

// tool list caching
/// Freshness hint attached to every `ListToolsResult`. Zero marks the tool list
/// immediately stale, so clients re-fetch it on each connection rather than
/// reusing a list that a rebuilt server binary may have changed.
pub(crate) const TOOL_LIST_CACHE_TTL_MS: u64 = 0;
