//! `brp_extras/get_diagnostics` tool - Get FPS diagnostics

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/get_diagnostics` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct GetDiagnosticsParams {
    /// Port number for BRP - defaults to 15702
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/get_diagnostics` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct GetDiagnosticsResult {
    /// The raw BRP response containing FPS diagnostics
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "FPS diagnostics retrieved")]
    pub message_template: String,
}
