//! `rpc.discover` tool - Discover available BRP methods

use bevy_brp_mcp_macros::ResultFieldPlacement;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{default_port, deserialize_port};

/// Parameters for the `rpc.discover` tool
#[derive(Deserialize, Serialize, JsonSchema, ResultFieldPlacement)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[serde(default = "default_port", deserialize_with = "deserialize_port")]
    #[to_call_info]
    pub port: u16,
}

/// Result for the `rpc.discover` tool
#[derive(Serialize, bevy_brp_mcp_macros::ResultFieldPlacement)]
pub struct RpcDiscoverResult {
    /// The raw BRP response containing method discovery information
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of methods discovered
    #[to_metadata(result_operation = "count_methods")]
    pub method_count: usize,
}
