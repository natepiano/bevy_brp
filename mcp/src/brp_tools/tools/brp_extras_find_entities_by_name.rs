//! `brp_extras/find_entities_by_name` tool - Locate entities by `Name` value

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `brp_extras/find_entities_by_name` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct FindEntitiesByNameParams {
    /// Name query pattern: `foo` (exact), `foo*` (starts with), `*foo` (ends
    /// with), `*foo*` (contains)
    pub name: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_extras/find_entities_by_name` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct FindEntitiesByNameResult {
    /// The raw BRP response - array of `{ entity, name }` matches
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of entities returned
    #[serde(rename = "entity_count")]
    #[to_metadata(result_operation = "count")]
    pub entities: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Found {entity_count} entities")]
    pub message_template: String,
}
