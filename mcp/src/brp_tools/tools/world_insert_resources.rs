//! `world.insert_resources` tool - Insert or update resources

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.insert_resources` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct InsertResourcesParams {
    /// The fully-qualified type name of the resource to insert or update
    pub resource: String,

    /// The resource value to insert.
    pub value: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.insert_resources` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(enhanced_errors = true)]
pub struct InsertResourcesResult {
    /// The raw BRP response data (empty for insert)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Inserted resource {resource}")]
    pub message_template: String,
}
