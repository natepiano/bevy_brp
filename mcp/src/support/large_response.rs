//! Large response handling utilities
//!
//! This module provides functionality to automatically save large responses
//! to temporary files when they exceed MCP token limits.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::error::{Error, Result};

/// Estimated characters per token for response size calculation
const CHARS_PER_TOKEN: usize = 4;

/// Default maximum tokens before saving to file (Claude Code MCP limitation)
/// Using 20,000 as a buffer below the 25,000 hard limit
const DEFAULT_MAX_RESPONSE_TOKENS: usize = 20_000;

/// Configuration for large response handling
pub struct LargeResponseConfig<'a> {
    /// Prefix for generated filenames (e.g., "`brp_response`_", "`log_list`_")
    pub file_prefix: &'a str,
    /// Optional custom token limit (defaults to `DEFAULT_MAX_RESPONSE_TOKENS`)
    pub max_tokens:  Option<usize>,
    /// Optional custom temp directory (defaults to `std::env::temp_dir()`)
    pub temp_dir:    Option<PathBuf>,
}

impl Default for LargeResponseConfig<'_> {
    fn default() -> Self {
        Self {
            file_prefix: "mcp_response_",
            max_tokens:  None,
            temp_dir:    None,
        }
    }
}

/// Check if response exceeds token limit and save to file if needed
///
/// Returns `Ok(Some(fallback_response))` if the response was saved to a file,
/// or Ok(None) if the response is small enough to return inline.
pub fn handle_large_response(
    response_data: &Value,
    identifier: &str,
    config: LargeResponseConfig,
) -> Result<Option<Value>> {
    use error_stack::ResultExt;

    let response_json = serde_json::to_string(response_data)
        .change_context(Error::General("Failed to serialize response".to_string()))?;

    let estimated_tokens = response_json.len() / CHARS_PER_TOKEN;
    let max_tokens = config.max_tokens.unwrap_or(DEFAULT_MAX_RESPONSE_TOKENS);

    if estimated_tokens > max_tokens {
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

        let temp_dir = config.temp_dir.unwrap_or_else(std::env::temp_dir);
        let filepath = temp_dir.join(&filename);

        // Save response to file
        fs::write(&filepath, &response_json).change_context(Error::FileOperation(format!(
            "Failed to write response to {}",
            filepath.display()
        )))?;

        // Return fallback response with file information
        let fallback_response = json!({
            "status": "success",
            "message": format!("Response too large ({estimated_tokens} tokens). Saved to {}", filepath.display()),
            "filepath": filepath.to_string_lossy(),
            "instructions": "Use Read tool to examine, Grep to search, or jq commands to filter the data."
        });

        Ok(Some(fallback_response))
    } else {
        Ok(None)
    }
}

/// Convenience function for BRP responses with default configuration
pub fn handle_brp_large_response(
    response_data: &Value,
    method_name: &str,
) -> Result<Option<Value>> {
    handle_large_response(
        response_data,
        method_name,
        LargeResponseConfig {
            file_prefix: "brp_response_",
            ..Default::default()
        },
    )
}
