use std::fs;
use std::time::Duration;
use std::time::SystemTime;

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::support::LogFileEntry;
use super::support::{self};
use crate::error::Error;
use crate::error::Result;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct DeleteLogsParams {
    /// Optional filter to delete logs for a specific app only
    #[to_metadata(skip_if_none)]
    pub app_name:           Option<String>,
    /// Optional filter to delete logs older than N seconds
    #[to_metadata(skip_if_none)]
    pub older_than_seconds: Option<u32>,
}

/// Result from cleaning up log files
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct DeleteLogsResult {
    /// List of deleted filenames
    #[to_metadata]
    deleted_files:      Vec<String>,
    /// Number of files deleted
    #[to_metadata]
    deleted_count:      usize,
    /// App name filter that was applied (if any)
    #[to_metadata(skip_if_none)]
    app_name_filter:    Option<String>,
    /// Age filter in seconds that was applied (if any)
    #[to_metadata(skip_if_none)]
    older_than_seconds: Option<u32>,
    /// Message template for formatting responses
    #[to_message(message_template = "Deleted {deleted_count} log files")]
    message_template:   String,
}

#[derive(ToolFn)]
#[tool_fn(params = "DeleteLogsParams", output = "DeleteLogsResult")]
pub struct DeleteLogs;

#[allow(clippy::unused_async)]
async fn handle_impl(params: DeleteLogsParams) -> Result<DeleteLogsResult> {
    let deleted_files = delete_log_files(params.app_name.as_deref(), params.older_than_seconds)?;

    Ok(DeleteLogsResult::new(
        deleted_files.clone(),
        deleted_files.len(),
        params.app_name.clone(),
        params.older_than_seconds,
    ))
}

fn delete_log_files(
    app_name_filter: Option<&str>,
    older_than_seconds: Option<u32>,
) -> Result<Vec<String>> {
    let mut deleted_files = Vec::new();

    // Calculate cutoff time if age filter is specified
    let cutoff_time = older_than_seconds
        .map(|seconds| SystemTime::now() - Duration::from_secs(u64::from(seconds)));

    // Use the iterator to get all log files with filters
    let filter = |entry: &LogFileEntry| -> bool {
        // Apply app name filter
        if let Some(app_filter) = app_name_filter
            && entry.app_name != app_filter
        {
            return false;
        }

        // Apply age filter if provided
        if let Some(cutoff) = cutoff_time
            && let Ok(modified) = entry.metadata.modified()
        {
            // Skip if file is newer than cutoff
            if modified > cutoff {
                return false;
            }
        }

        true
    };

    let log_entries = if app_name_filter.is_some() {
        // When filtering by app name, only consider app logs (with port pattern)
        support::iterate_app_log_files(filter)
            .map_err(|e| Error::tool_call_failed(e.to_string()))?
    } else {
        // When no app name filter, consider all log types
        support::iterate_log_files(filter).map_err(|e| Error::tool_call_failed(e.to_string()))?
    };

    // Delete the files
    for entry in log_entries {
        if fs::remove_file(&entry.path).is_ok() {
            deleted_files.push(entry.filename);
        }
    }

    Ok(deleted_files)
}
