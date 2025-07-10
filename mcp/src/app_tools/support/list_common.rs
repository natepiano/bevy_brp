//! Generic listing handler using the strategy pattern

use std::collections::HashSet;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::cargo_detector::CargoDetector;
use super::collection_strategy::CollectionStrategy;
use super::scanning;
use crate::service;
use crate::service::McpService;

/// Typed handler wrapper for binary listing operations that fetches search paths
pub async fn handle_list_binaries<F, Fut, T>(
    service: Arc<McpService>,
    context: RequestContext<RoleServer>,
    handler: F,
) -> Result<T, McpError>
where
    F: FnOnce(Vec<PathBuf>) -> Fut,
    Fut: Future<Output = Result<T, McpError>>,
{
    let search_paths = service::fetch_roots_and_get_paths(service, context).await?;
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
