//! Extractors for handling MCP tool calls and Bevy BRP responses
//!
//! This module provides two main extractor types:
//! - `McpCallExtractor` - extracts data from MCP tool call arguments
//! - `BevyResponseExtractor` - extracts data from Bevy BRP responses

mod bevy_response;
mod field_extractor;
mod mcp_call;
mod types;

pub use bevy_response::BevyResponseExtractor;
pub use field_extractor::{FieldExtractor, convert_extractor_type};
pub use mcp_call::McpCallExtractor;
pub use types::{ExtractedParams, ExtractorType, FormatterContext, ResponseField};
