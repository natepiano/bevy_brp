use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::json;

use super::support::LogFileEntry;
use crate::log_tools::support;
use crate::support::large_response::{LargeResponseConfig, handle_large_response};
use crate::support::params;
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;

pub fn handle(request: &rmcp::model::CallToolRequestParam) -> Result<CallToolResult, McpError> {
    // Extract optional app name filter
    let app_name_filter = params::extract_optional_string(request, "app_name", "");

    // Extract verbose flag (default to false)
    let verbose = params::extract_optional_bool(request, "verbose", false);

    let logs = list_log_files(app_name_filter, verbose)?;

    // Build the response data
    let response_data = json!({
        "logs": logs,
        "temp_directory": support::get_log_directory().display().to_string(),
    });

    // Check if response is too large and handle accordingly
    let final_data = handle_large_response(
        &response_data,
        "list_logs",
        LargeResponseConfig {
            file_prefix: "log_list_",
            ..Default::default()
        },
    )
    .map_err(|e| McpError::internal_error(format!("Failed to handle large response: {e}"), None))?;

    // If we got a fallback response (file was created), use that
    // Otherwise use the original response data
    let response = final_data.map_or_else(
        || {
            // Original response - small enough to return inline
            ResponseBuilder::success()
                .message(format!("Found {} log files", logs.len()))
                .data(response_data)
                .map_or_else(
                    |_| {
                        ResponseBuilder::error()
                            .message("Failed to serialize response data")
                            .build()
                    },
                    ResponseBuilder::build,
                )
        },
        |fallback_data| {
            // Extract the message and data from the fallback response
            let message = fallback_data
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Response saved to file");

            ResponseBuilder::success()
                .message(message)
                .data(fallback_data)
                .map_or_else(
                    |_| {
                        ResponseBuilder::error()
                            .message("Failed to serialize response data")
                            .build()
                    },
                    ResponseBuilder::build,
                )
        },
    );

    Ok(json_response_to_result(&response))
}

fn list_log_files(
    app_name_filter: &str,
    verbose: bool,
) -> Result<Vec<serde_json::Value>, McpError> {
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

    // Convert to JSON values
    let json_entries: Vec<serde_json::Value> = log_entries
        .into_iter()
        .map(|entry| entry.to_json(verbose))
        .collect();

    Ok(json_entries)
}
