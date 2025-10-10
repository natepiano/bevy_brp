//! `world.mutate_resources` tool - Mutate resource fields

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.mutate_resources` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct MutateResourcesParams {
    /// The fully-qualified type name of the resource to mutate
    pub resource: String,

    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value: Value,

    /// The path to the field within the resource (e.g., 'settings.volume')
    #[serde(default)]
    pub path: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.mutate_resources` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(enhanced_errors = true)]
pub struct MutateResourcesResult {
    /// The raw BRP response data (empty for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Mutated resource {resource}")]
    pub message_template: String,
}
