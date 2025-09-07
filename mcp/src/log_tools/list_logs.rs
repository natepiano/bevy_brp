use bevy_brp_mcp_macros::{ParamStruct, ResultStruct, ToolFn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::support::LogFileEntry;
use crate::error::{Error, Result};
use crate::log_tools::support;
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListLogsParams {
    /// Optional filter to list logs for a specific app only
    #[to_metadata(skip_if_none)]
    pub app_name: Option<String>,
    /// Include full details (path, timestamps, size in bytes). Default is false for minimal output
    #[to_metadata(skip_if_none)]
    pub verbose:  Option<bool>,
}

/// Result from listing log files
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct ListLogResult {
    /// List of log files found
    #[to_result]
    logs:             Vec<LogFileInfo>,
    /// Path to the temp directory containing logs
    #[to_metadata]
    temp_directory:   String,
    /// Log file count
    #[to_metadata]
    log_count:        usize,
    /// Message template for formatting responses
    #[to_message(message_template = "Found {log_count} log files")]
    message_template: String,
}

#[derive(ToolFn)]
#[tool_fn(params = "ListLogsParams", output = "ListLogResult")]
pub struct ListLogs;

async fn handle_impl(params: ListLogsParams) -> Result<ListLogResult> {
    let logs = list_log_files(params.app_name.as_deref(), params.verbose)?;
    Ok(ListLogResult::new(
        logs.clone(),
        support::get_log_directory().display().to_string(),
        logs.len(),
    ))
}

fn list_log_files(
    app_name_filter: Option<&str>,
    verbose: Option<bool>,
) -> Result<Vec<LogFileInfo>> {
    // Use the iterator to get all log files with optional filter
    let filter = |entry: &LogFileEntry| -> bool {
        // Apply app name filter if provided
        app_name_filter.map_or_else(|| true, |app_filter| entry.app_name == app_filter)
    };

    let mut log_entries =
        support::iterate_log_files(filter).map_err(|e| Error::tool_call_failed(e.to_string()))?;

    // Sort by timestamp (newest first)
    log_entries.sort_by(|a, b| {
        let ts_a = a.timestamp.parse::<u128>().unwrap_or(0);
        let ts_b = b.timestamp.parse::<u128>().unwrap_or(0);
        ts_b.cmp(&ts_a)
    });

    // Convert to LogFileInfo structs
    let use_verbose = verbose.unwrap_or(false);
    let log_infos: Vec<LogFileInfo> = log_entries
        .into_iter()
        .map(|entry| {
            if use_verbose {
                let size_bytes = entry.metadata.len();
                let modified = entry.metadata.modified().ok().map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                });
                let created = entry.metadata.created().ok().map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                });

                LogFileInfo {
                    filename: entry.filename,
                    app_name: entry.app_name,
                    path: Some(entry.path.display().to_string()),
                    size: Some(support::format_bytes(size_bytes)),
                    size_bytes: Some(size_bytes),
                    created,
                    modified,
                }
            } else {
                LogFileInfo {
                    filename:   entry.filename,
                    app_name:   entry.app_name,
                    path:       None,
                    size:       None,
                    size_bytes: None,
                    created:    None,
                    modified:   None,
                }
            }
        })
        .collect();

    Ok(log_infos)
}

/// Individual log file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileInfo {
    /// The filename
    pub filename:   String,
    /// The app name extracted from the filename
    pub app_name:   String,
    /// Full path to the file (included in verbose mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path:       Option<String>,
    /// Human-readable file size (included in verbose mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size:       Option<String>,
    /// File size in bytes (included in verbose mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Creation time as ISO string (included in verbose mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created:    Option<String>,
    /// Modification time as ISO string (included in verbose mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified:   Option<String>,
}
