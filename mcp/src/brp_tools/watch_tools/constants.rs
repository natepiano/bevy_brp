// buffer constants
/// Buffer size threshold that triggers a flush to disk
pub(super) const BUFFER_FLUSH_SIZE: usize = 4096;
/// Maximum size for the total buffer when processing incomplete lines (10MB)
pub(super) const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;
/// Maximum size for a single chunk in the SSE stream (1MB)
pub(super) const MAX_CHUNK_SIZE: usize = 1024 * 1024;
/// Initial capacity for the string buffer used to batch log writes
pub(super) const WATCH_LOG_BUFFER_CAPACITY: usize = 8192;
/// Buffer capacity for batching log entries before writing
pub(super) const WATCH_LOG_BUFFER_SIZE: usize = 1000;

// preview constants
/// Maximum bytes to include in debug preview of watch stream data
pub(super) const MAX_PREVIEW_BYTES: usize = 500;

// timing constants
/// Interval between automatic log buffer flushes
pub(super) const WATCH_LOG_FLUSH_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(100);
