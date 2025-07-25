//! Watch-related data types
//!
//! These types represent results from watch operations.

use bevy_brp_mcp_macros::ResultFieldPlacement;
use serde::{Deserialize, Serialize};

/// Result from starting a watch operation
#[derive(Debug, Clone, Serialize, Deserialize, ResultFieldPlacement)]
pub struct WatchStartResult {
    /// Watch ID
    #[to_metadata]
    pub watch_id: u32,
    /// Log path
    #[to_metadata]
    pub log_path: String,
}
