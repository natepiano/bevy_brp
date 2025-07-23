//! Watch-related data types
//!
//! These types represent results from watch operations.

use serde::{Deserialize, Serialize};

/// Result from starting a watch operation
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct WatchStartResult {
    /// Watch ID
    #[to_metadata]
    pub watch_id: u32,
    /// Log path
    #[to_metadata]
    pub log_path: String,
}
