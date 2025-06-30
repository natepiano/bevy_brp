use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rmcp::Error as McpError;

use super::cargo_detector::{BinaryInfo, CargoDetector, ExampleInfo};
use crate::error::{Error, report_to_mcp_error};

/// Helper function to safely canonicalize a path
/// Returns the canonicalized path if successful, otherwise returns the original path
fn safe_canonicalize(path: &Path, debug_info: Option<&mut Vec<String>>) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(e) => {
            if let Some(debug_info) = debug_info {
                debug_info.push(format!(
                    "Failed to canonicalize path '{}': {}",
                    path.display(),
                    e
                ));
            }
            path.to_path_buf()
        }
    }
}

/// Type of discovered project
#[derive(Debug, Clone)]
enum ProjectType {
    /// A workspace member with its workspace root
    Workspace { workspace_root: PathBuf },
    /// A standalone project
    Standalone,
}

/// Represents a discovered Cargo project with its discovery context
#[derive(Debug, Clone)]
struct DiscoveredProject {
    /// Path to the directory containing Cargo.toml
    path:         PathBuf,
    /// Type of project (workspace member or standalone)
    project_type: ProjectType,
}

/// Iterator over all valid Cargo project paths found in the given search paths
/// Recursively scans all directories at any depth
/// Smart deduplication: workspace-discovered apps take precedence over filesystem-discovered
pub fn iter_cargo_project_paths(
    search_paths: &[PathBuf],
    debug_info: &mut Vec<String>,
) -> Vec<PathBuf> {
    let mut visited_canonical = HashSet::new();
    let mut discovered_projects: HashMap<PathBuf, DiscoveredProject> = HashMap::new();

    for root in search_paths {
        let canonical_root = safe_canonicalize(root, Some(debug_info));
        recursive_scan(
            &canonical_root,
            &mut visited_canonical,
            &mut discovered_projects,
            debug_info,
        );
    }

    // Apply smart deduplication and return paths
    let mut final_paths = HashSet::new();
    let mut workspace_members = HashSet::new();

    // First pass: collect all workspace members
    for project in discovered_projects.values() {
        if matches!(project.project_type, ProjectType::Workspace { .. }) {
            workspace_members.insert(project.path.clone());
        }
    }

    // Second pass: add paths with proper attribution
    for project in discovered_projects.values() {
        match &project.project_type {
            ProjectType::Workspace { workspace_root } => {
                // For workspace members, use the workspace root
                final_paths.insert(workspace_root.clone());
            }
            ProjectType::Standalone => {
                // For standalone projects (not found as workspace members), use their actual path
                if !workspace_members.contains(&project.path) {
                    final_paths.insert(project.path.clone());
                }
            }
        }
        // If a project was found both ways, the workspace discovery takes precedence
    }

    final_paths.into_iter().collect()
}

/// Check if a directory should be skipped during scanning
fn should_skip_directory(dir: &Path) -> bool {
    dir.file_name().is_some_and(|name| {
        let name_str = name.to_string_lossy();
        name_str.starts_with('.') || name_str == "target"
    })
}

/// Discover workspace members from metadata
fn discover_workspace_members(
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
) {
    for package in &metadata.packages {
        // Only include packages that are workspace members
        if metadata.workspace_members.contains(&package.id) {
            let manifest_path = safe_canonicalize(
                &PathBuf::from(&package.manifest_path.as_std_path()),
                Some(debug_info),
            );
            if let Some(member_dir) = manifest_path.parent() {
                if member_dir.exists() {
                    let member_canonical = safe_canonicalize(member_dir, Some(debug_info));

                    discovered_projects.insert(
                        member_canonical.clone(),
                        DiscoveredProject {
                            path:         member_canonical,
                            project_type: ProjectType::Workspace {
                                workspace_root: workspace_root.to_path_buf(),
                            },
                        },
                    );
                } else {
                    debug_info.push(format!(
                        "Skipping workspace member '{}': directory does not exist at '{}'",
                        package.name,
                        member_dir.display()
                    ));
                }
            }
        }
    }
}

/// Process a directory that contains a Cargo.toml file
/// Handle workspace root discovery (either true workspace or standalone project)
fn handle_workspace_root(
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
    canonical_dir: PathBuf,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
) {
    if metadata.workspace_members.len() > 1 {
        // This is a true workspace root - discover all its members
        discover_workspace_members(metadata, workspace_root, discovered_projects, debug_info);
    } else {
        // This is a standalone project (single-member workspace)
        discovered_projects.insert(
            canonical_dir.clone(),
            DiscoveredProject {
                path:         canonical_dir,
                project_type: ProjectType::Standalone,
            },
        );
    }
}

/// Add a workspace member project to the discovered projects
fn add_workspace_member(
    dir: &Path,
    workspace_root: PathBuf,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
) {
    let canonical_dir = safe_canonicalize(dir, Some(debug_info));
    discovered_projects.insert(
        canonical_dir.clone(),
        DiscoveredProject {
            path:         canonical_dir,
            project_type: ProjectType::Workspace { workspace_root },
        },
    );
}

/// Add a standalone project (fallback for invalid cargo projects)
fn add_fallback_standalone(
    dir: &Path,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
) {
    let canonical_dir = safe_canonicalize(dir, Some(debug_info));
    // Only add as filesystem discovery if not already discovered as workspace member
    discovered_projects
        .entry(canonical_dir.clone())
        .or_insert(DiscoveredProject {
            path:         canonical_dir,
            project_type: ProjectType::Standalone,
        });
}

fn process_cargo_toml(
    dir: &Path,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
) {
    // Try to get metadata to determine if it's a workspace
    if let Ok(metadata) = cargo_metadata::MetadataCommand::new()
        .current_dir(dir)
        .exec()
    {
        let workspace_root: PathBuf = metadata.workspace_root.clone().into();

        // Check if this is a workspace root (not just a member)
        let canonical_dir = safe_canonicalize(dir, Some(debug_info));
        let canonical_workspace = safe_canonicalize(&workspace_root, Some(debug_info));
        let is_workspace_root = canonical_dir == canonical_workspace;

        if is_workspace_root {
            handle_workspace_root(
                &metadata,
                &workspace_root,
                canonical_dir,
                discovered_projects,
                debug_info,
            );
        } else {
            add_workspace_member(dir, workspace_root, discovered_projects, debug_info);
        }
    } else {
        add_fallback_standalone(dir, discovered_projects, debug_info);
    }
}

/// Recursively scan a directory for Cargo projects
fn recursive_scan(
    dir: &Path,
    visited_canonical: &mut HashSet<PathBuf>,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
) {
    recursive_scan_internal(
        dir,
        visited_canonical,
        discovered_projects,
        debug_info,
        false,
    );
}

/// Internal recursive scan that can skip the `should_skip` check for root paths
fn recursive_scan_internal(
    dir: &Path,
    visited_canonical: &mut HashSet<PathBuf>,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    debug_info: &mut Vec<String>,
    check_skip: bool,
) {
    // Skip if we've already visited this canonical path
    let canonical = safe_canonicalize(dir, Some(debug_info));
    if !visited_canonical.insert(canonical) {
        return;
    }

    // Skip hidden directories and target directories (but not for root search paths)
    if check_skip && should_skip_directory(dir) {
        return;
    }

    // Check if this directory contains a Cargo.toml
    let cargo_toml = dir.join("Cargo.toml");
    if cargo_toml.exists() {
        process_cargo_toml(dir, discovered_projects, debug_info);
    }

    // Recurse into all subdirectories (check skip for all subdirectories)
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                recursive_scan_internal(
                    &path,
                    visited_canonical,
                    discovered_projects,
                    debug_info,
                    true,
                );
            }
        }
    }
}

/// Extract workspace name from workspace root path
/// Returns the last component of the path as the workspace name
pub fn extract_workspace_name(workspace_root: &Path) -> Option<String> {
    workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(std::string::ToString::to_string)
}

/// Compute the relative path from the search roots to the given path
/// This is used to provide a stable identifier for disambiguation
fn compute_relative_path(
    path: &Path,
    search_paths: &[PathBuf],
    debug_info: &mut Vec<String>,
) -> PathBuf {
    // Try to find which search path this path is under
    for search_path in search_paths {
        let search_canonical = safe_canonicalize(search_path, Some(debug_info));
        let path_canonical = safe_canonicalize(path, Some(debug_info));
        if let Ok(relative) = path_canonical.strip_prefix(&search_canonical) {
            return relative.to_path_buf();
        }
    }

    // If we can't compute a relative path, return the path as-is relative to the current directory
    // This ensures full path information is preserved for disambiguation
    path.to_path_buf()
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
    let mut debug_info = Vec::new();

    for path in iter_cargo_project_paths(search_paths, &mut debug_info) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let found_apps = detector.find_bevy_apps();
            for mut app in found_apps {
                if app.name == app_name {
                    // Set the relative path based on the discovered project path
                    app.relative_path = compute_relative_path(&path, search_paths, &mut debug_info);
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
    let mut debug_info = Vec::new();

    for path in iter_cargo_project_paths(search_paths, &mut debug_info) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let found_examples = detector.find_bevy_examples();
            for mut example in found_examples {
                if example.name == example_name {
                    // Set the relative path based on the discovered project path
                    example.relative_path =
                        compute_relative_path(&path, search_paths, &mut debug_info);
                    examples.push(example);
                }
            }
        }
    }

    examples
}

/// Find a required app by name with path parameter handling
/// Returns an error with path options if duplicates found and no path specified
pub fn find_required_app_with_path(
    app_name: &str,
    path: Option<&str>,
    search_paths: &[PathBuf],
    debug_info: &mut Vec<String>,
) -> Result<BinaryInfo, McpError> {
    debug_info.push(format!("Searching for app '{app_name}'"));
    if let Some(p) = path {
        debug_info.push(format!("With path filter: {p}"));
    }

    let all_apps = find_all_apps_by_name(app_name, search_paths);
    debug_info.push(format!("Found {} matching app(s)", all_apps.len()));

    let filtered_apps = find_and_filter_by_path(
        all_apps,
        path,
        |app| Some(app.workspace_root.clone()),
        |app| &app.relative_path,
    );

    validate_single_result_or_error(filtered_apps, app_name, "app", "app_name", |app| {
        &app.relative_path
    })
}

/// Find a required example by name with path parameter handling
/// Returns an error with path options if duplicates found and no path specified
pub fn find_required_example_with_path(
    example_name: &str,
    path: Option<&str>,
    search_paths: &[PathBuf],
    debug_info: &mut Vec<String>,
) -> Result<ExampleInfo, McpError> {
    debug_info.push(format!("Searching for example '{example_name}'"));
    if let Some(p) = path {
        debug_info.push(format!("With path filter: {p}"));
    }

    let all_examples = find_all_examples_by_name(example_name, search_paths);
    debug_info.push(format!("Found {} matching example(s)", all_examples.len()));

    let filtered_examples = find_and_filter_by_path(
        all_examples,
        path,
        |example| get_workspace_root_from_manifest(&example.manifest_path),
        |example| &example.relative_path,
    );

    validate_single_result_or_error(
        filtered_examples,
        example_name,
        "example",
        "example_name",
        |example| &example.relative_path,
    )
}

/// Build error message for duplicate items across paths
fn build_path_selection_error(
    item_type: &str,
    item_name: &str,
    param_name: &str,
    paths: &[String],
) -> String {
    let path_list = paths
        .iter()
        .map(|p| format!("- {p}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Found multiple {item_type} named '{item_name}' at:\n{path_list}\n\nPlease specify which path to use:\n{{\"{param_name}\": \"{item_name}\", \"path\": \"path_name\"}}"
    )
}

/// Check if the relative path exactly matches the provided path string
fn exact_path_match(relative_path: &Path, path_str: &str) -> bool {
    relative_path.to_string_lossy() == path_str
}

/// Check if the relative path ends with the provided path (partial match)
fn partial_path_match(relative_path: &Path, path_str: &str) -> bool {
    if let Some(path_str_path) = Path::new(path_str).to_str() {
        if let Some(relative_str) = relative_path.to_str() {
            return relative_str.ends_with(path_str_path);
        }
    }
    false
}

/// Check if the workspace name matches the provided path string (for disambiguation)
fn workspace_name_match(workspace_root: Option<PathBuf>, path_str: &str) -> bool {
    if let Some(root) = workspace_root {
        if let Some(item_workspace) = extract_workspace_name(&root) {
            return item_workspace == path_str;
        }
    }
    false
}

/// Check if an item matches the given path using all available strategies
fn item_matches_path<T>(
    item: &T,
    path_str: &str,
    get_workspace_root: &impl Fn(&T) -> Option<PathBuf>,
    get_relative_path: &impl Fn(&T) -> &PathBuf,
) -> bool {
    let relative_path = get_relative_path(item);

    // Try exact match first
    if exact_path_match(relative_path, path_str) {
        return true;
    }

    // Then try partial match
    if partial_path_match(relative_path, path_str) {
        return true;
    }

    // Finally, fall back to workspace name matching
    workspace_name_match(get_workspace_root(item), path_str)
}

/// Find items by name and filter by path if provided
/// Supports full relative paths, partial paths, and workspace name fallback
fn find_and_filter_by_path<T>(
    all_items: Vec<T>,
    path: Option<&str>,
    get_workspace_root: impl Fn(&T) -> Option<PathBuf>,
    get_relative_path: impl Fn(&T) -> &PathBuf,
) -> Vec<T> {
    if let Some(path_str) = path {
        all_items
            .into_iter()
            .filter(|item| {
                item_matches_path(item, path_str, &get_workspace_root, &get_relative_path)
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
    get_relative_path: impl Fn(&T) -> &PathBuf,
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
            let paths: Vec<String> = items
                .iter()
                .map(|item| {
                    let relative_path = get_relative_path(item);
                    relative_path.to_string_lossy().to_string()
                })
                .filter(|path| !path.is_empty())
                .collect();

            let error_msg = build_path_selection_error(item_type, item_name, param_name, &paths);
            Err(report_to_mcp_error(&error_stack::Report::new(
                Error::PathDisambiguation {
                    message:         error_msg,
                    item_type:       item_type.to_string(),
                    item_name:       item_name.to_string(),
                    available_paths: paths,
                },
            )))
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_safe_canonicalize_with_valid_path() {
        let path = Path::new(".");
        let mut debug_info = Vec::new();
        let result = safe_canonicalize(path, Some(&mut debug_info));

        // Should return a valid path without errors
        assert!(result.is_absolute());
        assert!(debug_info.is_empty());
    }

    #[test]
    fn test_safe_canonicalize_with_invalid_path() {
        let path = Path::new("/non/existent/path/that/does/not/exist");
        let mut debug_info = Vec::new();
        let result = safe_canonicalize(path, Some(&mut debug_info));

        // Should return the original path and log an error
        assert_eq!(result, path.to_path_buf());
        assert!(!debug_info.is_empty());
        assert!(debug_info[0].contains("Failed to canonicalize"));
    }

    #[test]
    fn test_should_skip_directory() {
        // Should skip hidden directories
        assert!(should_skip_directory(Path::new(".git")));
        assert!(should_skip_directory(Path::new(".cargo")));

        // Should skip target directories
        assert!(should_skip_directory(Path::new("target")));

        // Should not skip normal directories
        assert!(!should_skip_directory(Path::new("src")));
        assert!(!should_skip_directory(Path::new("tests")));
    }

    #[test]
    fn test_find_and_filter_by_path_exact() {
        // Create test items with relative paths
        #[derive(Debug, Clone)]
        struct TestItem {
            relative_path:  PathBuf,
            workspace_root: Option<PathBuf>,
        }

        let items = vec![
            TestItem {
                relative_path:  PathBuf::from("workspace1/app1"),
                workspace_root: Some(PathBuf::from("/home/user/workspace1")),
            },
            TestItem {
                relative_path:  PathBuf::from("workspace2/app1"),
                workspace_root: Some(PathBuf::from("/home/user/workspace2")),
            },
            TestItem {
                relative_path:  PathBuf::from("workspace1/app2"),
                workspace_root: Some(PathBuf::from("/home/user/workspace1")),
            },
        ];

        // Test exact path matching
        let filtered = find_and_filter_by_path(
            items,
            Some("workspace1/app1"),
            |item| item.workspace_root.clone(),
            |item| &item.relative_path,
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].relative_path, PathBuf::from("workspace1/app1"));
    }

    #[test]
    fn test_find_and_filter_by_path_suffix() {
        // Create test items with relative paths
        #[derive(Debug, Clone)]
        struct TestItem {
            relative_path:  PathBuf,
            workspace_root: Option<PathBuf>,
        }

        let items = vec![
            TestItem {
                relative_path:  PathBuf::from("workspace1/app1"),
                workspace_root: Some(PathBuf::from("/home/user/workspace1")),
            },
            TestItem {
                relative_path:  PathBuf::from("workspace2/app1"),
                workspace_root: Some(PathBuf::from("/home/user/workspace2")),
            },
            TestItem {
                relative_path:  PathBuf::from("workspace1/app2"),
                workspace_root: Some(PathBuf::from("/home/user/workspace1")),
            },
        ];

        // Test suffix matching
        let filtered = find_and_filter_by_path(
            items,
            Some("app1"),
            |item| item.workspace_root.clone(),
            |item| &item.relative_path,
        );

        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .any(|i| i.relative_path == PathBuf::from("workspace1/app1"))
        );
        assert!(
            filtered
                .iter()
                .any(|i| i.relative_path == PathBuf::from("workspace2/app1"))
        );
    }

    #[test]
    fn test_extract_workspace_name() {
        let workspace_path = PathBuf::from("/home/user/projects/my-workspace");
        let name = extract_workspace_name(&workspace_path);
        assert_eq!(name, Some("my-workspace".to_string()));

        // Test with trailing slash
        let workspace_path = PathBuf::from("/home/user/projects/my-workspace/");
        let name = extract_workspace_name(&workspace_path);
        assert_eq!(name, Some("my-workspace".to_string()));

        // Test with root path
        let workspace_path = PathBuf::from("/");
        let name = extract_workspace_name(&workspace_path);
        assert_eq!(name, None);
    }

    #[test]
    fn test_compute_relative_path() {
        let mut debug_info = Vec::new();
        let search_paths = vec![
            PathBuf::from("/home/user/projects"),
            PathBuf::from("/home/user/work"),
        ];

        // Path under first search path
        let path = PathBuf::from("/home/user/projects/my-app");
        let relative = compute_relative_path(&path, &search_paths, &mut debug_info);
        assert_eq!(relative, PathBuf::from("my-app"));

        // Path not under any search path - should return full path for proper disambiguation
        let path = PathBuf::from("/home/user/other/my-app");
        let relative = compute_relative_path(&path, &search_paths, &mut debug_info);
        assert_eq!(relative, PathBuf::from("/home/user/other/my-app"));
    }

    #[test]
    fn test_recursive_scan_with_hidden_directories() {
        use std::fs;

        use tempfile::TempDir;

        // Create a temporary directory structure
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        // Create valid Cargo project structure
        fs::create_dir_all(temp_path.join("test-project/src")).expect("Failed to create src dir");
        fs::write(
            temp_path.join("test-project/Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "test-project"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write Cargo.toml");
        fs::write(
            temp_path.join("test-project/src/main.rs"),
            r#"fn main() {
    println!("Hello, world!");
}"#,
        )
        .expect("Failed to write main.rs");

        // Create hidden directories that should be skipped
        fs::create_dir_all(temp_path.join("test-project/.git/objects"))
            .expect("Failed to create .git dir");
        fs::create_dir_all(temp_path.join("test-project/target/debug"))
            .expect("Failed to create target dir");

        // Create another valid project in hidden dir (should be skipped)
        fs::create_dir_all(temp_path.join("test-project/.hidden/hidden-project/src"))
            .expect("Failed to create hidden project");
        fs::write(
            temp_path.join("test-project/.hidden/hidden-project/Cargo.toml"),
            r#"[package]
name = "hidden-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "hidden-project"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write hidden Cargo.toml");
        fs::write(
            temp_path.join("test-project/.hidden/hidden-project/src/main.rs"),
            r#"fn main() {
    println!("Hidden project");
}"#,
        )
        .expect("Failed to write hidden main.rs");

        let mut debug_info = Vec::new();
        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()], &mut debug_info);

        // Should find only the main project, not the hidden one
        assert_eq!(paths.len(), 1, "Should find exactly one project");
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("test-project"))
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.to_string_lossy().contains("hidden-project"))
        );
    }

    #[test]
    fn test_recursive_scan_cycle_detection() {
        use std::fs;
        use std::os::unix::fs::symlink;

        use tempfile::TempDir;

        // Create a temporary directory structure
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        // Create valid Cargo project structure
        fs::create_dir_all(temp_path.join("project-a/src"))
            .expect("Failed to create project-a src");
        fs::write(
            temp_path.join("project-a/Cargo.toml"),
            r#"[package]
name = "project-a"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "project-a"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write project-a Cargo.toml");
        fs::write(
            temp_path.join("project-a/src/main.rs"),
            r#"fn main() {
    println!("Project A");
}"#,
        )
        .expect("Failed to write project-a main.rs");

        fs::create_dir_all(temp_path.join("project-b/src"))
            .expect("Failed to create project-b src");
        fs::write(
            temp_path.join("project-b/Cargo.toml"),
            r#"[package]
name = "project-b"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "project-b"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write project-b Cargo.toml");
        fs::write(
            temp_path.join("project-b/src/main.rs"),
            r#"fn main() {
    println!("Project B");
}"#,
        )
        .expect("Failed to write project-b main.rs");

        // Create symlinks to create a cycle
        symlink(
            temp_path.join("project-b"),
            temp_path.join("project-a/link-to-b"),
        )
        .expect("Failed to create symlink a->b");
        symlink(
            temp_path.join("project-a"),
            temp_path.join("project-b/link-to-a"),
        )
        .expect("Failed to create symlink b->a");

        let mut debug_info = Vec::new();
        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()], &mut debug_info);

        // Should find both projects without infinite loop
        assert_eq!(paths.len(), 2, "Should find exactly two projects");
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("project-a"))
        );
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("project-b"))
        );
    }

    #[test]
    fn test_nested_workspace_discovery() {
        use std::fs;

        use tempfile::TempDir;

        // Create a temporary directory structure
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        // Create workspace root
        fs::create_dir_all(temp_path.join("workspace")).expect("Failed to create workspace dir");
        fs::write(
            temp_path.join("workspace/Cargo.toml"),
            r#"[workspace]
members = ["member-a", "member-b"]
resolver = "2"

[workspace.package]
edition = "2021"
"#,
        )
        .expect("Failed to write workspace Cargo.toml");

        // Create workspace members
        fs::create_dir_all(temp_path.join("workspace/member-a/src"))
            .expect("Failed to create member-a src");
        fs::write(
            temp_path.join("workspace/member-a/Cargo.toml"),
            r#"[package]
name = "member-a"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "member-a"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write member-a Cargo.toml");
        fs::write(
            temp_path.join("workspace/member-a/src/main.rs"),
            r#"fn main() {
    println!("Member A");
}"#,
        )
        .expect("Failed to write member-a main.rs");

        fs::create_dir_all(temp_path.join("workspace/member-b/src"))
            .expect("Failed to create member-b src");
        fs::write(
            temp_path.join("workspace/member-b/Cargo.toml"),
            r#"[package]
name = "member-b"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "member-b"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write member-b Cargo.toml");
        fs::write(
            temp_path.join("workspace/member-b/src/main.rs"),
            r#"fn main() {
    println!("Member B");
}"#,
        )
        .expect("Failed to write member-b main.rs");

        let mut debug_info = Vec::new();
        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()], &mut debug_info);

        // Should find the workspace root, not individual members
        assert_eq!(paths.len(), 1, "Should find exactly one workspace root");
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("workspace"))
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.to_string_lossy().contains("member-a"))
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.to_string_lossy().contains("member-b"))
        );
    }

    #[test]
    fn test_deeply_nested_recursive_scan() {
        use std::fs;

        use tempfile::TempDir;

        // Create a temporary directory structure
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        // Create deeply nested project structure (3+ levels)
        fs::create_dir_all(temp_path.join("level1/level2/level3/deep-project/src"))
            .expect("Failed to create deep structure");
        fs::write(
            temp_path.join("level1/level2/level3/deep-project/Cargo.toml"),
            r#"[package]
name = "deep-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "deep-project"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write deep Cargo.toml");
        fs::write(
            temp_path.join("level1/level2/level3/deep-project/src/main.rs"),
            r#"fn main() {
    println!("Deep project");
}"#,
        )
        .expect("Failed to write deep main.rs");

        // Create project at level 2
        fs::create_dir_all(temp_path.join("level1/level2/mid-project/src"))
            .expect("Failed to create mid project");
        fs::write(
            temp_path.join("level1/level2/mid-project/Cargo.toml"),
            r#"[package]
name = "mid-project" 
version = "0.1.0"
edition = "2021"

[[bin]]
name = "mid-project"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write mid Cargo.toml");
        fs::write(
            temp_path.join("level1/level2/mid-project/src/main.rs"),
            r#"fn main() {
    println!("Mid project");
}"#,
        )
        .expect("Failed to write mid main.rs");

        // Create project at root level
        fs::create_dir_all(temp_path.join("root-project/src"))
            .expect("Failed to create root project");
        fs::write(
            temp_path.join("root-project/Cargo.toml"),
            r#"[package]
name = "root-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "root-project"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write root Cargo.toml");
        fs::write(
            temp_path.join("root-project/src/main.rs"),
            r#"fn main() {
    println!("Root project");
}"#,
        )
        .expect("Failed to write root main.rs");

        let mut debug_info = Vec::new();
        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()], &mut debug_info);

        // Should find all three projects at different depths
        assert_eq!(paths.len(), 3, "Should find exactly three projects");
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("deep-project"))
        );
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("mid-project"))
        );
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("root-project"))
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_process_cargo_toml_workspace_detection() {
        use std::collections::HashMap;
        use std::fs;

        use tempfile::TempDir;

        // Create separate temp directories to avoid workspace interference
        let workspace_temp_dir = TempDir::new().expect("Failed to create workspace temp directory");
        let workspace_path = workspace_temp_dir.path();

        let standalone_temp_dir =
            TempDir::new().expect("Failed to create standalone temp directory");
        let standalone_path = standalone_temp_dir.path();

        // Test workspace root detection
        fs::create_dir_all(workspace_path.join("workspace"))
            .expect("Failed to create workspace dir");
        fs::write(
            workspace_path.join("workspace/Cargo.toml"),
            r#"[workspace]
members = ["app1", "app2"]
resolver = "2"

[workspace.package]
edition = "2021"
"#,
        )
        .expect("Failed to write workspace Cargo.toml");

        // Create workspace members
        fs::create_dir_all(workspace_path.join("workspace/app1/src"))
            .expect("Failed to create app1 src");
        fs::write(
            workspace_path.join("workspace/app1/Cargo.toml"),
            r#"[package]
name = "app1"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "app1"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write app1 Cargo.toml");
        fs::write(
            workspace_path.join("workspace/app1/src/main.rs"),
            r#"fn main() {
    println!("App 1");
}"#,
        )
        .expect("Failed to write app1 main.rs");

        fs::create_dir_all(workspace_path.join("workspace/app2/src"))
            .expect("Failed to create app2 src");
        fs::write(
            workspace_path.join("workspace/app2/Cargo.toml"),
            r#"[package]
name = "app2"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "app2"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write app2 Cargo.toml");
        fs::write(
            workspace_path.join("workspace/app2/src/main.rs"),
            r#"fn main() {
    println!("App 2");
}"#,
        )
        .expect("Failed to write app2 main.rs");

        // Create standalone project in separate directory
        fs::create_dir_all(standalone_path.join("standalone/src"))
            .expect("Failed to create standalone src");
        fs::write(
            standalone_path.join("standalone/Cargo.toml"),
            r#"[package]
name = "standalone"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "standalone"
path = "src/main.rs"
"#,
        )
        .expect("Failed to write standalone Cargo.toml");
        fs::write(
            standalone_path.join("standalone/src/main.rs"),
            r#"fn main() {
    println!("Standalone");
}"#,
        )
        .expect("Failed to write standalone main.rs");

        let mut discovered_projects = HashMap::new();
        let mut debug_info = Vec::new();

        // Test workspace root detection
        process_cargo_toml(
            &workspace_path.join("workspace"),
            &mut discovered_projects,
            &mut debug_info,
        );

        // Test standalone project detection
        process_cargo_toml(
            &standalone_path.join("standalone"),
            &mut discovered_projects,
            &mut debug_info,
        );

        // Should have workspace members and standalone project
        assert!(
            discovered_projects.len() >= 2,
            "Should find at least workspace members and standalone project"
        );

        // Check that workspace members are properly marked
        assert!(
            discovered_projects
                .values()
                .filter(|p| matches!(p.project_type, ProjectType::Workspace { .. }))
                .count()
                >= 2,
            "Should find at least 2 workspace members"
        );

        // Check that standalone project is properly marked
        assert!(
            discovered_projects
                .values()
                .filter(|p| matches!(p.project_type, ProjectType::Standalone))
                .count()
                >= 1,
            "Should find at least 1 standalone project"
        );
    }
}
