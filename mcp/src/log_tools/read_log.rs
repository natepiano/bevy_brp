use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use rmcp::Error as McpError;

use super::support;
use crate::error::{Error, report_to_mcp_error};
use crate::response::LogContentResult;
use crate::support::params;

pub fn handle(request: &rmcp::model::CallToolRequestParam) -> Result<LogContentResult, McpError> {
    // Extract parameters
    let filename = params::extract_required_string(request, "filename")?;
    let keyword = params::extract_optional_string(request, "keyword", "");
    let tail_lines = usize::try_from(params::extract_optional_number(request, "tail_lines", 0)?)
        .map_err(|_| -> McpError {
            report_to_mcp_error(&error_stack::Report::new(Error::invalid(
                "tail_lines",
                "value too large",
            )))
        })?;
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

    Ok(LogContentResult {
        filename: filename.to_string(),
        file_path: log_path.display().to_string(),
        size_bytes: metadata.len(),
        size_human: support::format_bytes(metadata.len()),
        lines_read: content.lines().count(),
        content,
        filtered_by_keyword: !keyword.is_empty(),
        tail_mode: tail_lines > 0,
    })
}

fn read_log_file(
    path: &Path,
    keyword: &str,
    tail_lines: usize,
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
        if keyword.is_empty() || line.to_lowercase().contains(&keyword.to_lowercase()) {
            lines.push(line);
        }
    }

    // Apply tail mode if requested
    let final_lines = if tail_lines > 0 && tail_lines < lines.len() {
        let skip_amount = lines.len() - tail_lines;
        lines.into_iter().skip(skip_amount).collect()
    } else {
        lines
    };

    let content = final_lines.join("\n");
    Ok((content, metadata))
}
