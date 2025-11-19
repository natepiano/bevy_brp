use std::path::PathBuf;

// ============================================================================
// LARGE RESPONSE TOKEN CALCULATION CONSTANTS
// ============================================================================

/// Estimated characters per token for response size calculation
pub const CHARS_PER_TOKEN: usize = 4;

/// Default maximum tokens before saving to file (Claude Code MCP limitation)
/// Using 10,000 as a conservative buffer below the 25,000 hard limit
/// (MCP seems to count tokens differently than our 4 chars/token estimate)
pub const DEFAULT_MAX_RESPONSE_TOKENS: usize = 15_000;

/// Configuration for large response handling
#[derive(Clone)]
pub struct LargeResponseConfig {
    /// Prefix for generated filenames (e.g., "`brp_response`_", "`log_list`_")
    pub file_prefix: String,
    /// Token limit for responses
    pub max_tokens:  usize,
    /// Directory for temporary files
    pub temp_dir:    PathBuf,
}

impl Default for LargeResponseConfig {
    fn default() -> Self {
        Self {
            file_prefix: "mcp_response_".to_string(),
            max_tokens:  DEFAULT_MAX_RESPONSE_TOKENS,
            temp_dir:    std::env::temp_dir(),
        }
    }
}
