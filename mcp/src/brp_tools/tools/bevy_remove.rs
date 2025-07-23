//! bevy/remove tool - Remove components from entities

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the bevy/remove tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RemoveParams {
    /// The entity ID to remove components from
    #[to_metadata]
    pub entity: u64,

    /// Array of component type names to remove
    #[to_result]
    pub components: Value,

    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/remove tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RemoveResult {
    /// The raw BRP response data (empty for remove)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,
}
