//! Listing handler using the strategy pattern

use std::collections::HashSet;

use super::cargo_detector::CargoDetector;
use super::collection_strategy::AllBevyTargetsStrategy;
use super::scanning;

/// Collect all Bevy targets (apps and examples) with `kind` and `brp_enabled` fields
pub fn collect_all_bevy_targets(search_paths: &[std::path::PathBuf]) -> Vec<serde_json::Value> {
    let mut all_items = Vec::new();
    let mut seen_items = HashSet::new();

    // Use the iterator to find all cargo projects
    for path in scanning::iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::try_from(path.as_path()) {
            let items = AllBevyTargetsStrategy::collect_items(&detector);
            for item in items {
                let key = AllBevyTargetsStrategy::create_unique_key(&item);
                if seen_items.insert(key) {
                    let item_path = AllBevyTargetsStrategy::get_path_for_relative(&item);
                    let relative_path = scanning::compute_relative_path(&item_path, search_paths);

                    let serialized_item = AllBevyTargetsStrategy::serialize_item(
                        &item,
                        relative_path.display().to_string(),
                    );
                    all_items.push(serialized_item);
                }
            }
        }
    }

    all_items
}
