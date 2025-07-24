//! bevy/registry/schema tool - Get type schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the bevy/registry/schema tool
#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RegistrySchemaParams {
    /// Include only types from these crates (e.g., [`bevy_transform`, `my_game`])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_crates: Option<Value>,

    /// Include only types with these reflect traits (e.g., [`Component`, `Resource`])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_types: Option<Value>,

    /// Exclude types from these crates (e.g., [`bevy_render`, `bevy_pbr`])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub without_crates: Option<Value>,

    /// Exclude types with these reflect traits (e.g., [`RenderResource`])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub without_types: Option<Value>,

    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/registry/schema tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RegistrySchemaResult {
    /// The raw BRP response - array of type schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of types returned
    #[to_metadata(computed_from = "result", computed_operation = "count")]
    pub type_count: usize,
}
