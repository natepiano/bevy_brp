//! Extraction type definitions for the MCP tools extraction system.
//!
//! This module defines the types used to specify how data should be extracted
//! from different sources during tool execution.

use serde_json::Value;

/// Result of parameter extraction
pub struct ExtractedParams {
    /// The extracted parameters
    pub params: Option<Value>,
    /// The BRP port to use
    pub port:   u16,
}
