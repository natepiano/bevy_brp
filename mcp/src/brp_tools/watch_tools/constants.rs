// buffer constants
use std::time::Duration;
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

// debug response fields
pub(super) const BUFFER_CONTENT_FIELD: &str = "buffer_content";
pub(super) const BUFFER_SIZE_FIELD: &str = "buffer_size";
pub(super) const CHUNK_SIZE_FIELD: &str = "chunk_size";
pub(super) const CHUNKS_RECEIVED_BEFORE_ERROR_FIELD: &str = "chunks_received_before_error";
pub(super) const CONTAINS_DATA_PREFIX_FIELD: &str = "contains_data_prefix";
pub(super) const CONTAINS_NEWLINE_FIELD: &str = "contains_newline";
pub(super) const CONTENT_TYPE_FIELD: &str = "content_type";
pub(super) const DATA_LENGTH_FIELD: &str = "data_length";
pub(super) const ELAPSED_SECONDS_FIELD: &str = "elapsed_seconds";
pub(super) const EMPTY_LINES_FIELD: &str = "empty_lines";
pub(super) const ENTITY_FIELD: &str = "entity";
pub(super) const ERROR_FIELD: &str = "error";
pub(super) const FINAL_BUFFER_SIZE_FIELD: &str = "final_buffer_size";
pub(super) const FULL_DATA_FIELD: &str = "full_data";
pub(super) const HAD_INCOMPLETE_LINE_FIELD: &str = "had_incomplete_line";
pub(super) const HAS_ERROR_FIELD: &str = "has_error";
pub(super) const HAS_ID_FIELD: &str = "has_id";
pub(super) const HAS_RESULT_FIELD: &str = "has_result";
pub(super) const HEADERS_COUNT_FIELD: &str = "headers_count";
pub(super) const IS_SSE_DATA_FIELD: &str = "is_sse_data";
pub(super) const JSON_KEYS_FIELD: &str = "json_keys";
pub(super) const LINE_BUFFER_SIZE_BEFORE_FIELD: &str = "line_buffer_size_before";
pub(super) const LINE_FIELD: &str = "line";
pub(super) const LINE_LENGTH_FIELD: &str = "line_length";
pub(super) const LINES_PROCESSED_FIELD: &str = "lines_processed";
pub(super) const PREVIEW_FIELD: &str = "preview";
pub(super) const RAW_DATA_FIELD: &str = "raw_data";
pub(super) const REMAINING_BUFFER_SIZE_FIELD: &str = "remaining_buffer_size";
pub(super) const RESPONSE_STATUS_FIELD: &str = "response_status";
pub(super) const STARTS_WITH_DATA_FIELD: &str = "starts_with_data";
pub(super) const STATUS_FIELD: &str = "status";
pub(super) const STATUS_TEXT_FIELD: &str = "status_text";
pub(super) const TIMESTAMP_FIELD: &str = "timestamp";
pub(super) const TOTAL_BUFFER_SIZE_BEFORE_FIELD: &str = "total_buffer_size_before";
pub(super) const TOTAL_CHUNKS_RECEIVED_FIELD: &str = "total_chunks_received";
pub(super) const UNKNOWN_STATUS_TEXT: &str = "Unknown";
pub(super) const WATCH_TYPE_FIELD: &str = "watch_type";

// preview constants
/// Maximum bytes to include in debug preview of watch stream data
pub(super) const MAX_PREVIEW_BYTES: usize = 500;

// response fields
pub(super) const CONTENT_TYPE_HEADER: &str = "content-type";
pub(super) const JSON_RPC_ERROR_FIELD: &str = "error";
pub(super) const JSON_RPC_ID_FIELD: &str = "id";
pub(super) const JSON_RPC_RESULT_FIELD: &str = "result";

// sse stream constants
/// Canonical Server-Sent Events `data:` line prefix (including trailing space).
pub(super) const SSE_DATA_PREFIX: &str = "data: ";

// timing constants
/// Interval between automatic log buffer flushes
pub(super) const WATCH_LOG_FLUSH_INTERVAL: Duration = std::time::Duration::from_millis(100);
