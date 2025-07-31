//! `bevy/insert_resource` tool - Insert or update resources

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/insert_resource` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct InsertResourceParams {
    /// The fully-qualified type name of the resource to insert or update
    pub resource: String,

    /// The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/insert_resource` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(format_discovery = true)]
pub struct InsertResourceResult {
    /// The raw BRP response data (empty for insert)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Format corrections applied during execution
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<serde_json::Value>>,

    /// Whether format discovery was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,

    /// Warning message when format corrections were applied
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub warning: Option<String>,

    /// Message template for formatting responses
    #[to_message(message_template = "Inserted resource {resource_name}")]
    pub message_template: String,
}
