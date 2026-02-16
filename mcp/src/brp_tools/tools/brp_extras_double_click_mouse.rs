//! `brp_extras/double_click_mouse` tool - Perform double click

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;
use crate::brp_tools::types::MouseButtonWrapper;

/// Parameters for the `brp_extras/double_click_mouse` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DoubleClickMouseParams {
    /// Mouse button to double click (Left, Right, Middle, Back, Forward)
    pub button: MouseButtonWrapper,

    /// Delay in milliseconds between clicks (default: 250ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u32>,

    /// Optional window entity ID to target (defaults to primary window)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/double_click_mouse` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct DoubleClickMouseResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Double click executed successfully")]
    pub message_template: String,
}
