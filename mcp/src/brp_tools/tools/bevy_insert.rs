//! bevy/insert tool - Insert or replace components on entities

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the bevy/insert tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct InsertParams {
    /// The entity ID to insert components into
    #[to_metadata]
    pub entity: u64,

    /// Object containing component data to insert. Keys are component types, values are component
    /// data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w],
    /// not objects with named fields.
    #[to_result]
    pub components: Value,

    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/insert tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct InsertResult {
    /// The raw BRP response data (usually empty for insert)
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
}
