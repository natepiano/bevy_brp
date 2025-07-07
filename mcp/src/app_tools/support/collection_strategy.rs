//! Collection strategy trait for app listing handlers

use serde_json::json;

use super::cargo_detector::{BinaryInfo, CargoDetector, ExampleInfo};
use crate::app_tools::constants::{PROFILE_DEBUG, PROFILE_RELEASE};

/// Helper function to create builds JSON for binary items
fn create_builds_json(item: &BinaryInfo) -> serde_json::Value {
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

/// Strategy trait for collecting and serializing different types of items
pub trait CollectionStrategy {
    type Item;

    /// Collect items from the detector
    fn collect_items(&self, detector: &CargoDetector) -> Vec<Self::Item>;

    /// Get the type name for messages
    fn get_type_name(&self) -> &'static str;

    /// Get the data field name for JSON response
    fn get_data_field_name(&self) -> &'static str;

    /// Create a unique key for deduplication
    fn create_unique_key(&self, item: &Self::Item) -> String;

    /// Serialize an item to JSON with relative path
    fn serialize_item(&self, item: &Self::Item, relative_path: String) -> serde_json::Value;
}

/// Strategy for collecting standard Bevy apps with build info
pub struct BevyAppsStrategy;

impl CollectionStrategy for BevyAppsStrategy {
    type Item = BinaryInfo;

    fn collect_items(&self, detector: &CargoDetector) -> Vec<Self::Item> {
        detector.find_bevy_apps()
    }

    fn get_type_name(&self) -> &'static str {
        "Bevy apps"
    }

    fn get_data_field_name(&self) -> &'static str {
        "apps"
    }

    fn create_unique_key(&self, item: &Self::Item) -> String {
        format!("{}::{}", item.workspace_root.display(), item.name)
    }

    fn serialize_item(&self, item: &Self::Item, relative_path: String) -> serde_json::Value {
        json!({
            "name": item.name,
            "workspace_root": item.workspace_root.display().to_string(),
            "manifest_path": item.manifest_path.display().to_string(),
            // The relative_path field is designed for round-trip compatibility with launch functions.
            // This path can be used directly in brp_launch_bevy_app's path parameter
            // to disambiguate between apps with the same name in different locations.
            "relative_path": relative_path,
            "builds": create_builds_json(item)
        })
    }
}

/// Strategy for collecting BRP-enabled apps with `brp_enabled` flag
pub struct BrpAppsStrategy;

impl CollectionStrategy for BrpAppsStrategy {
    type Item = BinaryInfo;

    fn collect_items(&self, detector: &CargoDetector) -> Vec<Self::Item> {
        detector.find_brp_enabled_apps()
    }

    fn get_type_name(&self) -> &'static str {
        "BRP-enabled Bevy apps"
    }

    fn get_data_field_name(&self) -> &'static str {
        "apps"
    }

    fn create_unique_key(&self, item: &Self::Item) -> String {
        format!("{}::{}", item.workspace_root.display(), item.name)
    }

    fn serialize_item(&self, item: &Self::Item, _relative_path: String) -> serde_json::Value {
        json!({
            "name": item.name,
            "workspace_root": item.workspace_root.display().to_string(),
            "manifest_path": item.manifest_path.display().to_string(),
            "builds": create_builds_json(item),
            "brp_enabled": true
        })
    }
}

/// Strategy for collecting examples without build info
pub struct BevyExamplesStrategy;

impl CollectionStrategy for BevyExamplesStrategy {
    type Item = ExampleInfo;

    fn collect_items(&self, detector: &CargoDetector) -> Vec<Self::Item> {
        detector.find_bevy_examples()
    }

    fn get_type_name(&self) -> &'static str {
        "Bevy examples"
    }

    fn get_data_field_name(&self) -> &'static str {
        "examples"
    }

    fn create_unique_key(&self, item: &Self::Item) -> String {
        format!("{}::{}", item.package_name, item.name)
    }

    fn serialize_item(&self, item: &Self::Item, relative_path: String) -> serde_json::Value {
        json!({
            "name": item.name,
            "package_name": item.package_name,
            "manifest_path": item.manifest_path.display().to_string(),
            // The relative_path field is designed for round-trip compatibility with launch functions.
            // This path can be used directly in brp_launch_bevy_example's path parameter
            // to disambiguate between examples with the same name in different locations.
            "relative_path": relative_path
        })
    }
}
