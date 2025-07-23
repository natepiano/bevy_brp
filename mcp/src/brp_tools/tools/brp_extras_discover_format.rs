//! brp_extras/discover_format tool - Discover component format information

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the brp_extras/discover_format tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DiscoverFormatParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,

    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the brp_extras/discover_format tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DiscoverFormatResult {
    /// The raw BRP response containing format information
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,
}
