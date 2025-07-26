//! List all active watches

use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::brp_tools::Port;
use crate::error::Result;
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

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
    pub port:       Port,
}

/// Result from listing active watches
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListActiveWatchesResult {
    /// List of active watches
    #[to_result]
    watches: Vec<WatchInfo>,

    /// Message template for formatting responses
    #[to_message(message_template = "Found {watch_count} active watches")]
    message_template: String,
}

pub struct BrpListActiveWatches;

impl ToolFn for BrpListActiveWatches {
    type Output = ListActiveWatchesResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        _ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move { Ok(ToolResult::from_result(handle_impl().await, LocalCallInfo)) })
    }
}

async fn handle_impl() -> Result<ListActiveWatchesResult> {
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

    Ok(ListActiveWatchesResult::new(watches))
}
