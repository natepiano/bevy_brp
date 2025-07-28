//! `bevy/get` tool - Get component data from entities

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `bevy/get` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct GetParams {
    /// The entity ID to get component data from
    pub entity: u64,

    /// Array of component types to retrieve. Each component must be a fully-qualified type name
    pub components: Value,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/get` tool
#[derive(Serialize, ResultStruct)]
pub struct GetResult {
    /// The raw BRP response with components and errors
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    result: Option<Value>,

    /// Count of components retrieved - computed from result.components object
    #[to_metadata(result_operation = "count_components")]
    component_count: usize,

    /// Count of errors if any components failed to retrieve
    #[to_metadata(skip_if_none, result_operation = "count_errors")]
    error_count: Option<usize>,

    /// Message template for formatting responses
    #[to_message(message_template = "Retrieved {component_count} components")]
    message_template: String,
}
