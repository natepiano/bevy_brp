//! `bevy/destroy` tool - Destroy entities permanently

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/destroy` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DestroyParams {
    /// The entity ID to destroy
    pub entity: u64,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/destroy` tool
#[derive(Serialize, ResultStruct)]
pub struct DestroyResult {
    /// The raw BRP response data (empty for destroy)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Destroyed entity {entity}")]
    message_template: String,
}
