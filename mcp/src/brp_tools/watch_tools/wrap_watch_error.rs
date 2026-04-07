use crate::error::Error;

/// Wrap errors from watch operations with consistent formatting
pub(super) fn wrap_watch_error<E: std::fmt::Display>(
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
