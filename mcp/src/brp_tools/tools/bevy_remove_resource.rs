//! `bevy/remove_resource` tool - Remove resources

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/remove_resource` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct RemoveResourceParams {
    /// The fully-qualified type name of the resource to remove
    #[to_metadata]
    pub resource: String,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/remove_resource` tool
#[derive(Serialize, ResultStruct)]
pub struct RemoveResourceResult {
    /// The raw BRP response data (empty for remove)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Removed resource {resource_name}")]
    pub message_template: String,
}
