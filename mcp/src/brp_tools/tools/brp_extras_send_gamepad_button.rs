//! `brp_extras/send_gamepad_button` tool - Set a simulated gamepad button

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/send_gamepad_button` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SendGamepadButtonParams {
    /// Gamepad entity returned by `brp_extras_connect_gamepad`
    pub gamepad: u64,

    /// Button name, a Bevy `GamepadButton` variant such as `South`, `Start` or `DPadUp`
    pub button: String,

    /// Analog value in [0.0, 1.0] (default: 1.0, pressed; 0.0 releases)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,

    /// Release after this many milliseconds on the real clock (default: hold until set again,
    /// max: 60000ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/send_gamepad_button` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct SendGamepadButtonResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Gamepad button set")]
    pub message_template: String,
}
