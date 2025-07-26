//! `brp_extras/send_keys` tool - Send keyboard input

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/send_keys` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SendKeysParams {
    /// Array of key code names to send
    pub keys: Vec<String>,

    /// Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1
    /// minute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    #[to_call_info]
    pub port: Port,
}

/// Result for the `brp_extras/send_keys` tool
#[derive(Serialize, ResultStruct)]
pub struct SendKeysResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Keys that were sent
    #[to_metadata(result_operation = "extract_keys_sent")]
    pub keys_sent: Vec<String>,

    /// Duration in milliseconds
    #[to_metadata(result_operation = "extract_duration_ms")]
    pub duration_ms: u32,

    /// Message template for formatting responses
    #[to_message(message_template = "Sent {key_count} keys")]
    pub message_template: String,
}
