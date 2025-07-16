//! List all active watches

use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::service::{HandlerContext, LocalContext};
use crate::tool::{HandlerResponse, HandlerResult, LocalToolFn};

/// Individual watch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchInfo {
    /// Watch ID
    pub watch_id:   u32,
    /// Entity ID being watched
    pub entity_id:  u64,
    /// Type of watch (get/list)
    pub watch_type: String,
    /// Log file path
    pub log_path:   String,
    /// BRP port
    pub port:       u16,
}

/// Result from listing active watches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListActiveWatchesResult {
    /// Status of the operation
    pub status:  String,
    /// Status message
    pub message: String,
    /// List of active watches
    pub watches: Vec<WatchInfo>,
    /// Total count of watches
    pub count:   usize,
}

impl HandlerResult for ListActiveWatchesResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct BrpListActiveWatches;

impl LocalToolFn for BrpListActiveWatches {
    fn call(&self, _ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_> {
        Box::pin(async move {
            handle_impl()
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl() -> std::result::Result<ListActiveWatchesResult, McpError> {
    // Get active watches from manager and release lock immediately
    let active_watches = {
        let manager = WATCH_MANAGER.lock().await;
        manager.list_active_watches()
    };

    // Convert to our typed format
    let watches: Vec<WatchInfo> = active_watches
        .iter()
        .map(|watch| WatchInfo {
            watch_id:   watch.watch_id,
            entity_id:  watch.entity_id,
            watch_type: watch.watch_type.clone(),
            log_path:   watch.log_path.to_string_lossy().to_string(),
            port:       watch.port,
        })
        .collect();

    let count = watches.len();
    Ok(ListActiveWatchesResult {
        status: "success".to_string(),
        message: format!("Found {count} active watches"),
        watches,
        count,
    })
}
