//! `brp_extras/send_gamepad_button` tool - Tap a simulated gamepad button

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;
use crate::brp_tools::gamepad::GamepadButtonWrapper;

/// Parameters for the `brp_extras/send_gamepad_button` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SendGamepadButtonParams {
    /// Gamepad entity returned by `brp_extras_connect_gamepad`
    pub gamepad: u64,

    /// Button to tap, such as `South`, `Start` or `DPadUp`
    pub button: GamepadButtonWrapper,

    /// Release after this many milliseconds on the real clock (default: 100ms, max: 60000ms)
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
    #[to_message(message_template = "Gamepad button sent")]
    pub message_template: String,
}
