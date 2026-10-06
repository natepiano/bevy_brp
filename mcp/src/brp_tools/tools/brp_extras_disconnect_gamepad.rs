//! `brp_extras/disconnect_gamepad` tool - Disconnect a simulated gamepad

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/disconnect_gamepad` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DisconnectGamepadParams {
    /// Gamepad entity returned by `brp_extras_connect_gamepad`
    pub gamepad: u64,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/disconnect_gamepad` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct DisconnectGamepadResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Simulated gamepad disconnected")]
    pub message_template: String,
}
