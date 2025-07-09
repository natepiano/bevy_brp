use crate::response::LogPathResult;
use crate::support::tracing::get_trace_log_path;

/// Handle the `brp_get_trace_log_path` tool request
pub fn handle() -> LogPathResult {
    // Get the trace log path
    let log_path = get_trace_log_path();
    let log_path_str = log_path.to_string_lossy().to_string();

    // Check if the file exists and get its size
    let (exists, file_size_bytes) =
        std::fs::metadata(&log_path).map_or((false, None), |metadata| (true, Some(metadata.len())));

    LogPathResult {
        log_path: log_path_str,
        exists,
        file_size_bytes,
    }
}
