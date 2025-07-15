//! Large response handling utilities
//!
//! This module provides functionality to automatically save large responses
//! to temporary files when they exceed MCP token limits.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use error_stack::ResultExt;
use serde_json::{Value, json};

use crate::constants::{CHARS_PER_TOKEN, DEFAULT_MAX_RESPONSE_TOKENS};
use crate::error::{Error, Result};

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

/// Check if response exceeds token limit and save to file if needed
pub fn handle_large_response(
    response_data: &Value,
    identifier: &str,
    config: LargeResponseConfig,
) -> Result<Option<Value>> {
    let response_json = serde_json::to_string(response_data)
        .change_context(Error::General("Failed to serialize response".to_string()))?;

    let estimated_tokens = response_json.len() / CHARS_PER_TOKEN;

    if estimated_tokens > config.max_tokens {
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
