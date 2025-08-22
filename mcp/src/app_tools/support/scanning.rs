use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tracing::debug;

use super::cargo_detector::{BevyTarget, CargoDetector, TargetType};
use super::errors::{NoTargetsFoundError, PathDisambiguationError};
use crate::error::Error;

/// Helper function to safely canonicalize a path
/// Returns the canonicalized path if successful, otherwise returns the original path
fn safe_canonicalize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(e) => {
            debug!("Failed to canonicalize path '{}': {}", path.display(), e);
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
pub fn iter_cargo_project_paths(search_paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut visited_canonical = HashSet::new();
    let mut discovered_projects: HashMap<PathBuf, DiscoveredProject> = HashMap::new();

    for root in search_paths {
        let canonical_root = safe_canonicalize(root);
        shallow_scan(
            &canonical_root,
            &mut visited_canonical,
            &mut discovered_projects,
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
) {
    for package in &metadata.packages {
        // Only include packages that are workspace members
        if metadata.workspace_members.contains(&package.id) {
            let manifest_path =
                safe_canonicalize(&PathBuf::from(&package.manifest_path.as_std_path()));
            if let Some(member_dir) = manifest_path.parent() {
                if member_dir.exists() {
                    let member_canonical = safe_canonicalize(member_dir);

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
                    debug!(
                        "Skipping workspace member '{}': directory does not exist at '{}'",
                        package.name,
                        member_dir.display()
                    );
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
) {
    if metadata.workspace_members.len() > 1 {
        // This is a true workspace root - discover all its members
        discover_workspace_members(metadata, workspace_root, discovered_projects);
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
) {
    let canonical_dir = safe_canonicalize(dir);
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
) {
    let canonical_dir = safe_canonicalize(dir);
    // Only add as filesystem discovery if not already discovered as workspace member
    discovered_projects
        .entry(canonical_dir.clone())
        .or_insert(DiscoveredProject {
            path:         canonical_dir,
            project_type: ProjectType::Standalone,
        });
}

fn process_cargo_toml(dir: &Path, discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>) {
    // Try to get metadata to determine if it's a workspace
    if let Ok(metadata) = cargo_metadata::MetadataCommand::new()
        .current_dir(dir)
        .exec()
    {
        let workspace_root: PathBuf = metadata.workspace_root.clone().into();

        // Check if this is a workspace root (not just a member)
        let canonical_dir = safe_canonicalize(dir);
        let canonical_workspace = safe_canonicalize(&workspace_root);
        let is_workspace_root = canonical_dir == canonical_workspace;

        if is_workspace_root {
            handle_workspace_root(
                &metadata,
                &workspace_root,
                canonical_dir,
                discovered_projects,
            );
        } else {
            add_workspace_member(dir, workspace_root, discovered_projects);
        }
    } else {
        add_fallback_standalone(dir, discovered_projects);
    }
}

/// Shallow scan a directory for Cargo projects (current + immediate subdirectories only)
fn shallow_scan(
    dir: &Path,
    visited_canonical: &mut HashSet<PathBuf>,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
) {
    shallow_scan_internal(dir, visited_canonical, discovered_projects, false);
}

/// Internal shallow scan that can skip the `should_skip` check for root paths
/// Maximum depth: current directory + immediate subdirectories only (2 levels)
fn shallow_scan_internal(
    dir: &Path,
    visited_canonical: &mut HashSet<PathBuf>,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    check_skip: bool,
) {
    // Skip if we've already visited this canonical path
    let canonical = safe_canonicalize(dir);
    if !visited_canonical.insert(canonical) {
        return;
    }

    // Skip hidden directories and target directories (but not for root search paths)
    if check_skip && should_skip_directory(dir) {
        return;
    }

    // Level 0: Check if this directory contains a Cargo.toml
    let cargo_toml = dir.join("Cargo.toml");
    if cargo_toml.exists() {
        process_cargo_toml(dir, discovered_projects);
    }

    // Level 1: Check immediate subdirectories only (no recursion)
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && !should_skip_directory(&path) {
                // Check for Cargo.toml in immediate subdirectory
                let sub_cargo_toml = path.join("Cargo.toml");
                if sub_cargo_toml.exists() {
                    // Skip if we've already visited this canonical path
                    let sub_canonical = safe_canonicalize(&path);
                    if visited_canonical.insert(sub_canonical) {
                        process_cargo_toml(&path, discovered_projects);
                    }
                }
            }
        }
    }
}

/// Extract workspace name from workspace root path
/// Returns the last component of the path as the workspace name
/// Compute the relative path from the search roots to the given path
/// This is used to provide a stable identifier for disambiguation
///
/// Special handling for empty relative paths:
/// When the discovered path exactly matches a search path (e.g., when a Cargo project
/// is directly in a search root), the relative path would be empty. In this case,
/// we use the directory name itself as the identifier to ensure round-trip compatibility
/// with path parameters. For example, if searching in "/workspace/test-app" and finding
/// a project at that exact path, we return "test-app" rather than an empty path.
pub fn compute_relative_path(path: &Path, search_paths: &[PathBuf]) -> PathBuf {
    // Try to find which search path this path is under
    for search_path in search_paths {
        let search_canonical = safe_canonicalize(search_path);
        let path_canonical = safe_canonicalize(path);
        if let Ok(relative) = path_canonical.strip_prefix(&search_canonical) {
            // Special case: If the relative path is empty (meaning path == search_path),
            // we need a meaningful identifier for round-trip compatibility
            if relative.as_os_str().is_empty() {
                // Use the last component of the path as the identifier
                // This ensures paths like "test-app" work in both list and launch functions
                if let Some(name) = path_canonical.file_name() {
                    return PathBuf::from(name);
                }
                // Fallback to "." if we can't get a name (shouldn't happen in practice)
                return PathBuf::from(".");
            }
            return relative.to_path_buf();
        }
    }

    // If we can't compute a relative path, return the path as-is relative to the current directory
    // This ensures full path information is preserved for disambiguation
    path.to_path_buf()
}

/// Find all targets (apps and examples) by name across search paths, filtered by target type if
/// specified This allows detection of duplicates across workspaces
pub fn find_all_targets_by_name(
    target_name: &str,
    target_type: Option<TargetType>,
    search_paths: &[PathBuf],
) -> Vec<BevyTarget> {
    let mut targets = Vec::new();

    for path in iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let found_targets = detector.find_bevy_targets();
            for mut target in found_targets {
                if target.name == target_name {
                    // Filter by target type if specified
                    if let Some(required_type) = target_type
                        && target.target_type != required_type
                    {
                        continue;
                    }

                    // Set the relative path based on the discovered project path
                    target.relative_path = compute_relative_path(&path, search_paths);
                    targets.push(target);
                }
            }
        }
    }

    targets
}

/// Find a required target by name with path parameter handling
/// Returns an error with enhanced path error messages if duplicates found and no path specified
pub fn find_required_target_with_path(
    target_name: &str,
    target_type: TargetType,
    path: Option<&str>,
    search_paths: &[PathBuf],
) -> Result<BevyTarget, Error> {
    let target_type_str = match target_type {
        TargetType::App => "app",
        TargetType::Example => "example",
    };

    debug!("Searching for {target_type_str} '{target_name}'");
    if let Some(p) = path {
        debug!("With path filter: {p}");
    }

    let all_targets = find_all_targets_by_name(target_name, Some(target_type), search_paths);
    debug!("Found {} matching {target_type_str}(s)", all_targets.len());

    // If a path is provided and we found multiple targets, check for ambiguity
    if let Some(path_str) = path
        && all_targets.len() > 1
    {
        let filtered_targets =
            find_and_filter_by_path(all_targets.clone(), path, |target| &target.relative_path);

        // If filtering resulted in 0 matches but there were multiple targets,
        // check if the path could have been ambiguous
        if filtered_targets.is_empty() {
            // Check if the path partially matches multiple targets
            let partial_matches: Vec<_> = all_targets
                .iter()
                .filter(|target| {
                    let relative_path = &target.relative_path;
                    partial_path_match(relative_path, path_str)
                })
                .collect();

            if partial_matches.len() > 1 {
                // This is an ambiguous partial path
                let paths: Vec<String> = partial_matches
                    .iter()
                    .map(|target| target.relative_path.to_string_lossy().to_string())
                    .collect();

                let path_disambiguation_error = PathDisambiguationError::new(
                    paths,
                    target_name.to_string(),
                    target_type_str.to_string(),
                );
                return Err(Error::Structured {
                    result: Box::new(path_disambiguation_error),
                });
            }

            // Enhanced error message for path not found
            let available_paths: Vec<String> = all_targets
                .iter()
                .map(|target| target.relative_path.to_string_lossy().to_string())
                .collect();

            let path_disambiguation_error = PathDisambiguationError::new(
                available_paths,
                target_name.to_string(),
                target_type_str.to_string(),
            );
            return Err(Error::Structured {
                result: Box::new(path_disambiguation_error),
            });
        }

        return validate_single_result_or_error(
            filtered_targets,
            target_name,
            target_type_str,
            |target| &target.relative_path,
        );
    }

    let filtered_targets =
        find_and_filter_by_path(all_targets, path, |target| &target.relative_path);

    validate_single_result_or_error(filtered_targets, target_name, target_type_str, |target| {
        &target.relative_path
    })
}

/// Check if the relative path exactly matches the provided path string
fn exact_path_match(relative_path: &Path, path_str: &str) -> bool {
    relative_path.to_string_lossy() == path_str
}

/// Check if the relative path ends with the provided path (partial match)
fn partial_path_match(relative_path: &Path, path_str: &str) -> bool {
    if let Some(path_str_path) = Path::new(path_str).to_str() {
        if let Some(relative_str) = relative_path.to_str() {
            // Check if it ends with the path (suffix match)
            if relative_str.ends_with(path_str_path) {
                return true;
            }
            // Also check if the path string is contained within any path component
            // This handles cases like "duplicate" matching "test-duplicate-a"
            for component in relative_path.components() {
                if let Some(component_str) = component.as_os_str().to_str() {
                    if component_str.contains(path_str) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Find items by name and filter by path if provided
/// Prioritizes exact matches over partial matches
fn find_and_filter_by_path<T>(
    all_items: Vec<T>,
    path: Option<&str>,
    get_relative_path: impl Fn(&T) -> &PathBuf,
) -> Vec<T> {
    if let Some(path_str) = path {
        // First, check if there are any exact matches
        let has_exact_match = all_items
            .iter()
            .any(|item| exact_path_match(get_relative_path(item), path_str));

        if has_exact_match {
            // Return only exact matches
            all_items
                .into_iter()
                .filter(|item| exact_path_match(get_relative_path(item), path_str))
                .collect()
        } else {
            // No exact matches, fall back to partial matches
            all_items
                .into_iter()
                .filter(|item| partial_path_match(get_relative_path(item), path_str))
                .collect()
        }
    } else {
        all_items
    }
}

/// Validate that exactly one item was found, or return helpful error
fn validate_single_result_or_error<T>(
    items: Vec<T>,
    item_name: &str,
    item_type: &str,
    get_relative_path: impl Fn(&T) -> &PathBuf,
) -> Result<T, Error> {
    match items.len() {
        0 => {
            let no_targets_error =
                NoTargetsFoundError::new(item_name.to_string(), item_type.to_string());
            Err(Error::Structured {
                result: Box::new(no_targets_error),
            })
        }
        1 => {
            // We know exactly one item exists
            let mut iter = items.into_iter();
            iter.next().map_or_else(
                || {
                    let no_targets_error =
                        NoTargetsFoundError::new(item_name.to_string(), item_type.to_string());
                    Err(Error::Structured {
                        result: Box::new(no_targets_error),
                    })
                },
                |item| Ok(item),
            )
        }
        _ => {
            let all_paths: Vec<String> = items
                .iter()
                .map(|item| {
                    let relative_path = get_relative_path(item);
                    relative_path.to_string_lossy().to_string()
                })
                .collect();

            let non_empty_paths: Vec<String> = all_paths
                .iter()
                .filter(|path| !path.is_empty())
                .cloned()
                .collect();

            let path_disambiguation_error = PathDisambiguationError::new(
                non_empty_paths,
                item_name.to_string(),
                item_type.to_string(),
            );
            Err(Error::Structured {
                result: Box::new(path_disambiguation_error),
            })
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
        let result = safe_canonicalize(path);

        // Should return a valid path without errors
        assert!(result.is_absolute());
    }

    #[test]
    fn test_safe_canonicalize_with_invalid_path() {
        let path = Path::new("/non/existent/path/that/does/not/exist");
        let result = safe_canonicalize(path);

        // Should return the original path and log an error
        assert_eq!(result, path.to_path_buf());
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
            relative_path: PathBuf,
        }

        let items = vec![
            TestItem {
                relative_path: PathBuf::from("workspace1/app1"),
            },
            TestItem {
                relative_path: PathBuf::from("workspace2/app1"),
            },
            TestItem {
                relative_path: PathBuf::from("workspace1/app2"),
            },
        ];

        // Test exact path matching
        let filtered =
            find_and_filter_by_path(items, Some("workspace1/app1"), |item| &item.relative_path);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].relative_path, PathBuf::from("workspace1/app1"));
    }

    #[test]
    fn test_find_and_filter_by_path_suffix() {
        // Create test items with relative paths
        #[derive(Debug, Clone)]
        struct TestItem {
            relative_path: PathBuf,
        }

        let items = vec![
            TestItem {
                relative_path: PathBuf::from("workspace1/app1"),
            },
            TestItem {
                relative_path: PathBuf::from("workspace2/app1"),
            },
            TestItem {
                relative_path: PathBuf::from("workspace1/app2"),
            },
        ];

        // Test suffix matching
        let filtered = find_and_filter_by_path(items, Some("app1"), |item| &item.relative_path);

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
    fn test_compute_relative_path() {
        let search_paths = vec![
            PathBuf::from("/home/user/projects"),
            PathBuf::from("/home/user/work"),
        ];

        // Path under first search path
        let path = PathBuf::from("/home/user/projects/my-app");
        let relative = compute_relative_path(&path, &search_paths);
        assert_eq!(relative, PathBuf::from("my-app"));

        // Path not under any search path - should return full path for proper disambiguation
        let path = PathBuf::from("/home/user/other/my-app");
        let relative = compute_relative_path(&path, &search_paths);
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

        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()]);

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

        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()]);

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

        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()]);

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

        // Test workspace root detection
        process_cargo_toml(&workspace_path.join("workspace"), &mut discovered_projects);

        // Test standalone project detection
        process_cargo_toml(
            &standalone_path.join("standalone"),
            &mut discovered_projects,
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
