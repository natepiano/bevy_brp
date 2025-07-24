//! `bevy/destroy` tool - Destroy entities permanently

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/destroy` tool
#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DestroyParams {
    /// The entity ID to destroy
    pub entity: u64,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/destroy` tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DestroyResult {
    /// The raw BRP response data (empty for destroy)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,
}
