//! `bevy/list` tool - List components on an entity or all component types

use bevy_brp_mcp_macros::ResultFieldPlacement;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/list` tool
#[derive(Deserialize, Serialize, JsonSchema, ResultFieldPlacement)]
pub struct ListParams {
    /// Optional entity ID to list components for - to list all types, do not pass entity parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/list` tool
#[derive(Serialize, bevy_brp_mcp_macros::ResultFieldPlacement)]
pub struct ListResult {
    /// The raw BRP response data - an array of component type names
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of components - computed from result array length
    #[to_metadata(result_operation = "count")]
    pub component_count: usize,
}
