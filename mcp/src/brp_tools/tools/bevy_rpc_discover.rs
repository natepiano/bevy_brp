//! rpc.discover tool - Discover available BRP methods

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the rpc.discover tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the rpc.discover tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RpcDiscoverResult {
    /// The raw BRP response containing method discovery information
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of methods discovered
    #[to_metadata(computed_from = "result", computed_operation = "count_methods")]
    pub method_count: usize,
}
