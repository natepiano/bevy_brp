// Watch module

pub mod bevy_get_watch;
pub mod bevy_list_watch;
pub mod brp_list_active;
pub mod brp_stop_watch;
mod logger;
pub mod manager;
mod task;
mod types;

pub use brp_list_active::WatchInfo;
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
