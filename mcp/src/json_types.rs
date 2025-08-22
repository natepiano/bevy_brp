//! JSON schema type definitions for MCP tools
//!
//! This module provides standardized JSON schema type names used across
//! the MCP tool ecosystem for parameter and response schema generation.

use serde::Serialize;
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

/// JSON schema type names for type schema generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, Serialize, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JsonSchemaType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
    Null,
}

impl From<JsonSchemaType> for Value {
    fn from(schema_type: JsonSchemaType) -> Self {
        Self::String(schema_type.as_ref().to_string())
    }
}
