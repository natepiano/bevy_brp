// Watch module

mod brp_list_active;
mod brp_stop_watch;
mod logger;
mod manager;
mod task;
mod types;
mod world_get_components_watch;
mod world_list_components_watch;

pub use task::{start_entity_watch_task, start_list_watch_task};

use crate::error::Error;

/// Wrap errors from watch operations with consistent formatting
pub fn wrap_watch_error<E: std::fmt::Display>(
    operation: &str,
    entity_id: Option<u64>,
    error: E,
) -> Error {
    let message = entity_id.map_or_else(
        || format!("{operation}: {error}"),
        |id| format!("{operation} for entity {id}: {error}"),
    );
    Error::WatchOperation(message)
}

pub use brp_list_active::BrpListActiveWatches;
pub use brp_stop_watch::{BrpStopWatch, StopWatchParams};
pub use manager::WatchManager;
pub use world_get_components_watch::{BevyGetWatch, GetWatchParams};
pub use world_list_components_watch::{BevyListWatch, ListWatchParams};
