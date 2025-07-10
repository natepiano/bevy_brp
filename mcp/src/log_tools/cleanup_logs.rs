use std::fs;
use std::time::{Duration, SystemTime};

use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use super::support::{self, LogFileEntry};
use crate::extractors::McpCallExtractor;
use crate::handler::{HandlerContext, HandlerResponse, HandlerResult, LocalHandler};

/// Result from cleaning up log files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupLogsResult {
    /// Number of files deleted
    pub deleted_count:      usize,
    /// List of deleted filenames
    pub deleted_files:      Vec<String>,
    /// App name filter that was applied (if any)
    pub app_name_filter:    Option<String>,
    /// Age filter in seconds that was applied (if any)
    pub older_than_seconds: Option<u32>,
}

impl HandlerResult for CleanupLogsResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct CleanupLogs;

impl LocalHandler for CleanupLogs {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        // Extract parameters before the async block
        let extractor = McpCallExtractor::from_request(&ctx.request);
        let app_name_filter = extractor.get_optional_string("app_name", "");
        let older_than_seconds = match extractor.get_optional_u32("older_than_seconds", 0) {
            Ok(n) => n,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            handle_impl(&app_name_filter, older_than_seconds)
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

fn handle_impl(
    app_name_filter: &str,
    older_than_seconds: u32,
) -> Result<CleanupLogsResult, McpError> {
    let (deleted_count, deleted_files) = cleanup_log_files(app_name_filter, older_than_seconds)?;

    Ok(CleanupLogsResult {
        deleted_count,
        deleted_files,
        app_name_filter: if app_name_filter.is_empty() {
            None
        } else {
            Some(app_name_filter.to_string())
        },
        older_than_seconds: if older_than_seconds == 0 {
            None
        } else {
            Some(older_than_seconds)
        },
    })
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
