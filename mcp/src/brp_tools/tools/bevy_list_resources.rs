//! `bevy/list_resources` tool - List all registered resources

use bevy_brp_mcp_macros::ResultFieldPlacement;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `bevy/list_resources` tool
#[derive(Deserialize, Serialize, JsonSchema, ResultFieldPlacement)]
pub struct ListResourcesParams {
    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `bevy/list_resources` tool
#[derive(Serialize, ResultFieldPlacement)]
pub struct ListResourcesResult {
    /// The raw BRP response - array of resource type names
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of resources
    #[to_metadata(result_operation = "count")]
    pub resource_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Found {resource_count} resources")]
    pub message_template: String,
}
