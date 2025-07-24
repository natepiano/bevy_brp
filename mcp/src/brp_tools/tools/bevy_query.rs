//! bevy/query tool - Query entities by components

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the bevy/query tool
#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
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
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/query tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct QueryResult {
    /// The raw BRP response - array of entities with their components
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of entities returned
    #[to_metadata(computed_from = "result", computed_operation = "count")]
    pub entity_count: usize,

    /// Total count of components across all entities
    #[to_metadata(
        computed_from = "result",
        computed_operation = "count_query_components"
    )]
    pub component_count: usize,
}
