//! Watch-related data types
//!
//! These types represent results from watch operations.

use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};

/// Result from starting a watch operation
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct WatchStartResult {
    /// Watch ID
    #[to_metadata]
    watch_id: u32,
    /// Log path
    #[to_metadata]
    log_path: String,

    /// Message template for formatting responses
    #[to_message(message_template = "Started watch {watch_id}")]
    message_template: String,
}
