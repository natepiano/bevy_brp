//! Generic listing handler using the strategy pattern

use std::collections::HashSet;

use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::cargo_detector::CargoDetector;
use super::collection_strategy::CollectionStrategy;
use super::scanning;
use crate::BrpMcpService;
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::service;

/// Generic handler for listing items using a collection strategy
pub async fn handle_listing<S: CollectionStrategy>(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
    strategy: S,
) -> Result<CallToolResult, McpError> {
    service::handle_with_paths(service, context, |search_paths| async move {
        let items = collect_all_items(&search_paths, &strategy);

        let response = ResponseBuilder::success()
            .message(format!(
                "Found {} {}",
                items.len(),
                strategy.get_type_name()
            ))
            .data(json!({
                strategy.get_data_field_name(): items
            }))
            .map_or_else(
                |_| {
                    ResponseBuilder::error()
                        .message("Failed to serialize response data")
                        .build()
                },
                ResponseBuilder::build,
            );

        Ok(json_response_to_result(&response))
    })
    .await
}

/// Collect all items using the provided strategy
fn collect_all_items<S: CollectionStrategy>(
    search_paths: &[std::path::PathBuf],
    strategy: &S,
) -> Vec<serde_json::Value> {
    let mut all_items = Vec::new();
    let mut seen_items = HashSet::new();
    let mut debug_info = Vec::new();

    // Use the iterator to find all cargo projects
    for path in scanning::iter_cargo_project_paths(search_paths, &mut debug_info) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let items = strategy.collect_items(&detector);
            for item in items {
                // Create a unique key using the strategy
                let key = strategy.create_unique_key(&item);
                if seen_items.insert(key) {
                    // Compute relative path for the project
                    let relative_path =
                        scanning::compute_relative_path(&path, search_paths, &mut debug_info);

                    let serialized_item =
                        strategy.serialize_item(&item, relative_path.display().to_string());
                    all_items.push(serialized_item);
                }
            }
        }
    }

    all_items
}
