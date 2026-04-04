//! `brp_extras/type_text` tool - Type text sequentially (one char per frame)

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/type_text` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TypeTextParams {
    /// Text to type (supports letters, numbers, symbols, newlines, tabs)
    pub text: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/type_text` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct TypeTextResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Number of characters queued for typing
    #[to_metadata(result_operation = "extract_chars_queued")]
    pub chars_queued: usize,

    /// Characters that couldn't be mapped (skipped)
    #[to_metadata(result_operation = "extract_skipped")]
    pub skipped: Vec<char>,

    /// Message template for formatting responses
    #[to_message(message_template = "Queued {chars_queued} characters for typing")]
    pub message_template: String,
}
