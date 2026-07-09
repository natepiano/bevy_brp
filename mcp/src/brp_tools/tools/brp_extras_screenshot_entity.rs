//! `brp_extras/screenshot_entity` tool - PNG crop of a single UI node

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/screenshot_entity` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ScreenshotEntityParams {
    /// The entity ID to screenshot (must be a laid-out UI node)
    pub entity: u64,

    /// File path where the cropped screenshot should be saved
    pub path: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/screenshot_entity` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ScreenshotEntityResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Entity {entity} screenshot saved to {path}")]
    pub message_template: String,
}
