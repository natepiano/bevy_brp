//! `brp_extras/move_mouse` tool - Move mouse cursor

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/move_mouse` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct MoveMouseParams {
    /// Delta movement (relative) as [x, y]. Exactly one of delta or position must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<(f32, f32)>,

    /// Absolute position as [x, y]. Exactly one of delta or position must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<(f32, f32)>,

    /// Optional window entity ID to target (defaults to primary window)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/move_mouse` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct MoveMouseResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Mouse moved successfully")]
    pub message_template: String,
}
