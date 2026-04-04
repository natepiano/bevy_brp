//! `brp_extras/pinch_gesture` tool - Send pinch gesture events

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/pinch_gesture` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct PinchGestureParams {
    /// Pinch delta value (positive = zoom in, negative = zoom out)
    pub delta: f32,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/pinch_gesture` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct PinchGestureResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Pinch gesture sent successfully")]
    pub message_template: String,
}
