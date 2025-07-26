use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::support;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, LocalCallInfo, ToolFn, ToolResult};

#[derive(Deserialize, JsonSchema, ParamStruct)]
pub struct ReadLogParams {
    /// The log filename (e.g., `bevy_brp_mcp_myapp_1234567890.log`)
    #[to_metadata]
    pub filename:   String,
    /// Optional keyword to filter lines (case-insensitive)
    #[to_metadata(skip_if_none)]
    pub keyword:    Option<String>,
    /// Optional number of lines to read from the end of file
    #[to_metadata(skip_if_none)]
    pub tail_lines: Option<u32>,
}

/// Result from reading a log file
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
#[allow(clippy::too_many_arguments)]
pub struct ReadLogResult {
    /// The filename that was read
    #[to_metadata]
    filename:            String,
    /// Full path to the file
    #[to_metadata]
    file_path:           String,
    /// Size of the file in bytes
    #[to_metadata]
    size_bytes:          u64,
    /// Human-readable file size
    #[to_metadata]
    size_human:          String,
    /// Number of lines read
    #[to_metadata]
    lines_read:          usize,
    /// The actual log content
    #[to_result]
    content:             String,
    /// Whether content was filtered by keyword
    #[to_metadata]
    filtered_by_keyword: bool,
    /// Whether tail mode was used
    #[to_metadata]
    tail_mode:           bool,
    /// Message template for formatting responses
    #[to_message(message_template = "Read {lines_read} lines from {filename}")]
    message_template:    String,
}

pub struct ReadLog;

impl ToolFn for ReadLog {
    type Output = ReadLogResult;
    type CallInfoData = LocalCallInfo;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>> {
        Box::pin(async move {
            let params: ReadLogParams = ctx.extract_parameter_values()?;

            // Convert tail_lines if provided
            let tail_lines = match params.tail_lines {
                Some(lines) => match usize::try_from(lines) {
                    Ok(n) => Some(n),
                    Err(_) => {
                        return Ok(ToolResult::from_result(
                            Err(Error::invalid("tail_lines", "tail_lines value too large").into()),
                            LocalCallInfo,
                        ));
                    }
                },
                None => None,
            };

            Ok(ToolResult::from_result(
                handle_impl(&params.filename, params.keyword.as_deref(), tail_lines),
                LocalCallInfo,
            ))
        })
    }
}

fn handle_impl(
    filename: &str,
    keyword: Option<&str>,
    tail_lines: Option<usize>,
) -> Result<ReadLogResult> {
    // Validate filename format for security
    if !support::is_valid_log_filename(filename) {
        return Err(Error::invalid("filename", "only bevy_brp_mcp log files can be read").into());
    }

    // Build full path
    let log_path = support::get_log_file_path(filename);

    // Check if file exists
    if !log_path.exists() {
        return Err(Error::missing(&format!("log file '{filename}'")).into());
    }

    // Read the log file
    let (content, metadata) = read_log_file(&log_path, keyword, tail_lines)?;

    Ok(ReadLogResult::new(
        filename.to_string(),
        log_path.display().to_string(),
        metadata.len(),
        support::format_bytes(metadata.len()),
        content.lines().count(),
        content,
        keyword.is_some(),
        tail_lines.is_some(),
    ))
}

fn read_log_file(
    path: &Path,
    keyword: Option<&str>,
    tail_lines: Option<usize>,
) -> Result<(String, std::fs::Metadata)> {
    // Get file metadata
    let metadata =
        std::fs::metadata(path).map_err(|e| Error::io_failed("get file metadata", path, &e))?;

    // Open the file
    let file = File::open(path).map_err(|e| Error::io_failed("open log file", path, &e))?;

    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();

    // Read lines with optional keyword filtering
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| Error::io_failed("read line from log", path, &e))?;

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
