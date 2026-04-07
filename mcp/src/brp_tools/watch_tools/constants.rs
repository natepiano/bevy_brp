/// Buffer size threshold that triggers a flush to disk
pub(super) const BUFFER_FLUSH_SIZE: usize = 4096;

/// Maximum bytes to include in debug preview of watch stream data
pub(super) const MAX_PREVIEW_BYTES: usize = 500;

/// Buffer capacity for batching log entries before writing
pub(super) const WATCH_LOG_BUFFER_SIZE: usize = 1000;

/// Interval between automatic log buffer flushes
pub(super) const WATCH_LOG_FLUSH_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(100);
