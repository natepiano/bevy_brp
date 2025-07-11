// Local support modules for app_tools

pub mod cargo_detector;
pub mod collection_strategy;
pub mod launch_common;
pub mod list_common;
pub mod logging;
pub mod process;
pub mod scanning;

/// Extracts duplicate paths from an error message.
///
/// This function parses error messages that contain multiple paths, typically
/// formatted as "Found multiple X named 'Y' at:\n- path1\n- path2"
///
/// # Arguments
/// * `error_msg` - The error message to parse
///
/// # Returns
/// * `Option<Vec<String>>` - The extracted paths, or None if no paths found
pub fn extract_duplicate_paths(error_msg: &str) -> Option<Vec<String>> {
    if error_msg.contains("Found multiple") {
        let lines: Vec<&str> = error_msg.lines().collect();
        let mut paths = Vec::new();
        for line in &lines[1..] {
            if let Some(path) = line.strip_prefix("- ") {
                paths.push(path.to_string());
            }
        }
        if paths.is_empty() { None } else { Some(paths) }
    } else {
        None
    }
}
