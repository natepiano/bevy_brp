//! brp_extras/set_debug_mode tool - Enable/disable debug mode

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for the brp_extras/set_debug_mode tool
#[derive(Deserialize, Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct SetDebugModeParams {
    /// Enable or disable debug mode for `bevy_brp_extras` plugin
    pub enabled: bool,

    /// The BRP port (default: 15702)
    #[to_call_info]
    pub port: u16,
}

/// Result for the brp_extras/set_debug_mode tool
#[derive(Serialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct SetDebugModeResult {
    /// The raw BRP response
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Whether debug is now enabled
    #[to_metadata(computed_from = "result", computed_operation = "extract_debug_enabled")]
    pub debug_enabled: bool,

    /// Details message
    #[to_metadata(
        skip_if_none,
        computed_from = "result",
        computed_operation = "extract_message"
    )]
    pub details: Option<String>,
}
