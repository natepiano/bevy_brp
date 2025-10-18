//! `world.reparent_entities` tool - Change entity parents

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.reparent_entities` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ReparentEntitiesParams {
    /// Array of entity IDs to reparent
    pub entities: Vec<u64>,

    /// The new parent entity ID (omit to remove parent)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub parent: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.reparent_entities` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ReparentEntitiesResult {
    /// The raw BRP response data (empty for reparent)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Reparented {entity_count} entities")]
    pub message_template: String,
}
