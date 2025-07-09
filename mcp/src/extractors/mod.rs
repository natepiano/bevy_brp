//! Extractors for handling MCP tool calls and Bevy BRP responses
//!
//! This module provides two main extractor types:
//! - `McpCallExtractor` - extracts data from MCP tool call arguments
//! - `BevyResponseExtractor` - extracts data from Bevy BRP responses

mod bevy_response;
mod mcp_call;

pub use bevy_response::BevyResponseExtractor;
pub use mcp_call::McpCallExtractor;
