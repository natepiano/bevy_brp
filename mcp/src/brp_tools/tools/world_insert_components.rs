//! `world.insert_components` tool - Insert or replace components on entities

use std::collections::HashMap;

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.insert_components` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct InsertComponentsParams {
    /// The entity ID to insert components into
    pub entity: u64,

    /// Object containing component data to insert. Keys are component types, values are component
    pub components: HashMap<String, Value>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.insert_components` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(enhanced_errors = true)]
pub struct InsertComponentsResult {
    /// The raw BRP response data (usually empty for insert)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Inserted components into entity {entity}")]
    pub message_template: String,
}
