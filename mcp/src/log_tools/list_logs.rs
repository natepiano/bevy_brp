use rmcp::Error as McpError;

use super::support::LogFileEntry;
use crate::extractors::McpCallExtractor;
use crate::log_tools::support;
use crate::response::{LogFileInfo, LogListResult};

pub fn handle(request: &rmcp::model::CallToolRequestParam) -> Result<LogListResult, McpError> {
    // Extract optional app name filter
    let extractor = McpCallExtractor::from_request(request);
    let app_name_filter = extractor.get_optional_string("app_name", "");

    // Extract verbose flag (default to false)
    let verbose = extractor.get_optional_bool("verbose", false);

    let logs = list_log_files(app_name_filter, verbose)?;
    let count = logs.len();

    Ok(LogListResult {
        logs,
        temp_directory: support::get_log_directory().display().to_string(),
        count,
    })
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
