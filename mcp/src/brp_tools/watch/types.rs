//! Watch-related data types
//!
//! These types represent results from watch operations.

use serde::{Deserialize, Serialize};

use crate::tool::HandlerResult;

/// Result from starting a watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStartResult {
    /// Watch ID
    pub watch_id: u32,
    /// Log path
    pub log_path: String,
}

impl HandlerResult for WatchStartResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}
