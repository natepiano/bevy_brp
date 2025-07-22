use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::support;
use crate::error::{Error, report_to_mcp_error};
use crate::tool::{HandlerContext, HandlerResponse, ToolFn};

#[derive(Deserialize, JsonSchema)]
pub struct ReadLogParams {
    /// The log filename (e.g., `bevy_brp_mcp_myapp_1234567890.log`)
    pub filename:   String,
    /// Optional keyword to filter lines (case-insensitive)
    pub keyword:    Option<String>,
    /// Optional number of lines to read from the end of file
    pub tail_lines: Option<u32>,
}

/// Result from reading a log file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadLogResult {
    /// The filename that was read
    pub filename:            String,
    /// Full path to the file
    pub file_path:           String,
    /// Size of the file in bytes
    pub size_bytes:          u64,
    /// Human-readable file size
    pub size_human:          String,
    /// Number of lines read
    pub lines_read:          usize,
    /// The actual log content
    pub content:             String,
    /// Whether content was filtered by keyword
    pub filtered_by_keyword: bool,
    /// Whether tail mode was used
    pub tail_mode:           bool,
}

pub struct ReadLog;

impl ToolFn for ReadLog {
    type Output = ReadLogResult;
    type CallInfoData = crate::response::LocalCallInfo;

    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)> {
        // Extract typed parameters
        let params: ReadLogParams = match ctx.extract_parameter_values() {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            // Convert tail_lines if provided
            let tail_lines = match params.tail_lines {
                Some(lines) => match usize::try_from(lines) {
                    Ok(n) => Some(n),
                    Err(_) => {
                        return Err(
                            Error::invalid("tail_lines", "tail_lines value too large").into()
                        );
                    }
                },
                None => None,
            };

            let result = handle_impl(&params.filename, params.keyword.as_deref(), tail_lines)
                .map_err(|e| Error::tool_call_failed(e.message))?;
            Ok((crate::response::LocalCallInfo, result))
        })
    }
}

fn handle_impl(
    filename: &str,
    keyword: Option<&str>,
    tail_lines: Option<usize>,
) -> Result<ReadLogResult, McpError> {
    // Validate filename format for security
    if !support::is_valid_log_filename(filename) {
        return Err(report_to_mcp_error(&error_stack::Report::new(
            Error::invalid("filename", "only bevy_brp_mcp log files can be read"),
        )));
    }

    // Build full path
    let log_path = support::get_log_file_path(filename);

    // Check if file exists
    if !log_path.exists() {
        return Err(report_to_mcp_error(&error_stack::Report::new(
            Error::missing(&format!("log file '{filename}'")),
        )));
    }

    // Read the log file
    let (content, metadata) = read_log_file(&log_path, keyword, tail_lines)?;

    Ok(ReadLogResult {
        filename: filename.to_string(),
        file_path: log_path.display().to_string(),
        size_bytes: metadata.len(),
        size_human: support::format_bytes(metadata.len()),
        lines_read: content.lines().count(),
        content,
        filtered_by_keyword: keyword.is_some(),
        tail_mode: tail_lines.is_some(),
    })
}

fn read_log_file(
    path: &Path,
    keyword: Option<&str>,
    tail_lines: Option<usize>,
) -> Result<(String, std::fs::Metadata), McpError> {
    // Get file metadata
    let metadata = std::fs::metadata(path).map_err(|e| {
        report_to_mcp_error(&error_stack::Report::new(Error::io_failed(
            "get file metadata",
            path,
            &e,
        )))
    })?;

    // Open the file
    let file = File::open(path).map_err(|e| {
        report_to_mcp_error(&error_stack::Report::new(Error::io_failed(
            "open log file",
            path,
            &e,
        )))
    })?;

    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();

    // Read lines with optional keyword filtering
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| {
            report_to_mcp_error(&error_stack::Report::new(Error::io_failed(
                "read line from log",
                path,
                &e,
            )))
        })?;

        // Apply keyword filter if provided
        let should_include =
            keyword.is_none_or(|kw| line.to_lowercase().contains(&kw.to_lowercase()));

        if should_include {
            lines.push(line);
        }
    }

    // Apply tail mode if requested
    let final_lines = if let Some(tail_count) = tail_lines {
        if tail_count > 0 && tail_count < lines.len() {
            let skip_amount = lines.len() - tail_count;
            lines.into_iter().skip(skip_amount).collect()
        } else {
            lines
        }
    } else {
        lines
    };

    let content = final_lines.join("\n");
    Ok((content, metadata))
}
