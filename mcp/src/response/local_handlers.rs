//! Data structures for local handler results
//!
//! These types represent the raw data returned by local handlers before formatting.
//! They are converted to JSON responses by the formatter based on tool definitions.

use serde::{Deserialize, Serialize};

/// Result from setting the tracing level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingLevelResult {
    /// The new tracing level that was set
    pub level:    String,
    /// The log file where trace output is written
    pub log_file: String,
}

/// Result from getting the trace log path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPathResult {
    /// Full path to the trace log file
    pub log_path:        String,
    /// Whether the log file currently exists
    pub exists:          bool,
    /// Size of the log file in bytes (if it exists)
    pub file_size_bytes: Option<u64>,
}

/// Individual log file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileInfo {
    /// The filename
    pub filename:   String,
    /// The app name extracted from the filename
    pub app_name:   String,
    /// Full path to the file (included in verbose mode)
    pub path:       Option<String>,
    /// Human-readable file size (included in verbose mode)
    pub size:       Option<String>,
    /// File size in bytes (included in verbose mode)
    pub size_bytes: Option<u64>,
    /// Creation time as ISO string (included in verbose mode)
    pub created:    Option<String>,
    /// Modification time as ISO string (included in verbose mode)
    pub modified:   Option<String>,
}

/// Result from listing log files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogListResult {
    /// List of log files found
    pub logs:           Vec<LogFileInfo>,
    /// Path to the temp directory containing logs
    pub temp_directory: String,
    /// Total count of log files
    pub count:          usize,
}

/// Result from reading a log file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContentResult {
    /// The filename that was read
    pub filename:            String,
    /// Full path to the file
    pub file_path:           String,
    /// Size of the file in bytes
    pub size_bytes:          u64,
    /// Human-readable file size
    pub size_human:          String,
    /// Number of lines read
    pub lines_read:          usize,
    /// The actual log content
    pub content:             String,
    /// Whether content was filtered by keyword
    pub filtered_by_keyword: bool,
    /// Whether tail mode was used
    pub tail_mode:           bool,
}

/// Result from cleaning up log files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    /// Number of files deleted
    pub deleted_count:      usize,
    /// List of deleted filenames
    pub deleted_files:      Vec<String>,
    /// App name filter that was applied (if any)
    pub app_name_filter:    Option<String>,
    /// Age filter in seconds that was applied (if any)
    pub older_than_seconds: Option<u32>,
}
