use std::fs;
use std::time::{Duration, SystemTime};

use rmcp::model::{CallToolRequestParam, CallToolResult};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::log_utils;
use crate::BrpMcpService;
use crate::support::{params, response};

pub fn handle(
    _service: &BrpMcpService,
    request: &CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    // Extract parameters
    let app_name_filter = params::extract_optional_string(request, "app_name", "");
    let older_than_seconds = params::extract_optional_u32(request, "older_than_seconds", 0)?;

    let (deleted_count, deleted_files) = cleanup_log_files(app_name_filter, older_than_seconds)?;

    Ok(response::success_json_response(
        format!("Deleted {deleted_count} log files"),
        json!({
            "deleted_count": deleted_count,
            "deleted_files": deleted_files,
            "app_name_filter": if app_name_filter.is_empty() { json!(null) } else { json!(app_name_filter) },
            "older_than_seconds": if older_than_seconds == 0 { json!(null) } else { json!(older_than_seconds) },
        }),
    ))
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
    let filter = |entry: &log_utils::LogFileEntry| -> bool {
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

    let log_entries = log_utils::iterate_log_files(filter)?;

    // Delete the files
    for entry in log_entries {
        match fs::remove_file(&entry.path) {
            Ok(()) => {
                deleted_files.push(entry.filename);
            }
            Err(e) => {
                eprintln!("Warning: Failed to delete {}: {}", entry.filename, e);
            }
        }
    }

    let deleted_count = deleted_files.len();
    Ok((deleted_count, deleted_files))
}
