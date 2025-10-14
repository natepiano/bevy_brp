//! `brp_extras/screenshot` tool - Capture screenshots

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/screenshot` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ScreenshotParams {
    /// File path where the screenshot should be saved
    pub path: String,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/screenshot` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ScreenshotResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Screenshot saved to {path}")]
    pub message_template: String,
}
