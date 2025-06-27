use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::cargo_detector::CargoDetector;
use super::support::scanning;
use crate::BrpMcpService;
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::service;

pub async fn handle(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    service::handle_with_paths(service, context, |search_paths| async move {
        let examples = collect_all_examples(&search_paths);

        let response = ResponseBuilder::success()
            .message(format!("Found {} Bevy examples", examples.len()))
            .data(json!({
                "examples": examples
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

fn collect_all_examples(search_paths: &[std::path::PathBuf]) -> Vec<serde_json::Value> {
    let mut all_examples = Vec::new();

    // Use the iterator to find all cargo projects
    for path in scanning::iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let examples = detector.find_bevy_examples();
            for example in examples {
                all_examples.push(json!({
                    "name": example.name,
                    "package_name": example.package_name,
                    "manifest_path": example.manifest_path.display().to_string()
                }));
            }
        }
    }

    all_examples
}
