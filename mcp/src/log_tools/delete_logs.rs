use std::fs;
use std::time::{Duration, SystemTime};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::support::{self, LogFileEntry};
use crate::error::{Error, Result};
use crate::response::LocalCallInfo;
use crate::tool::{HandlerContext, HandlerResponse, ToolFn, WithCallInfo};

#[derive(Deserialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DeleteLogsParams {
    /// Optional filter to delete logs for a specific app only
    #[to_metadata(skip_if_none)]
    pub app_name:           Option<String>,
    /// Optional filter to delete logs older than N seconds
    #[to_metadata(skip_if_none)]
    pub older_than_seconds: Option<u32>,
}

/// Result from cleaning up log files
#[derive(Debug, Clone, Serialize, Deserialize, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DeleteLogsResult {
    /// List of deleted filenames
    #[to_metadata]
    pub deleted_files:      Vec<String>,
    /// Number of files deleted
    #[to_metadata]
    pub deleted_count:      usize,
    /// App name filter that was applied (if any)
    #[to_metadata(skip_if_none)]
    pub app_name_filter:    Option<String>,
    /// Age filter in seconds that was applied (if any)
    #[to_metadata(skip_if_none)]
    pub older_than_seconds: Option<u32>,
}

pub struct DeleteLogs;

impl ToolFn for DeleteLogs {
    type Output = DeleteLogsResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        ctx: &HandlerContext,
    ) -> HandlerResponse<(Self::CallInfoData, crate::error::Result<Self::Output>)> {
        // Extract typed parameters
        let params: DeleteLogsParams = match ctx.extract_parameter_values() {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Ok(Err(e).with_call_info(LocalCallInfo)) }),
        };

        Box::pin(async move {
            Ok(
                handle_impl(params.app_name.as_deref(), params.older_than_seconds)
                    .with_call_info(LocalCallInfo),
            )
        })
    }
}

fn handle_impl(
    app_name_filter: Option<&str>,
    older_than_seconds: Option<u32>,
) -> Result<DeleteLogsResult> {
    let deleted_files = delete_log_files(app_name_filter, older_than_seconds)?;

    Ok(DeleteLogsResult {
        deleted_count: deleted_files.len(),
        deleted_files,
        app_name_filter: app_name_filter.map(String::from),
        older_than_seconds,
    })
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
        if let Some(app_filter) = app_name_filter {
            if entry.app_name != app_filter {
                return false;
            }
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
