use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use bevy_kana::ToF64;
use error_stack::ResultExt;
use regex::Regex;

use super::constants::BYTES_PER_UNIT;
use super::constants::LOG_EXTENSION;
use super::constants::LOG_PREFIX;
use crate::error::Error;
use crate::error::Result;

// Static regex for parsing app log filenames
static APP_LOG_REGEX: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^bevy_brp_mcp_(.+?)_port\d+_(\d+)_\d+\.log$").ok());

/// Validates if a filename follows the `bevy_brp_mcp` log naming convention
pub(super) fn is_valid_log_filename(filename: &str) -> bool {
    filename.starts_with(LOG_PREFIX) && filename.ends_with(LOG_EXTENSION)
}

/// Parses app log filename with port pattern into app name and timestamp
/// Returns `Some((app_name, timestamp_str))` if matches app log pattern, `None` otherwise
///
/// Format: `bevy_brp_mcp`_{`app_name`}_port{number}_{timestamp}_{suffix}.log
/// Extracts `app_name` as the part between "`bevy_brp_mcp`_" and "_port{number}"
pub(super) fn parse_app_log_filename(filename: &str) -> Option<(String, String)> {
    if !is_valid_log_filename(filename) {
        return None;
    }

    // Use the static regex, returning None if regex compilation failed
    let regex = APP_LOG_REGEX.as_ref()?;

    if let Some(captures) = regex.captures(filename) {
        let app_name = captures.get(1)?.as_str().to_string();
        let timestamp = captures.get(2)?.as_str().to_string();
        return Some((app_name, timestamp));
    }

    None
}

/// Parses any log filename into app name and timestamp components
/// Returns `Some((app_name, timestamp_str))` if valid, `None` otherwise
///
/// Tries app log pattern first, falls back to generic pattern for other log types
pub(super) fn parse_log_filename(filename: &str) -> Option<(String, String)> {
    // Try app log pattern first
    if let Some(result) = parse_app_log_filename(filename) {
        return Some(result);
    }

    // Fallback for other log types (watch logs, etc.)
    if !is_valid_log_filename(filename) {
        return None;
    }

    let parts: Vec<&str> = filename
        .trim_start_matches(LOG_PREFIX)
        .trim_end_matches(LOG_EXTENSION)
        .rsplitn(2, '_')
        .collect();

    if parts.len() != 2 {
        return None;
    }

    // Parts are reversed due to rsplitn
    let timestamp_str = parts[0].to_string();
    let app_name = parts[1].to_string();

    Some((app_name, timestamp_str))
}

/// Formats bytes into human-readable string with appropriate unit
pub(super) fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes.to_f64();
    let mut unit_index = 0;

    while size >= BYTES_PER_UNIT && unit_index < UNITS.len() - 1 {
        size /= BYTES_PER_UNIT;
        unit_index += 1;
    }

    let unit = UNITS[unit_index];
    if unit_index == 0 {
        format!("{bytes} {unit}")
    } else {
        format!("{size:.2} {unit}")
    }
}

/// Gets the log directory (system temp directory)
pub(super) fn get_log_directory() -> PathBuf { std::env::temp_dir() }

/// Gets the full path for a log file given its filename
pub(super) fn get_log_file_path(filename: &str) -> PathBuf { get_log_directory().join(filename) }

/// Represents a log file entry with metadata
#[derive(Debug, Clone)]
pub(super) struct LogFileEntry {
    pub(super) filename:  String,
    pub(super) app_name:  String,
    pub(super) timestamp: String,
    pub(super) path:      PathBuf,
    pub(super) metadata:  fs::Metadata,
}

/// Iterates over app log files (port pattern only) in the temp directory with optional filtering
/// The filter function receives a `LogFileEntry` and returns true to include it
pub(super) fn iterate_app_log_files<F>(filter: F) -> Result<Vec<LogFileEntry>>
where
    F: Fn(&LogFileEntry) -> bool,
{
    let temp_dir = get_log_directory();
    let mut log_entries = Vec::new();

    // Read the temp directory
    let entries = fs::read_dir(&temp_dir)
        .change_context(Error::FileOperation(
            "Failed to read temp directory".to_string(),
        ))
        .attach(format!("Path: {}", temp_dir.display()))?;

    // Process each entry
    for entry in entries {
        let entry = entry
            .change_context(Error::FileOperation(
                "Failed to read directory entry".to_string(),
            ))
            .attach(format!("Directory: {}", temp_dir.display()))?;

        let path = entry.path();
        let filename = path.file_name().and_then(OsStr::to_str).unwrap_or("");

        // Parse only app log filenames (with port pattern)
        if let Some((app_name, timestamp)) = parse_app_log_filename(filename) {
            // Get file metadata
            let metadata = entry
                .metadata()
                .change_context(Error::FileOperation(
                    "Failed to get file metadata".to_string(),
                ))
                .attach(format!("Path: {}", path.display()))?;

            let log_entry = LogFileEntry {
                filename: filename.to_string(),
                app_name,
                timestamp,
                path,
                metadata,
            };

            // Apply filter
            if filter(&log_entry) {
                log_entries.push(log_entry);
            }
        }
    }

    Ok(log_entries)
}

/// Iterates over all log files in the temp directory with optional filtering
/// The filter function receives a `LogFileEntry` and returns true to include it
pub(super) fn iterate_log_files<F>(filter: F) -> Result<Vec<LogFileEntry>>
where
    F: Fn(&LogFileEntry) -> bool,
{
    let temp_dir = get_log_directory();
    let mut log_entries = Vec::new();

    // Read the temp directory
    let entries = fs::read_dir(&temp_dir)
        .change_context(Error::FileOperation(
            "Failed to read temp directory".to_string(),
        ))
        .attach(format!("Path: {}", temp_dir.display()))?;

    // Process each entry
    for entry in entries {
        let entry = entry
            .change_context(Error::FileOperation(
                "Failed to read directory entry".to_string(),
            ))
            .attach(format!("Directory: {}", temp_dir.display()))?;

        let path = entry.path();
        let filename = path.file_name().and_then(OsStr::to_str).unwrap_or("");

        // Parse the filename
        if let Some((app_name, timestamp)) = parse_log_filename(filename) {
            // Get file metadata
            let metadata = entry
                .metadata()
                .change_context(Error::FileOperation(
                    "Failed to get file metadata".to_string(),
                ))
                .attach(format!("Path: {}", path.display()))?;

            let log_entry = LogFileEntry {
                filename: filename.to_string(),
                app_name,
                timestamp,
                path,
                metadata,
            };

            // Apply filter
            if filter(&log_entry) {
                log_entries.push(log_entry);
            }
        }
    }

    Ok(log_entries)
}
