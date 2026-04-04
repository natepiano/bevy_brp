//! `world.get_components` tool - Get component data from entities

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.get_components` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct GetComponentsParams {
    /// The entity ID to get component data from
    pub entity: u64,

    /// Array of component types to retrieve. Each component must be a fully-qualified type name
    pub components: Vec<String>,

    /// If true, returns error on unknown component types (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.get_components` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct GetComponentsResult {
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
