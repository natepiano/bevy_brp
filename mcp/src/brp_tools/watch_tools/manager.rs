//! Watch manager for coordinating file-based watch logging

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::info;

use crate::error::{Error, Result};

/// Global watch manager instance
pub static WATCH_MANAGER: std::sync::LazyLock<Arc<Mutex<WatchManager>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(WatchManager::new())));

/// Information about an active watch
#[derive(Debug, Clone)]
pub struct WatchInfo {
    pub watch_id:   u32,
    pub entity_id:  u64,
    pub watch_type: String,
    pub log_path:   PathBuf,
    pub port:       u16,
}

/// Manager for watch subscriptions
pub struct WatchManager {
    /// Monotonic counter for watch IDs
    next_watch_id:      AtomicU32,
    /// Active watches mapped by watch ID
    pub active_watches: HashMap<u32, (WatchInfo, JoinHandle<()>)>,
}

impl WatchManager {
    /// Create a new watch manager
    pub fn new() -> Self {
        Self {
            next_watch_id:  AtomicU32::new(1),
            active_watches: HashMap::new(),
        }
    }

    /// Get the next watch ID (monotonically increasing)
    pub fn next_id(&self) -> u32 {
        self.next_watch_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Stop a watch by ID
    pub fn stop_watch(&mut self, watch_id: u32) -> Result<()> {
        if let Some((info, handle)) = self.active_watches.remove(&watch_id) {
            info!("Stopping watch {} for entity {}", watch_id, info.entity_id);
            handle.abort();
            Ok(())
        } else {
            Err(error_stack::Report::new(Error::WatchOperation(format!(
                "Failed to stop watch {watch_id}: watch not found"
            ))))
        }
    }

    /// List all active watches
    pub fn list_active_watches(&self) -> Vec<WatchInfo> {
        self.active_watches
            .values()
            .map(|(info, _)| info.clone())
            .collect()
    }
}

/// Initialize the watch manager
pub async fn initialize_watch_manager() {
    info!("Watch manager initialized");
    // Force initialization of the lazy static
    let _ = WATCH_MANAGER.lock().await;
}
