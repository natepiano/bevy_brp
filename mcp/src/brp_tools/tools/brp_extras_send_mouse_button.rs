//! `brp_extras/send_mouse_button` tool - Send mouse button input

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;
use crate::brp_tools::types::MouseButtonWrapper;

/// Parameters for the `brp_extras/send_mouse_button` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SendMouseButtonParams {
    /// Mouse button to press (Left, Right, Middle, Back, Forward)
    pub button: MouseButtonWrapper,

    /// Duration in milliseconds to hold the button before releasing (default: 100ms, max: 60000ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,

    /// Optional window entity ID to target (defaults to primary window)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/send_mouse_button` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct SendMouseButtonResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Mouse button pressed successfully")]
    pub message_template: String,
}
