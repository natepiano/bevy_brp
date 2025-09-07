//! `brp_extras/set_window_title` tool - Change window title

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/set_window_title` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SetWindowTitleParams {
    /// The new title to set for the window
    pub title: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/set_window_title` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct SetWindowTitleResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Status of the operation
    #[to_metadata(result_operation = "extract_status")]
    pub status: String,

    /// The old window title
    #[to_metadata(result_operation = "extract_old_title")]
    pub old_title: String,

    /// The new window title
    #[to_metadata(result_operation = "extract_new_title")]
    pub new_title: String,

    /// Message template for formatting responses
    #[to_message(message_template = "Window title changed from '{old_title}' to '{new_title}'")]
    pub message_template: String,
}
