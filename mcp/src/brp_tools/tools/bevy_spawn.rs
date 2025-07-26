//! `bevy/spawn` tool - Spawn entities with components

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/spawn` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SpawnParams {
    /// Object containing component data to spawn with. Keys are component types, values are
    /// component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat:
    /// [x,y,z,w], not objects with named fields.
    pub components: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    #[to_call_info]
    pub port: Port,
}

/// Result for the `bevy/spawn` tool
#[derive(Serialize, ResultStruct)]
pub struct SpawnResult {
    /// The raw BRP response data containing the new entity ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// The spawned entity ID
    #[to_metadata(result_operation = "extract_entity")]
    pub entity: u64,

    /// Format corrections applied during spawn
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<Value>>,

    /// Status of format correction
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,

    /// Message template for formatting responses
    #[to_message(message_template = "Spawned entity {entity}")]
    pub message_template: String,
}
