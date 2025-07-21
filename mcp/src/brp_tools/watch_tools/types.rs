//! Watch-related data types
//!
//! These types represent results from watch operations.

use serde::{Deserialize, Serialize};

/// Result from starting a watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStartResult {
    /// Watch ID
    pub watch_id: u32,
    /// Log path
    pub log_path: String,
}
