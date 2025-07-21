use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::support::LogFileEntry;
use crate::error::Error;
use crate::log_tools::support;
use crate::tool::{HandlerContext, HandlerResponse, LocalToolFn, NoMethod, NoPort, ParameterName};

#[derive(Deserialize, JsonSchema)]
pub struct ListLogsParams {
    /// Optional filter to list logs for a specific app only
    pub app_name: Option<String>,
    /// Include full details (path, timestamps, size in bytes). Default is false for minimal output
    pub verbose:  Option<bool>,
}

/// Result from listing log files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLogResult {
    /// List of log files found
    pub logs:           Vec<LogFileInfo>,
    /// Path to the temp directory containing logs
    pub temp_directory: String,
}

/// Handler for the `brp_list_logs` tool using the `LocalFn` approach
pub struct ListLogs;

impl LocalToolFn for ListLogs {
    type Output = ListLogResult;

    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<Self::Output> {
        // Extract optional app name filter
        let app_name_filter = ctx
            .extract_with_default(ParameterName::AppName, "")
            .into_string()
            .unwrap_or_default();

        // Extract verbose flag (default to false)
        let verbose = ctx
            .extract_with_default(ParameterName::Verbose, false)
            .into_bool()
            .unwrap_or(false);

        Box::pin(async move {
            match list_log_files(&app_name_filter, verbose) {
                Ok(logs) => Ok(ListLogResult {
                    logs,
                    temp_directory: support::get_log_directory().display().to_string(),
                }),
                Err(e) => Err(Error::tool_call_failed(e.message).into()),
            }
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
