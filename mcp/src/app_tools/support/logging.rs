use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use error_stack::ResultExt;

/// Create a log file for a Bevy app launch
use super::cargo_detector::TargetType;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;

pub fn create_log_file(
    name: &str,
    target_type: TargetType,
    profile: &str,
    binary_path: &Path,
    manifest_dir: &Path,
    port: Port,
) -> Result<(PathBuf, File)> {
    // Generate unique log file name in temp directory
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .change_context(Error::LogOperation("Failed to get timestamp".to_string()))
        .attach("System time error")?
        .as_millis();

    // Port provides uniqueness for multiple instances
    let log_file_path =
        std::env::temp_dir().join(format!("bevy_brp_mcp_{name}_port{port}_{timestamp}.log"));

    // Create log file
    let mut log_file = File::create(&log_file_path)
        .change_context(Error::LogOperation("Failed to create log file".to_string()))
        .attach(format!("Path: {}", log_file_path.display()))?;

    // Write header
    writeln!(log_file, "=== Bevy BRP MCP Launch Log ===").change_context(Error::LogOperation(
        "Failed to write to log file".to_string(),
    ))?;
    writeln!(log_file, "Started at: {:?}", std::time::SystemTime::now()).change_context(
        Error::LogOperation("Failed to write to log file".to_string()),
    )?;
    writeln!(log_file, "{target_type}: {name}").change_context(Error::LogOperation(
        "Failed to write to log file".to_string(),
    ))?;
    writeln!(log_file, "Profile: {profile}").change_context(Error::LogOperation(
        "Failed to write to log file".to_string(),
    ))?;
    writeln!(log_file, "Binary: {}", binary_path.display()).change_context(Error::LogOperation(
        "Failed to write to log file".to_string(),
    ))?;
    writeln!(log_file, "Working directory: {}", manifest_dir.display()).change_context(
        Error::LogOperation("Failed to write to log file".to_string()),
    )?;
    writeln!(log_file, "============================================\n").change_context(
        Error::LogOperation("Failed to write to log file".to_string()),
    )?;
    log_file
        .sync_all()
        .change_context(Error::LogOperation("Failed to sync log file".to_string()))?;

    Ok((log_file_path, log_file))
}

/// Open an existing log file for appending (for stdout/stderr redirection)
pub fn open_log_file_for_redirect(log_file_path: &Path) -> Result<File> {
    File::options()
        .append(true)
        .open(log_file_path)
        .change_context(Error::LogOperation(
            "Failed to open log file for redirect".to_string(),
        ))
        .attach(format!("Path: {}", log_file_path.display()))
}

/// Appends additional text to an existing log file
pub fn append_to_log_file(log_file_path: &Path, content: &str) -> Result<()> {
    let mut file = File::options()
        .append(true)
        .open(log_file_path)
        .change_context(Error::LogOperation(
            "Failed to open log file for appending".to_string(),
        ))
        .attach(format!("Path: {}", log_file_path.display()))?;

    write!(file, "{content}").change_context(Error::LogOperation(
        "Failed to write to log file".to_string(),
    ))?;

    file.sync_all()
        .change_context(Error::LogOperation("Failed to sync log file".to_string()))?;

    Ok(())
}
