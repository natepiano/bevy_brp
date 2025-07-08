// Parameter name constants used across modules
pub const PARAM_BINARY_PATH: &str = "path";

// ---- large response token calculation constants ----

/// Estimated characters per token for response size calculation
pub const CHARS_PER_TOKEN: usize = 4;

/// Default maximum tokens before saving to file (Claude Code MCP limitation)
/// Using 20,000 as a buffer below the 25,000 hard limit
pub const DEFAULT_MAX_RESPONSE_TOKENS: usize = 20_000;
