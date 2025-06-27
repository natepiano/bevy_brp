use std::path::{Path, PathBuf};

use rmcp::Error as McpError;

use super::cargo_detector::{BinaryInfo, CargoDetector, ExampleInfo};
use crate::error::{Error, report_to_mcp_error};

/// Iterator over all valid Cargo project paths found in the given search paths
/// Yields paths to directories containing Cargo.toml files
pub fn iter_cargo_project_paths(search_paths: &[PathBuf]) -> impl Iterator<Item = PathBuf> + '_ {
    search_paths.iter().flat_map(|root| {
        let mut paths = Vec::new();

        // Check the root itself
        if root.join("Cargo.toml").exists() {
            paths.push(root.clone());
        }

        // Check immediate subdirectories
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("Cargo.toml").exists() {
                    // Skip hidden directories and target
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with('.') || name_str == "target" {
                            continue;
                        }
                    }
                    paths.push(path);
                }
            }
        }

        paths.into_iter()
    })
}

/// Extract workspace name from workspace root path
/// Returns the last component of the path as the workspace name
pub fn extract_workspace_name(workspace_root: &Path) -> Option<String> {
    workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(std::string::ToString::to_string)
}

/// Get workspace root from manifest path for examples
/// Walks up the directory structure to find the workspace root
pub fn get_workspace_root_from_manifest(manifest_path: &Path) -> Option<PathBuf> {
    let mut path = manifest_path.parent()?;

    // Walk up the directory tree looking for a Cargo.toml with [workspace]
    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this Cargo.toml defines a workspace
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(path.to_path_buf());
                }
            }
        }

        // Move up one directory
        match path.parent() {
            Some(parent) => path = parent,
            None => break,
        }
    }

    // If no workspace found, use the manifest's parent directory
    manifest_path.parent().map(std::path::Path::to_path_buf)
}

/// Find all apps by name across search paths, returning Vec instead of Option
/// This allows detection of duplicates across workspaces
pub fn find_all_apps_by_name(app_name: &str, search_paths: &[PathBuf]) -> Vec<BinaryInfo> {
    let mut apps = Vec::new();

    for path in iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let found_apps = detector.find_bevy_apps();
            for app in found_apps {
                if app.name == app_name {
                    apps.push(app);
                }
            }
        }
    }

    apps
}

/// Find all examples by name across search paths, returning Vec instead of Option
/// This allows detection of duplicates across workspaces
pub fn find_all_examples_by_name(example_name: &str, search_paths: &[PathBuf]) -> Vec<ExampleInfo> {
    let mut examples = Vec::new();

    for path in iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let found_examples = detector.find_bevy_examples();
            for example in found_examples {
                if example.name == example_name {
                    examples.push(example);
                }
            }
        }
    }

    examples
}

/// Find a required app by name with workspace parameter handling
/// Returns an error with workspace options if duplicates found and no workspace specified
pub fn find_required_app_with_workspace(
    app_name: &str,
    workspace: Option<&str>,
    search_paths: &[PathBuf],
) -> Result<BinaryInfo, McpError> {
    let all_apps = find_all_apps_by_name(app_name, search_paths);

    let filtered_apps =
        find_and_filter_by_workspace(all_apps, workspace, |app| Some(app.workspace_root.clone()));

    validate_single_result_or_error(filtered_apps, app_name, "app", "app_name", |app| {
        Some(app.workspace_root.clone())
    })
}

/// Find a required example by name with workspace parameter handling
/// Returns an error with workspace options if duplicates found and no workspace specified
pub fn find_required_example_with_workspace(
    example_name: &str,
    workspace: Option<&str>,
    search_paths: &[PathBuf],
) -> Result<ExampleInfo, McpError> {
    let all_examples = find_all_examples_by_name(example_name, search_paths);

    let filtered_examples = find_and_filter_by_workspace(all_examples, workspace, |example| {
        get_workspace_root_from_manifest(&example.manifest_path)
    });

    validate_single_result_or_error(
        filtered_examples,
        example_name,
        "example",
        "example_name",
        |example| get_workspace_root_from_manifest(&example.manifest_path),
    )
}

/// Build error message for duplicate items across workspaces
fn build_workspace_selection_error(
    item_type: &str,
    item_name: &str,
    param_name: &str,
    workspaces: &[String],
) -> String {
    let workspace_list = workspaces
        .iter()
        .map(|w| format!("- Workspace: {w}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Found multiple {item_type} named '{item_name}' in different workspaces:\n\n{workspace_list}\n\nPlease specify which workspace to use:\n{{\"{param_name}\": \"{item_name}\", \"workspace\": \"workspace_name\"}}"
    )
}

/// Find items by name and filter by workspace if provided
fn find_and_filter_by_workspace<T>(
    all_items: Vec<T>,
    workspace: Option<&str>,
    get_workspace_root: impl Fn(&T) -> Option<PathBuf>,
) -> Vec<T> {
    if let Some(workspace_name) = workspace {
        all_items
            .into_iter()
            .filter(|item| {
                if let Some(root) = get_workspace_root(item) {
                    if let Some(item_workspace) = extract_workspace_name(&root) {
                        return item_workspace == workspace_name;
                    }
                }
                false
            })
            .collect()
    } else {
        all_items
    }
}

/// Validate that exactly one item was found, or return helpful error
fn validate_single_result_or_error<T>(
    items: Vec<T>,
    item_name: &str,
    item_type: &str,
    param_name: &str,
    get_workspace_root: impl Fn(&T) -> Option<PathBuf>,
) -> Result<T, McpError> {
    match items.len() {
        0 => Err(report_to_mcp_error(
            &error_stack::Report::new(Error::Configuration(format!(
                "Bevy {item_type} '{item_name}' not found in search paths"
            )))
            .attach_printable(format!("Item type: {item_type}"))
            .attach_printable(format!("Item name: {item_name}")),
        )),
        1 => {
            // We know exactly one item exists
            let mut iter = items.into_iter();
            iter.next().map_or_else(
                || {
                    Err(report_to_mcp_error(
                        &error_stack::Report::new(Error::Configuration(format!(
                            "Bevy {item_type} '{item_name}' not found in search paths"
                        )))
                        .attach_printable(format!("Item type: {item_type}"))
                        .attach_printable(format!("Item name: {item_name}")),
                    ))
                },
                |item| Ok(item),
            )
        }
        _ => {
            let workspaces: Vec<String> = items
                .iter()
                .filter_map(|item| {
                    get_workspace_root(item).and_then(|root| extract_workspace_name(&root))
                })
                .collect();

            let error_msg =
                build_workspace_selection_error(item_type, item_name, param_name, &workspaces);
            Err(report_to_mcp_error(&error_stack::Report::new(
                Error::WorkspaceDisambiguation {
                    message:              error_msg,
                    item_type:            item_type.to_string(),
                    item_name:            item_name.to_string(),
                    available_workspaces: workspaces,
                },
            )))
        }
    }
}
