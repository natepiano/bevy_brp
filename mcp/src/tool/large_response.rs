use std::path::PathBuf;

use super::constants::DEFAULT_MAX_RESPONSE_TOKENS;

/// Configuration for large response handling
#[derive(Clone)]
pub(super) struct LargeResponseConfig {
    /// Prefix for generated filenames (e.g., "`brp_response`_", "`log_list`_")
    pub(super) file_prefix: String,
    /// Token limit for responses
    pub(super) max_tokens:  usize,
    /// Directory for temporary files
    pub(super) temp_dir:    PathBuf,
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
