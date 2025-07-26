//! `bevy/mutate_resource` tool - Mutate resource fields

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/mutate_resource` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct MutateResourceParams {
    /// The fully-qualified type name of the resource to mutate
    #[to_metadata]
    pub resource: String,

    /// The path to the field within the resource (e.g., 'settings.volume')
    pub path: String,

    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    #[to_call_info]
    pub port: Port,
}

/// Result for the `bevy/mutate_resource` tool
#[derive(Serialize, ResultStruct)]
pub struct MutateResourceResult {
    /// The raw BRP response data (empty for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Format corrections applied during mutation
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<Value>>,

    /// Status of format correction
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,

    /// Message template for formatting responses
    #[to_message(message_template = "Mutated resource {resource_name}")]
    pub message_template: String,
}
