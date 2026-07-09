//! `brp_extras/snapshot` tool - Recursive YAML outline of a UI entity tree

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/snapshot` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct SnapshotParams {
    /// Entity ID to root the outline at (defaults to every top-level UI node)
    #[to_metadata(skip_if_none)]
    pub root: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/snapshot` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct SnapshotResult {
    /// The raw BRP response - `{ "yaml": "<outline>" }`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Captured UI snapshot")]
    pub message_template: String,
}
