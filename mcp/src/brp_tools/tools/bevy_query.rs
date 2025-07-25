//! `bevy/query` tool - Query entities by components

use bevy_brp_mcp_macros::ResultFieldPlacement;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/query` tool
#[derive(Deserialize, Serialize, JsonSchema, ResultFieldPlacement)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve. Properties: components (array), option
    /// (array), has (array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Object specifying which entities to query. Properties: with (array), without (array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,

    /// If true, returns error on unknown component types (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/query` tool
#[derive(Serialize, bevy_brp_mcp_macros::ResultFieldPlacement)]
pub struct QueryResult {
    /// The raw BRP response - array of entities with their components
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of entities returned
    #[to_metadata(result_operation = "count")]
    pub entity_count: usize,

    /// Total count of components across all entities
    #[to_metadata(result_operation = "count_query_components")]
    pub component_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Found {entity_count} entities")]
    pub message_template: String,
}
