//! `bevy/get` tool - Get component data from entities

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/get` tool
#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct GetParams {
    /// The entity ID to get component data from
    #[to_metadata]
    pub entity: u64,

    /// Array of component types to retrieve. Each component must be a fully-qualified type name
    pub components: Value,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/get` tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct GetResult {
    /// The raw BRP response with components and errors
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of components retrieved - computed from result.components object
    #[to_metadata(result_operation = "count_components")]
    pub component_count: usize,

    /// Count of errors if any components failed to retrieve
    #[to_metadata(skip_if_none, result_operation = "count_errors")]
    pub error_count: Option<usize>,
}
