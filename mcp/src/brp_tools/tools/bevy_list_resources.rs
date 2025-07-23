//! bevy/list_resources tool - List all registered resources

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the bevy/list_resources tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListResourcesParams {
    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the bevy/list_resources tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListResourcesResult {
    /// The raw BRP response - array of resource type names
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of resources
    #[to_metadata(computed_from = "result", computed_operation = "count")]
    pub resource_count: usize,
}
