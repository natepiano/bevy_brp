//! `brp_extras/set_gamepad_button` tool - Set a simulated gamepad button

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;
use crate::brp_tools::gamepad::GamepadButtonWrapper;

/// Parameters for the `brp_extras/set_gamepad_button` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SetGamepadButtonParams {
    /// Gamepad entity returned by `brp_extras_connect_gamepad`
    pub gamepad: u64,

    /// Button to set, such as `South`, `Start` or `DPadUp`
    pub button: GamepadButtonWrapper,

    /// Analog value in [0.0, 1.0]; 1.0 pressed, 0.0 released; stays until set again
    pub value: f32,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/set_gamepad_button` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct SetGamepadButtonResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Gamepad button set")]
    pub message_template: String,
}
