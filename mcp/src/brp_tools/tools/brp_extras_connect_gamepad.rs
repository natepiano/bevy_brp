//! `brp_extras/connect_gamepad` tool - Connect a simulated gamepad

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/connect_gamepad` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ConnectGamepadParams {
    /// Name reported for the pad (default: "Simulated gamepad (BRP)")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/connect_gamepad` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ConnectGamepadResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Simulated gamepad connected")]
    pub message_template: String,
}
