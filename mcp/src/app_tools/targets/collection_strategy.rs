//! Collection strategy trait for app listing handlers

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

use super::cargo_detector::BevyTarget;
use super::cargo_detector::BrpLevel;
use super::cargo_detector::CargoDetector;
use crate::app_tools::constants::PROFILE_DEBUG;
use crate::app_tools::constants::PROFILE_RELEASE;

/// Helper function to create builds JSON for binary items
fn create_builds_json(item: &BevyTarget) -> Value {
    let profiles = vec![PROFILE_DEBUG, PROFILE_RELEASE];
    let mut builds = json!({});
    for profile in &profiles {
        let binary_path = item.get_binary_path(profile);
        builds[profile] = json!({
            "path": binary_path.display().to_string(),
            "built": binary_path.exists()
        });
    }
    builds
}

/// Unified strategy for collecting all Bevy targets (apps and examples)
/// with `kind` and `brp_level` fields on each item.
///
/// Only targets declared in workspace `Cargo.toml` files are included (via `cargo metadata`).
pub(super) struct AllBevyTargetsStrategy;

/// A `BevyTarget` enriched with BRP status.
///
/// For bins, `brp_level` reflects whether the package's `src/` tree uses BRP plugins.
/// For examples, the individual source file is checked for BRP plugin imports.
pub(super) struct EnrichedTarget {
    pub(super) target:    BevyTarget,
    pub(super) brp_level: BrpLevel,
}

impl AllBevyTargetsStrategy {
    pub(super) fn collect_items(detector: &CargoDetector) -> Vec<EnrichedTarget> {
        let all_targets = detector.find_bevy_targets();

        // Build a package-level BRP lookup for bins (requires deep `src/` scan)
        let brp_targets = detector.find_brp_targets();
        let brp_keys: HashSet<String> = brp_targets
            .iter()
            .map(|t| format!("{}::{}", t.manifest_path.display(), t.name))
            .collect();

        // Enrich each target with BRP level using a hybrid approach:
        // - Bins: package-level set lookup (scans `src/` tree for BRP plugin registration) then
        //   file-level check for extras vs brp_only distinction
        // - Examples: per-file check (reads the example's source file directly)
        all_targets
            .into_iter()
            .map(|target| {
                let brp_level = if target.is_app() {
                    let key = format!("{}::{}", target.manifest_path.display(), target.name);
                    if brp_keys.contains(&key) {
                        // Package has BRP — check the specific binary's source for level
                        CargoDetector::file_brp_level(&target.source_path)
                    } else {
                        BrpLevel::None
                    }
                } else {
                    CargoDetector::file_brp_level(&target.source_path)
                };
                EnrichedTarget { target, brp_level }
            })
            .collect()
    }

    pub(super) fn create_unique_key(item: &EnrichedTarget) -> String {
        format!(
            "{}::{}::{}",
            item.target.manifest_path.display(),
            item.target.name,
            item.target.target_type.as_ref()
        )
    }

    pub(super) fn get_path_for_relative(item: &EnrichedTarget) -> PathBuf {
        item.target
            .manifest_path
            .parent()
            .unwrap_or(&item.target.manifest_path)
            .to_path_buf()
    }

    pub(super) fn serialize_item(item: &EnrichedTarget, relative_path: String) -> Value {
        json!({
            "name": item.target.name,
            "kind": item.target.target_type.as_ref(),
            "package_name": item.target.package_name,
            "brp_level": item.brp_level.as_str(),
            "workspace_root": item.target.workspace_root.display().to_string(),
            "manifest_path": item.target.manifest_path.display().to_string(),
            // The relative_path field is designed for round-trip compatibility with launch functions.
            // This path can be used directly in `brp_launch`'s path parameter
            // to disambiguate between targets with the same name in different locations.
            "relative_path": relative_path,
            "builds": create_builds_json(&item.target)
        })
    }
}
