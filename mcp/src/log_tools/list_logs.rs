use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::support::LogFileEntry;
use crate::constants::PARAM_APP_NAME;
use crate::log_tools::support;
use crate::service::{HandlerContext, NoMethod, NoPort};
use crate::tool::{HandlerResponse, HandlerResult, LocalToolFn};

/// Result from listing log files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLogResult {
    /// List of log files found
    pub logs:           Vec<LogFileInfo>,
    /// Path to the temp directory containing logs
    pub temp_directory: String,
    /// Total count of log files
    pub count:          usize,
}

impl HandlerResult for ListLogResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Handler for the `brp_list_logs` tool using the `LocalFn` approach
pub struct ListLogs;

impl LocalToolFn for ListLogs {
    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<'_> {
        // Extract optional app name filter
        let app_name_filter = ctx.extract_optional_string(PARAM_APP_NAME, "");

        // Extract verbose flag (default to false)
        let verbose = ctx.extract_optional_bool("verbose", false);

        Box::pin(async move {
            let logs = list_log_files(&app_name_filter, verbose)?;
            let count = logs.len();

            let result = ListLogResult {
                logs,
                temp_directory: support::get_log_directory().display().to_string(),
                count,
            };

            Ok(Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

fn list_log_files(app_name_filter: &str, verbose: bool) -> Result<Vec<LogFileInfo>, McpError> {
    // Use the iterator to get all log files with optional filter
    let filter = |entry: &LogFileEntry| -> bool {
        app_name_filter.is_empty() || entry.app_name == app_name_filter
    };

    let mut log_entries = support::iterate_log_files(filter)?;

    // Sort by timestamp (newest first)
    log_entries.sort_by(|a, b| {
        let ts_a = a.timestamp.parse::<u128>().unwrap_or(0);
        let ts_b = b.timestamp.parse::<u128>().unwrap_or(0);
        ts_b.cmp(&ts_a)
    });

    // Convert to LogFileInfo structs
    let log_infos: Vec<LogFileInfo> = log_entries
        .into_iter()
        .map(|entry| {
            if verbose {
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
