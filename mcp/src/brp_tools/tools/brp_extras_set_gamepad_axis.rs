//! `brp_extras/set_gamepad_axis` tool - Set a simulated gamepad axis

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/set_gamepad_axis` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SetGamepadAxisParams {
    /// Gamepad entity returned by `brp_extras_connect_gamepad`
    pub gamepad: u64,

    /// Axis name, a Bevy `GamepadAxis` variant such as `LeftStickX` or `RightZ`
    pub axis: String,

    /// Value in [-1.0, 1.0]; the axis stays there until set again
    pub value: f32,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/set_gamepad_axis` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct SetGamepadAxisResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Gamepad axis set")]
    pub message_template: String,
}
