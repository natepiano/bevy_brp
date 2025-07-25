//! Generic listing handler using the strategy pattern

use std::collections::HashSet;
use std::future::Future;
use std::path::PathBuf;

use rmcp::ErrorData as McpError;

use super::cargo_detector::CargoDetector;
use super::collection_strategy::CollectionStrategy;
use super::scanning;
use crate::tool::HandlerContext;

/// Typed handler wrapper for binary listing operations that fetches search paths
pub async fn handle_list_binaries<F, Fut, T>(
    handler_context: HandlerContext,
    handler: F,
) -> Result<T, McpError>
where
    F: FnOnce(Vec<PathBuf>) -> Fut + Send,
    Fut: Future<Output = Result<T, McpError>> + Send,
{
    let search_paths = handler_context.roots;
    handler(search_paths).await
}

/// Collect all items using the provided strategy
pub fn collect_all_items<S: CollectionStrategy>(
    search_paths: &[std::path::PathBuf],
    strategy: &S,
) -> Vec<serde_json::Value> {
    let mut all_items = Vec::new();
    let mut seen_items = HashSet::new();

    // Use the iterator to find all cargo projects
    for path in scanning::iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let items = strategy.collect_items(&detector);
            for item in items {
                // Create a unique key using the strategy
                let key = strategy.create_unique_key(&item);
                if seen_items.insert(key) {
                    // Compute relative path for the project
                    let relative_path = scanning::compute_relative_path(&path, search_paths);

                    let serialized_item =
                        strategy.serialize_item(&item, relative_path.display().to_string());
                    all_items.push(serialized_item);
                }
            }
        }
    }

    all_items
}
