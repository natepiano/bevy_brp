use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use tracing::debug;

/// Safely canonicalize a path.
///
/// Returns the canonicalized path when available; otherwise returns the
/// original path unchanged.
pub(super) fn safe_canonicalize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(error) => {
            debug!("Failed to canonicalize '{}': {error}", path.display());
            path.to_path_buf()
        },
    }
}

#[derive(Debug, Clone)]
enum ProjectType {
    /// A workspace member with its workspace root.
    Workspace { workspace_root: PathBuf },
    /// A standalone project.
    Standalone,
}

#[derive(Debug, Clone)]
struct DiscoveredProject {
    /// Path to the directory containing `Cargo.toml`.
    path:         PathBuf,
    /// Type of project (workspace member or standalone).
    project_type: ProjectType,
}

/// Iterate over all valid Cargo project paths found in the given search paths.
///
/// The scan checks each root and its immediate subdirectories, then prefers
/// workspace-discovered apps over filesystem-discovered duplicates.
pub fn iter_cargo_project_paths(search_paths: &[PathBuf]) -> Vec<PathBuf> {
    let start = Instant::now();
    debug!(
        "Starting iter_cargo_project_paths with {} search paths",
        search_paths.len()
    );
    for (index, path) in search_paths.iter().enumerate() {
        debug!("  Search path {index}: {}", path.display());
    }

    let mut visited_canonical = HashSet::new();
    let mut discovered_projects: HashMap<PathBuf, DiscoveredProject> = HashMap::new();

    for root in search_paths {
        let root_start = Instant::now();
        let canonical_root = safe_canonicalize(root);
        debug!(
            "Scanning root: {} (canonical: {})",
            root.display(),
            canonical_root.display()
        );
        shallow_scan(
            &canonical_root,
            &mut visited_canonical,
            &mut discovered_projects,
        );
        debug!("Scanned {} in {:?}", root.display(), root_start.elapsed());
    }

    let mut final_paths = HashSet::new();
    let mut workspace_members = HashSet::new();

    for project in discovered_projects.values() {
        if matches!(project.project_type, ProjectType::Workspace { .. }) {
            workspace_members.insert(project.path.clone());
        }
    }

    for project in discovered_projects.values() {
        match &project.project_type {
            ProjectType::Workspace { workspace_root } => {
                final_paths.insert(workspace_root.clone());
            },
            ProjectType::Standalone => {
                if !workspace_members.contains(&project.path) {
                    final_paths.insert(project.path.clone());
                }
            },
        }
    }

    debug!(
        "iter_cargo_project_paths completed in {:?}",
        start.elapsed()
    );
    final_paths.into_iter().collect()
}

fn should_skip_directory(dir: &Path) -> bool {
    dir.file_name().is_some_and(|name| {
        let name_str = name.to_string_lossy();
        name_str.starts_with('.') || name_str == "target"
    })
}

fn discover_workspace_members(
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
) {
    for package in &metadata.packages {
        if metadata.workspace_members.contains(&package.id) {
            let manifest_path =
                safe_canonicalize(&PathBuf::from(&package.manifest_path.as_std_path()));
            let Some(member_dir) = manifest_path.parent() else {
                continue;
            };
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
                    "Skipping workspace member '{}': missing directory '{}'",
                    package.name,
                    member_dir.display()
                );
            }
        }
    }
}

fn handle_workspace_root(
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
    canonical_dir: PathBuf,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
) {
    if metadata.workspace_members.len() > 1 {
        discover_workspace_members(metadata, workspace_root, discovered_projects);
    } else {
        discovered_projects.insert(
            canonical_dir.clone(),
            DiscoveredProject {
                path:         canonical_dir,
                project_type: ProjectType::Standalone,
            },
        );
    }
}

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

fn add_fallback_standalone(
    dir: &Path,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
) {
    let canonical_dir = safe_canonicalize(dir);
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
) -> bool {
    if let Ok(metadata) = cargo_metadata::MetadataCommand::new()
        .current_dir(dir)
        .exec()
    {
        let workspace_root: PathBuf = metadata.workspace_root.clone().into();
        let canonical_dir = safe_canonicalize(dir);
        let canonical_workspace = safe_canonicalize(&workspace_root);
        let is_workspace_root = canonical_dir == canonical_workspace;

        if is_workspace_root {
            let is_multi_member_workspace = metadata.workspace_members.len() > 1;
            handle_workspace_root(
                &metadata,
                &workspace_root,
                canonical_dir,
                discovered_projects,
            );
            return is_multi_member_workspace;
        }
        add_workspace_member(dir, workspace_root, discovered_projects);
    } else {
        add_fallback_standalone(dir, discovered_projects);
    }
    false
}

fn shallow_scan(
    dir: &Path,
    visited_canonical: &mut HashSet<PathBuf>,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
) {
    shallow_scan_internal(
        dir,
        visited_canonical,
        discovered_projects,
        RootDirectorySkipPolicy::Bypass,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RootDirectorySkipPolicy {
    Apply,
    Bypass,
}

fn shallow_scan_internal(
    dir: &Path,
    visited_canonical: &mut HashSet<PathBuf>,
    discovered_projects: &mut HashMap<PathBuf, DiscoveredProject>,
    root_skip_policy: RootDirectorySkipPolicy,
) {
    let scan_start = Instant::now();
    debug!("shallow_scan_internal: {}", dir.display());

    let canonical = safe_canonicalize(dir);
    if !visited_canonical.insert(canonical) {
        debug!("  Already visited, skipping");
        return;
    }

    if root_skip_policy == RootDirectorySkipPolicy::Apply && should_skip_directory(dir) {
        debug!("  Skipping directory (hidden or target)");
        return;
    }

    let cargo_toml = dir.join("Cargo.toml");
    let skip_subdirs = if cargo_toml.exists() {
        debug!("  Found Cargo.toml at level 0");
        process_cargo_toml(dir, discovered_projects)
    } else {
        false
    };

    if skip_subdirs {
        debug!("  Skipping subdirectory scan - workspace members already discovered");
        debug!(
            "  shallow_scan_internal completed in {:?}",
            scan_start.elapsed()
        );
        return;
    }

    let read_dir_start = Instant::now();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let read_dir_elapsed = read_dir_start.elapsed();
        debug!("  read_dir took {:?}", read_dir_elapsed);

        let mut entry_count = 0;
        let mut skipped_count = 0;
        let mut found_count = 0;

        for entry in entries.flatten() {
            entry_count += 1;
            let path = entry.path();
            if path.is_dir() && !should_skip_directory(&path) {
                let sub_cargo_toml = path.join("Cargo.toml");
                if sub_cargo_toml.exists() {
                    found_count += 1;
                    debug!("  Found Cargo.toml in subdirectory: {}", path.display());
                    let sub_canonical = safe_canonicalize(&path);
                    if visited_canonical.insert(sub_canonical) {
                        process_cargo_toml(&path, discovered_projects);
                    }
                }
            } else {
                skipped_count += 1;
            }
        }

        debug!("  Processed {entry_count} entries ({skipped_count} skipped, {found_count} found)");
    }

    debug!(
        "  shallow_scan_internal completed in {:?}",
        scan_start.elapsed()
    );
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests should panic on unexpected values"
)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    fn write_binary_project(project_dir: &Path, name: &str, edition_line: &str) {
        fs::create_dir_all(project_dir.join("src")).expect("Failed to create src dir");
        fs::write(
            project_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n{edition_line}\n\n[[bin]]\nname = \"{name}\"\npath = \"src/main.rs\"\n"
            ),
        )
        .expect("Failed to write Cargo.toml");
        fs::write(
            project_dir.join("src/main.rs"),
            format!("fn main() {{\n    println!(\"{name}\");\n}}"),
        )
        .expect("Failed to write main.rs");
    }

    fn write_workspace_root(workspace_dir: &Path, members: &[&str]) {
        fs::create_dir_all(workspace_dir).expect("Failed to create workspace dir");
        let members = members
            .iter()
            .map(|member| format!("\"{member}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            workspace_dir.join("Cargo.toml"),
            format!(
                "[workspace]\nmembers = [{members}]\nresolver = \"2\"\n\n[workspace.package]\nedition = \"2021\"\n"
            ),
        )
        .expect("Failed to write workspace Cargo.toml");
    }

    #[test]
    fn test_safe_canonicalize_with_valid_path() {
        let path = Path::new(".");
        let result = safe_canonicalize(path);
        assert!(result.is_absolute());
    }

    #[test]
    fn test_safe_canonicalize_with_invalid_path() {
        let path = Path::new("/non/existent/path/that/does/not/exist");
        let result = safe_canonicalize(path);
        assert_eq!(result, path.to_path_buf());
    }

    #[test]
    fn test_should_skip_directory() {
        assert!(should_skip_directory(Path::new(".git")));
        assert!(should_skip_directory(Path::new(".cargo")));
        assert!(should_skip_directory(Path::new("target")));
        assert!(!should_skip_directory(Path::new("src")));
        assert!(!should_skip_directory(Path::new("tests")));
    }

    #[test]
    fn test_recursive_scan_with_hidden_directories() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        write_binary_project(
            &temp_path.join("test-project"),
            "test-project",
            "edition = \"2021\"",
        );

        fs::create_dir_all(temp_path.join("test-project/.git/objects"))
            .expect("Failed to create .git dir");
        fs::create_dir_all(temp_path.join("test-project/target/debug"))
            .expect("Failed to create target dir");

        write_binary_project(
            &temp_path.join("test-project/.hidden/hidden-project"),
            "hidden-project",
            "edition = \"2021\"",
        );

        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()]);

        assert_eq!(paths.len(), 1, "Should find exactly one project");
        assert!(
            paths
                .iter()
                .any(|path| path.to_string_lossy().contains("test-project"))
        );
        assert!(
            !paths
                .iter()
                .any(|path| path.to_string_lossy().contains("hidden-project"))
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_recursive_scan_cycle_detection() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        write_binary_project(
            &temp_path.join("project-a"),
            "project-a",
            "edition = \"2021\"",
        );
        write_binary_project(
            &temp_path.join("project-b"),
            "project-b",
            "edition = \"2021\"",
        );

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

        assert_eq!(paths.len(), 2, "Should find exactly two projects");
        assert!(
            paths
                .iter()
                .any(|path| path.to_string_lossy().contains("project-a"))
        );
        assert!(
            paths
                .iter()
                .any(|path| path.to_string_lossy().contains("project-b"))
        );
    }

    #[test]
    fn test_nested_workspace_discovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        let workspace_dir = temp_path.join("workspace");

        write_workspace_root(&workspace_dir, &["member-a", "member-b"]);
        write_binary_project(
            &workspace_dir.join("member-a"),
            "member-a",
            "edition.workspace = true",
        );
        write_binary_project(
            &workspace_dir.join("member-b"),
            "member-b",
            "edition.workspace = true",
        );

        let paths = iter_cargo_project_paths(&[temp_path.to_path_buf()]);

        assert_eq!(paths.len(), 1, "Should find exactly one workspace root");
        assert!(
            paths
                .iter()
                .any(|path| path.to_string_lossy().contains("workspace"))
        );
        assert!(
            !paths
                .iter()
                .any(|path| path.to_string_lossy().contains("member-a"))
        );
        assert!(
            !paths
                .iter()
                .any(|path| path.to_string_lossy().contains("member-b"))
        );
    }

    #[test]
    fn test_process_cargo_toml_workspace_detection() {
        let workspace_temp_dir = TempDir::new().expect("Failed to create workspace temp directory");
        let workspace_path = workspace_temp_dir.path();
        let workspace_dir = workspace_path.join("workspace");

        let standalone_temp_dir =
            TempDir::new().expect("Failed to create standalone temp directory");
        let standalone_path = standalone_temp_dir.path();

        write_workspace_root(&workspace_dir, &["app1", "app2"]);
        write_binary_project(
            &workspace_dir.join("app1"),
            "app1",
            "edition.workspace = true",
        );
        write_binary_project(
            &workspace_dir.join("app2"),
            "app2",
            "edition.workspace = true",
        );
        write_binary_project(
            &standalone_path.join("standalone"),
            "standalone",
            "edition = \"2021\"",
        );

        let mut discovered_projects = HashMap::new();
        process_cargo_toml(&workspace_dir, &mut discovered_projects);
        process_cargo_toml(
            &standalone_path.join("standalone"),
            &mut discovered_projects,
        );

        let workspace_members = discovered_projects
            .values()
            .filter(|project| matches!(project.project_type, ProjectType::Workspace { .. }))
            .count();
        let standalone_projects = discovered_projects
            .values()
            .filter(|project| matches!(project.project_type, ProjectType::Standalone))
            .count();

        assert!(
            discovered_projects.len() >= 2,
            "Should find at least workspace members and standalone project"
        );
        assert!(
            workspace_members >= 2,
            "Should find at least 2 workspace members"
        );
        assert!(
            standalone_projects >= 1,
            "Should find at least 1 standalone project"
        );
    }
}
