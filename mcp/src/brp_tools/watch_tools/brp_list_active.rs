//! List all active watches

use serde::{Deserialize, Serialize};

use super::manager::WATCH_MANAGER;
use crate::error::Result;
use crate::response::LocalCallInfo;
use crate::tool::{HandlerContext, HandlerResponse, ToolFn, WithCallInfo};

/// Individual watch information
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct WatchInfo {
    /// Watch ID
    #[to_result]
    pub watch_id:   u32,
    /// Entity ID being watched
    #[to_result]
    pub entity_id:  u64,
    /// Type of watch (get/list)
    #[to_result]
    pub watch_type: String,
    /// Log file path
    #[to_result]
    pub log_path:   String,
    /// BRP port
    #[to_result]
    pub port:       u16,
}

/// Result from listing active watches
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListActiveWatchesResult {
    /// List of active watches
    #[to_result]
    pub watches: Vec<WatchInfo>,
}

pub struct BrpListActiveWatches;

impl ToolFn for BrpListActiveWatches {
    type Output = ListActiveWatchesResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        _ctx: &HandlerContext,
    ) -> HandlerResponse<(Self::CallInfoData, Result<Self::Output>)> {
        Box::pin(async move { Ok(handle_impl().await.with_call_info(LocalCallInfo)) })
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

    Ok(ListActiveWatchesResult { watches })
}
