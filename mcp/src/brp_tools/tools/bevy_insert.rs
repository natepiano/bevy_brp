//! `bevy/insert` tool - Insert or replace components on entities

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/insert` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct InsertParams {
    /// The entity ID to insert components into
    pub entity: u64,

    /// Object containing component data to insert. Keys are component types, values are component
    /// data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w],
    /// not objects with named fields.
    pub components: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/insert` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(format_discovery = true)]
pub struct InsertResult {
    /// The raw BRP response data (usually empty for insert)
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
    #[to_message(message_template = "Inserted components into entity {entity}")]
    pub message_template: String,
}

/// Error type for insert format discovery failures
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct InsertFormatError {
    #[to_error_info]
    pub entity: u64,

    #[to_error_info]
    pub components: Value,

    #[to_error_info(skip_if_none)]
    pub format_corrections: Option<Vec<Value>>,

    #[to_error_info]
    pub brp_error_code: i32,

    #[to_error_info(skip_if_none)]
    pub original_error: Option<String>,

    #[to_message]
    pub message_template: Option<String>,
}
