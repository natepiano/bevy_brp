//! `bevy/remove` tool - Remove components from entities

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/remove` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct RemoveParams {
    /// The entity ID to remove components from
    pub entity: u64,

    /// Array of component type names to remove
    pub components: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/remove` tool
#[derive(Serialize, ResultStruct)]
pub struct RemoveResult {
    /// The raw BRP response data (empty for remove)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Removed components from entity {entity}")]
    pub message_template: String,
}
