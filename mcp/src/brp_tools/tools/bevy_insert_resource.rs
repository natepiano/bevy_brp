//! `bevy/insert_resource` tool - Insert or update resources

use bevy_brp_mcp_macros::ResultFieldPlacement;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/insert_resource` tool
#[derive(Deserialize, Serialize, JsonSchema, ResultFieldPlacement)]
pub struct InsertResourceParams {
    /// The fully-qualified type name of the resource to insert or update
    #[to_metadata]
    pub resource: String,

    /// The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value: Value,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/insert_resource` tool
#[derive(Serialize, ResultFieldPlacement)]
pub struct InsertResourceResult {
    /// The raw BRP response data (empty for insert)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Format corrections applied during insertion
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<Value>>,

    /// Status of format correction
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,

    /// Message template for formatting responses
    #[to_message(message_template = "Inserted resource {resource_name}")]
    pub message_template: String,
}
