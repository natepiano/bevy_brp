//! `brp_extras/double_tap_gesture` tool - Send double tap gesture events

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/double_tap_gesture` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DoubleTapGestureParams {
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/double_tap_gesture` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct DoubleTapGestureResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Double tap gesture sent successfully")]
    pub message_template: String,
}
