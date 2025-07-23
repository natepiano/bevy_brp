//! bevy/reparent tool - Change entity parents

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the bevy/reparent tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ReparentParams {
    /// Array of entity IDs to reparent
    #[to_metadata]
    pub entities: Vec<u64>,

    /// The new parent entity ID (omit to remove parent)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub parent: Option<u64>,

    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/reparent tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ReparentResult {
    /// The raw BRP response data (empty for reparent)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,
}
