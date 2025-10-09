//! `world.get_resources` tool - Get resource data

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.get_resources` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct GetResourcesParams {
    /// The fully-qualified type name of the resource to get
    pub resource: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.get_resources` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct GetResourcesResult {
    /// The raw BRP response containing the resource data
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Retrieved {resource_name} resource")]
    pub message_template: String,
}
