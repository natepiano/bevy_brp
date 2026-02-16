//! `brp_extras/scroll_mouse` tool - Send mouse wheel scroll events

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;
use crate::brp_tools::types::ScrollUnitWrapper;

/// Parameters for the `brp_extras/scroll_mouse` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ScrollMouseParams {
    /// Horizontal scroll amount
    pub x: f32,

    /// Vertical scroll amount
    pub y: f32,

    /// Scroll unit: "Line" or "Pixel"
    pub unit: ScrollUnitWrapper,

    /// Optional window entity ID to target (defaults to primary window)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/scroll_mouse` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ScrollMouseResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Scroll executed successfully")]
    pub message_template: String,
}
