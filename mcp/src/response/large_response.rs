//! Large response handling utilities
//!
//! This module provides functionality to automatically save large responses
//! to temporary files when they exceed MCP token limits.
//!
//! At some point we should replace this with pagination.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use error_stack::ResultExt;
use serde_json::json;

use crate::error::{Error, Result};
use crate::response::builder::JsonResponse;

// ============================================================================
// LARGE RESPONSE TOKEN CALCULATION CONSTANTS
// ============================================================================

/// Estimated characters per token for response size calculation
pub const CHARS_PER_TOKEN: usize = 4;

/// Default maximum tokens before saving to file (Claude Code MCP limitation)
/// Using 10,000 as a conservative buffer below the 25,000 hard limit
/// (MCP seems to count tokens differently than our 4 chars/token estimate)
pub const DEFAULT_MAX_RESPONSE_TOKENS: usize = 9_000;

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

/// Check if response exceeds token limit and save result field to file if needed
pub fn handle_large_response(
    response: JsonResponse,
    identifier: &str,
    config: LargeResponseConfig,
) -> Result<JsonResponse> {
    // First check if the entire response is too large
    let response_json = response.to_json_fallback();
    let estimated_tokens = response_json.len() / CHARS_PER_TOKEN;

    if estimated_tokens > config.max_tokens {
        // Only extract and save the result field if it exists
        if let Some(result_field) = &response.result {
            // Generate timestamp for unique filename
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .change_context(Error::General("Failed to get timestamp".to_string()))?
                .as_secs();

            let sanitized_identifier = identifier.replace(['/', ' '], "_");
            let filename = format!(
                "{}{}{}.json",
                config.file_prefix, sanitized_identifier, timestamp
            );

            let filepath = config.temp_dir.join(&filename);

            // Save only the result field to file
            let result_json = serde_json::to_string_pretty(result_field).change_context(
                Error::General("Failed to serialize result field".to_string()),
            )?;

            fs::write(&filepath, &result_json).change_context(Error::FileOperation(format!(
                "Failed to write result to {}",
                filepath.display()
            )))?;

            // Create new response with result field replaced by file info
            let mut modified_response = response;
            modified_response.result = Some(json!({
                "saved_to_file": true,
                "filepath": filepath.to_string_lossy(),
                "instructions": "Use Read tool to examine, Grep to search, or jq commands to filter the data.",
                "original_size_tokens": estimated_tokens
            }));

            return Ok(modified_response);
        }
    }

    // Response is small enough or has no result field, return as-is
    Ok(response)
}
