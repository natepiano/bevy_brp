use std::path::Path;
use std::path::PathBuf;

use super::project_discovery;
use crate::app_tools::targets::cargo_detector::BevyTarget;

/// Compute the relative path from the search roots to the given path.
///
/// This is used to provide a stable identifier for disambiguation.
pub fn compute_relative_path(path: &Path, search_paths: &[PathBuf]) -> PathBuf {
    for search_path in search_paths {
        let search_canonical = project_discovery::safe_canonicalize(search_path);
        let path_canonical = project_discovery::safe_canonicalize(path);
        if let Ok(relative) = path_canonical.strip_prefix(&search_canonical) {
            if relative.as_os_str().is_empty() {
                if let Some(name) = path_canonical.file_name() {
                    return PathBuf::from(name);
                }
                return PathBuf::from(".");
            }
            return relative.to_path_buf();
        }
    }

    path.to_path_buf()
}

/// Filter targets to only those whose package directory is under the given path scope.
///
/// When a user explicitly provides a `path` search root, workspace resolution
/// via `cargo metadata` may expand the search to the full workspace. This
/// post-filter restricts results to targets whose manifest directory is
/// actually under the user-specified path.
pub fn filter_targets_by_path_scope(targets: Vec<BevyTarget>, scope: &Path) -> Vec<BevyTarget> {
    let canonical_scope = project_discovery::safe_canonicalize(scope);
    targets
        .into_iter()
        .filter(|target| {
            let manifest_dir = target
                .manifest_path
                .parent()
                .unwrap_or(&target.manifest_path);
            let canonical_manifest = project_discovery::safe_canonicalize(manifest_dir);
            canonical_manifest.starts_with(&canonical_scope)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;
    use crate::app_tools::targets::cargo_detector::TargetType;

    fn make_target(name: &str, package_name: &str, manifest_path: &str) -> BevyTarget {
        BevyTarget {
            name:           name.to_string(),
            target_type:    TargetType::Example,
            package_name:   package_name.to_string(),
            workspace_root: PathBuf::from("/workspace"),
            manifest_path:  PathBuf::from(manifest_path),
            relative_path:  PathBuf::new(),
            source_path:    PathBuf::new(),
        }
    }

    #[test]
    fn test_find_and_filter_by_path_exact() {
        let search_paths = vec![PathBuf::from("/home/user/projects")];
        let path = PathBuf::from("/home/user/projects/workspace1/app1");
        let relative = compute_relative_path(&path, &search_paths);
        assert_eq!(relative, PathBuf::from("workspace1/app1"));
    }

    #[test]
    fn test_find_and_filter_by_path_suffix() {
        let search_paths = vec![PathBuf::from("/home/user/projects")];
        let path1 = PathBuf::from("/home/user/projects/workspace1/app1");
        let path2 = PathBuf::from("/home/user/projects/workspace2/app1");

        let rel1 = compute_relative_path(&path1, &search_paths);
        let rel2 = compute_relative_path(&path2, &search_paths);

        assert_eq!(rel1, PathBuf::from("workspace1/app1"));
        assert_eq!(rel2, PathBuf::from("workspace2/app1"));
    }

    #[test]
    fn test_compute_relative_path() {
        let search_paths = vec![
            PathBuf::from("/home/user/projects"),
            PathBuf::from("/home/user/work"),
        ];

        let path = PathBuf::from("/home/user/projects/my-app");
        let relative = compute_relative_path(&path, &search_paths);
        assert_eq!(relative, PathBuf::from("my-app"));

        let path = PathBuf::from("/home/user/other/my-app");
        let relative = compute_relative_path(&path, &search_paths);
        assert_eq!(relative, PathBuf::from("/home/user/other/my-app"));
    }

    #[test]
    fn test_filter_targets_by_path_scope_includes_targets_under_scope() {
        let targets = vec![
            make_target("app_a", "pkg-a", "/workspace/sub-a/Cargo.toml"),
            make_target("app_b", "pkg-b", "/workspace/sub-b/Cargo.toml"),
        ];

        let filtered = filter_targets_by_path_scope(targets, Path::new("/workspace/sub-a"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "app_a");
    }

    #[test]
    fn test_filter_targets_by_path_scope_excludes_targets_outside_scope() {
        let targets = vec![
            make_target("app_a", "pkg-a", "/workspace/sub-a/Cargo.toml"),
            make_target("app_b", "pkg-b", "/workspace/sub-b/Cargo.toml"),
        ];

        let filtered = filter_targets_by_path_scope(targets, Path::new("/workspace/sub-c"));

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_targets_by_path_scope_includes_nested_targets() {
        let targets = vec![
            make_target("app_a", "pkg-a", "/workspace/group/sub-a/Cargo.toml"),
            make_target("app_b", "pkg-b", "/workspace/group/sub-b/Cargo.toml"),
            make_target("app_c", "pkg-c", "/workspace/other/Cargo.toml"),
        ];

        let filtered = filter_targets_by_path_scope(targets, Path::new("/workspace/group"));

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|target| target.name == "app_a"));
        assert!(filtered.iter().any(|target| target.name == "app_b"));
    }

    #[test]
    fn test_filter_targets_by_path_scope_workspace_root_includes_all() {
        let targets = vec![
            make_target("app_a", "pkg-a", "/workspace/sub-a/Cargo.toml"),
            make_target("app_b", "pkg-b", "/workspace/sub-b/Cargo.toml"),
        ];

        let filtered = filter_targets_by_path_scope(targets, Path::new("/workspace"));

        assert_eq!(filtered.len(), 2);
    }
}
