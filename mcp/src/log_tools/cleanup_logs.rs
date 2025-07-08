use std::fs;
use std::time::{Duration, SystemTime};

use rmcp::Error as McpError;
use rmcp::model::{CallToolRequestParam, CallToolResult};
use serde_json::json;

use super::support::{self, LogFileEntry};
use crate::support::params;
use crate::support::response::ResponseBuilder;

pub fn handle(request: &CallToolRequestParam) -> Result<CallToolResult, McpError> {
    // Extract parameters
    let app_name_filter = params::extract_optional_string(request, "app_name", "");
    let older_than_seconds = params::extract_optional_u32(request, "older_than_seconds", 0)?;

    let (deleted_count, deleted_files) = cleanup_log_files(app_name_filter, older_than_seconds)?;

    let response = ResponseBuilder::success()
        .message(format!("Deleted {deleted_count} log files"))
        .data(json!({
            "deleted_count": deleted_count,
            "deleted_files": deleted_files,
            "app_name_filter": if app_name_filter.is_empty() { json!(null) } else { json!(app_name_filter) },
            "older_than_seconds": if older_than_seconds == 0 { json!(null) } else { json!(older_than_seconds) },
        }))
        .map_or_else(
            |_| ResponseBuilder::error()
                .message("Failed to serialize response data")
                .build(),
            ResponseBuilder::build,
        );

    Ok(response.to_call_tool_result())
}

fn cleanup_log_files(
    app_name_filter: &str,
    older_than_seconds: u32,
) -> Result<(usize, Vec<String>), McpError> {
    let mut deleted_files = Vec::new();

    // Calculate cutoff time if age filter is specified
    let cutoff_time = if older_than_seconds > 0 {
        Some(SystemTime::now() - Duration::from_secs(u64::from(older_than_seconds)))
    } else {
        None
    };

    // Use the iterator to get all log files with filters
    let filter = |entry: &LogFileEntry| -> bool {
        // Apply app name filter
        if !app_name_filter.is_empty() && entry.app_name != app_name_filter {
            return false;
        }

        // Apply age filter if provided
        if let Some(cutoff) = cutoff_time {
            if let Ok(modified) = entry.metadata.modified() {
                // Skip if file is newer than cutoff
                if modified > cutoff {
                    return false;
                }
            }
        }

        true
    };

    let log_entries = support::iterate_log_files(filter)?;

    // Delete the files
    for entry in log_entries {
        if fs::remove_file(&entry.path).is_ok() {
            deleted_files.push(entry.filename);
        }
    }

    let deleted_count = deleted_files.len();
    Ok((deleted_count, deleted_files))
}
