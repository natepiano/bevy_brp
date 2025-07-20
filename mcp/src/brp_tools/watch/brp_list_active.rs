//! List all active watches

use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::tool::{
    HandlerContext, HandlerResponse, HandlerResult, LocalToolFn, NoMethod, NoPort, ToolError,
    ToolResult,
};

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
    /// List of active watches
    pub watches: Vec<WatchInfo>,
}

impl HandlerResult for ListActiveWatchesResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct BrpListActiveWatches;

impl LocalToolFn for BrpListActiveWatches {
    fn call(&self, _ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<'_> {
        Box::pin(async move {
            let result = handle_impl().await;
            let tool_result = ToolResult(result);
            Ok(Box::new(tool_result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl() -> std::result::Result<ListActiveWatchesResult, ToolError> {
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

    Ok(ListActiveWatchesResult { watches })
}
