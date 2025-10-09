//! `world.spawn_entity` tool - Spawn entities with components

use std::collections::HashMap;

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.spawn_entity` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SpawnEntityParams {
    /// Object containing component data to spawn with. Keys are component types, values are
    /// component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat:
    /// [x,y,z,w], not objects with named fields.
    pub components: HashMap<String, Value>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.spawn_entity` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(enhanced_errors = true)]
pub struct SpawnEntityResult {
    /// The raw BRP response data containing the new entity ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// The spawned entity ID
    #[to_metadata(result_operation = "extract_entity")]
    pub entity: u64,

    /// Message template for formatting responses
    #[to_message(message_template = "Spawned entity {entity}")]
    pub message_template: String,
}
