//! `world.despawn_entity` tool - Despawn entities permanently

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.despawn_entity` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DespawnEntityParams {
    /// The entity ID to despawn
    pub entity: u64,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.despawn_entity` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct DespawnEntityResult {
    /// The raw BRP response data (empty for despawn)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Despawned entity {entity}")]
    message_template: String,
}
