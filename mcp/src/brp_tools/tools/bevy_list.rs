//! `bevy/list` tool - List components on an entity or all component types

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/list` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListParams {
    /// Optional entity ID to list components for - to list all types, do not pass entity parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<u64>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/list` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ListResult {
    /// The raw BRP response data - an array of component type names
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    result: Option<Value>,

    /// Count of components - computed from result array length
    #[to_metadata(result_operation = "count")]
    component_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Found {component_count} components")]
    message_template: String,
}
