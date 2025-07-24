//! bevy/get_resource tool - Get resource data

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the bevy/get_resource tool
#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct GetResourceParams {
    /// The fully-qualified type name of the resource to get
    pub resource: String,

    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/get_resource tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct GetResourceResult {
    /// The raw BRP response containing the resource data
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,
}
