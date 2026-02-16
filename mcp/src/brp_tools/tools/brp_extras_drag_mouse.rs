//! `brp_extras/drag_mouse` tool - Drag mouse from start to end position

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;
use crate::brp_tools::types::MouseButtonWrapper;

/// Parameters for the `brp_extras/drag_mouse` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DragMouseParams {
    /// Mouse button to use for dragging (Left, Right, Middle, Back, Forward)
    pub button: MouseButtonWrapper,

    /// Starting position as [x, y]
    pub start: (f32, f32),

    /// Ending position as [x, y]
    pub end: (f32, f32),

    /// Number of frames over which to interpolate the drag
    pub frames: u32,

    /// Optional window entity ID to target (defaults to primary window)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/drag_mouse` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct DragMouseResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Drag operation started successfully")]
    pub message_template: String,
}
