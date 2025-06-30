//! Simple cargo detector based on `bevy_brp_tool`

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};

/// Information about a binary target
#[derive(Debug, Clone)]
pub struct BinaryInfo {
    /// Name of the binary
    pub name:           String,
    /// Workspace root
    pub workspace_root: PathBuf,
    /// Path to the package's Cargo.toml
    pub manifest_path:  PathBuf,
    /// Relative path from scan root to this item
    pub relative_path:  PathBuf,
}

impl BinaryInfo {
    /// Get the path to the binary for a given profile
    pub fn get_binary_path(&self, profile: &str) -> PathBuf {
        self.workspace_root
            .join("target")
            .join(profile)
            .join(&self.name)
    }
}

/// Information about an example
#[derive(Debug, Clone)]
pub struct ExampleInfo {
    /// Name of the example
    pub name:          String,
    /// Package name
    pub package_name:  String,
    /// Path to the package's Cargo.toml
    pub manifest_path: PathBuf,
    /// Relative path from scan root to this item
    pub relative_path: PathBuf,
}

/// Detects binary targets in a project or workspace
pub struct CargoDetector {
    metadata: Metadata,
}

impl CargoDetector {
    /// Create a detector for a specific path
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let metadata = MetadataCommand::new()
            .current_dir(path.as_ref())
            .exec()
            .context("Failed to execute cargo metadata")?;

        Ok(Self { metadata })
    }

    /// Check if a package is a workspace member
    fn is_workspace_member(&self, package: &Package) -> bool {
        self.metadata.workspace_members.contains(&package.id)
    }

    /// Find packages that match the given filter criteria
    fn find_packages_with_filter<'a, F>(&'a self, filter: F) -> impl Iterator<Item = &'a Package>
    where
        F: Fn(&Package) -> bool + 'a,
    {
        self.metadata
            .packages
            .iter()
            .filter(|p| self.is_workspace_member(p))
            .filter(move |p| filter(p))
    }

    /// Extract binary targets from a package
    fn extract_binary_targets<'a>(
        &'a self,
        package: &'a Package,
    ) -> impl Iterator<Item = BinaryInfo> + 'a {
        package
            .targets
            .iter()
            .filter(|t| t.is_bin())
            .map(move |t| BinaryInfo {
                name:           t.name.clone(),
                workspace_root: self.metadata.workspace_root.clone().into(),
                manifest_path:  package.manifest_path.clone().into(),
                relative_path:  PathBuf::new(), // Will be set by scanning logic
            })
    }

    /// Extract example targets from a package
    fn extract_example_targets(package: &Package) -> impl Iterator<Item = ExampleInfo> + '_ {
        package
            .targets
            .iter()
            .filter(|t| t.is_example())
            .map(move |t| ExampleInfo {
                name:          t.name.clone(),
                package_name:  package.name.to_string(),
                manifest_path: package.manifest_path.clone().into(),
                relative_path: PathBuf::new(), // Will be set by scanning logic
            })
    }

    /// Filter for packages that depend on Bevy (excluding `bevy_brp_mcp` itself)
    fn bevy_app_filter(package: &Package) -> bool {
        package.name.as_str() != "bevy_brp_mcp" && Self::package_depends_on_bevy(package)
    }

    /// Filter for packages that have BRP support and are not `bevy_brp_mcp` itself
    fn brp_app_filter(package: &Package) -> bool {
        package.name.as_str() != "bevy_brp_mcp" && Self::package_has_brp_support(package)
    }

    /// Find all Bevy applications (binaries) in the workspace/project
    pub fn find_bevy_apps(&self) -> Vec<BinaryInfo> {
        self.find_packages_with_filter(Self::bevy_app_filter)
            .flat_map(|p| self.extract_binary_targets(p))
            .collect()
    }

    /// Find all Bevy examples in the workspace/project
    pub fn find_bevy_examples(&self) -> Vec<ExampleInfo> {
        self.find_packages_with_filter(Self::bevy_app_filter)
            .flat_map(Self::extract_example_targets)
            .collect()
    }

    /// Find all BRP-enabled Bevy applications (binaries) in the workspace/project
    pub fn find_brp_enabled_apps(&self) -> Vec<BinaryInfo> {
        self.find_packages_with_filter(Self::brp_app_filter)
            .flat_map(|p| self.extract_binary_targets(p))
            .collect()
    }

    fn package_depends_on_bevy(package: &Package) -> bool {
        // Check direct dependencies (including workspace dependencies)
        package.dependencies.iter().any(|dep| dep.name == "bevy")
    }

    /// Check if a package has BRP (Bevy Remote Protocol) support enabled
    fn package_has_brp_support(package: &Package) -> bool {
        // First check: Must have bevy dependency with bevy_remote feature available
        if !Self::package_has_bevy_remote_feature(package) {
            return false;
        }

        // Second check: Must actually use BRP plugins in source code
        Self::package_uses_brp_plugins(package)
    }

    /// Check if a package has `bevy_remote` feature available (either explicit or workspace
    /// inherited)
    fn package_has_bevy_remote_feature(package: &Package) -> bool {
        // Check if bevy dependency includes bevy_remote feature or uses workspace inheritance
        package.dependencies.iter().any(|dep| {
            if dep.name == "bevy" {
                // If it has explicit features, check for bevy_remote
                if dep.features.is_empty() {
                    // If no explicit features, assume workspace inheritance
                    // (we'll verify actual usage in the code scanning step)
                    true
                } else {
                    dep.features.iter().any(|feature| feature == "bevy_remote")
                }
            } else {
                false
            }
        })
    }

    /// Check if a package uses `RemotePlugin` or `BrpExtrasPlugin` in its source code
    fn package_uses_brp_plugins(package: &Package) -> bool {
        // Get the package directory
        let Some(package_dir) = package.manifest_path.parent() else {
            return false;
        };

        // Check all .rs files in src/ directory
        let src_dir = package_dir.join("src");
        if !src_dir.exists() {
            return false;
        }

        Self::check_directory_for_brp_plugins(src_dir.as_std_path())
    }

    /// Recursively check directory for BRP plugin usage
    fn check_directory_for_brp_plugins(dir: &std::path::Path) -> bool {
        use std::fs;

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    if Self::check_directory_for_brp_plugins(&path) {
                        return true;
                    }
                } else if path.extension().is_some_and(|ext| ext == "rs")
                    && Self::file_uses_brp_plugins(&path)
                {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a specific file uses `RemotePlugin` or `BrpExtrasPlugin`
    fn file_uses_brp_plugins(file_path: &std::path::Path) -> bool {
        use std::fs;

        fs::read_to_string(file_path).is_ok_and(|content| {
            // Look for actual use statements that import BRP plugins
            // Use more specific patterns to avoid matching our own detection code
            let has_remote_plugin_import = content.contains("use bevy::remote::RemotePlugin")
                || (content.contains("use bevy::remote::{") && content.contains("RemotePlugin"));

            let has_brp_extras_plugin_import = content
                .contains("use bevy_brp_extras::BrpExtrasPlugin")
                || (content.contains("use bevy_brp_extras::{")
                    && content.contains("BrpExtrasPlugin"));

            has_remote_plugin_import || has_brp_extras_plugin_import
        })
    }
}
