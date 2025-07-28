//! `brp_extras/discover_format` tool - Discover component format information

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/discover_format` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DiscoverFormatParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,

    /// Enable debug information in the response (default: false)
    #[serde(default)]
    pub enable_debug_info: bool,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/discover_format` tool
#[derive(Serialize, ResultStruct)]
pub struct DiscoverFormatResult {
    /// The raw BRP response containing format information
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Discovered {type_count} formats")]
    pub message_template: String,
}
