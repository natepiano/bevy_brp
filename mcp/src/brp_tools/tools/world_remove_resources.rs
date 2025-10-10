//! `world.remove_resources` tool - Remove resources

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.remove_resources` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct RemoveResourcesParams {
    /// The fully-qualified type name of the resource to remove
    pub resource: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.remove_resources` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct RemoveResourcesResult {
    /// The raw BRP response data (empty for remove)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Removed resource {resource}")]
    pub message_template: String,
}
