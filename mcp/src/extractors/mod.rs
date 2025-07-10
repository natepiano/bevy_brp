//! Extractors for handling MCP tool calls and response data
//!
//! This module provides extractor functionality:
//! - `McpCallExtractor` - extracts data from MCP tool call arguments
//! - `ResponseExtractorType` - defines extraction strategies for response data

mod mcp_call;
mod types;

pub use mcp_call::McpCallExtractor;
pub use types::ExtractedParams;
