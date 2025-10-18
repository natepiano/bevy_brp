//! `rpc.discover` tool - Discover available BRP methods

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `rpc.discover` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `rpc.discover` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct RpcDiscoverResult {
    /// The raw BRP response containing method discovery information
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of methods discovered
    #[to_metadata(result_operation = "count_methods")]
    pub method_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Discovered {method_count} methods")]
    pub message_template: String,
}
