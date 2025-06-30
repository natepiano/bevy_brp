use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::json;

use super::support::cargo_detector::CargoDetector;
use super::support::scanning;
use crate::BrpMcpService;
use crate::constants::{PROFILE_DEBUG, PROFILE_RELEASE};
use crate::support::response::ResponseBuilder;
use crate::support::serialization::json_response_to_result;
use crate::support::service;

pub async fn handle(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    service::handle_with_paths(service, context, |search_paths| async move {
        let apps = collect_all_apps(&search_paths);

        let response = ResponseBuilder::success()
            .message(format!("Found {} Bevy apps", apps.len()))
            .data(json!({
                "apps": apps
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

fn collect_all_apps(search_paths: &[std::path::PathBuf]) -> Vec<serde_json::Value> {
    let mut all_apps = Vec::new();
    let mut seen_apps = std::collections::HashSet::new();
    let profiles = vec![PROFILE_DEBUG, PROFILE_RELEASE];
    let mut debug_info = Vec::new();

    // Use the iterator to find all cargo projects
    for path in scanning::iter_cargo_project_paths(search_paths, &mut debug_info) {
        if let Ok(detector) = CargoDetector::from_path(&path) {
            let apps = detector.find_bevy_apps();
            for app in apps {
                // Create a unique key based on app name and workspace root
                let key = format!("{}::{}", app.workspace_root.display(), app.name);
                if seen_apps.insert(key) {
                    let mut builds = json!({});
                    for profile in &profiles {
                        let binary_path = app.get_binary_path(profile);
                        builds[profile] = json!({
                            "path": binary_path.display().to_string(),
                            "built": binary_path.exists()
                        });
                    }

                    // Compute relative path for the project
                    let relative_path =
                        scanning::compute_relative_path(&path, search_paths, &mut debug_info);

                    all_apps.push(json!({
                        "name": app.name,
                        "workspace_root": app.workspace_root.display().to_string(),
                        "manifest_path": app.manifest_path.display().to_string(),
                        // The relative_path field is designed for round-trip compatibility with launch functions.
                        // This path can be used directly in brp_launch_bevy_app's path parameter
                        // to disambiguate between apps with the same name in different locations.
                        "relative_path": relative_path.display().to_string(),
                        "builds": builds
                    }));
                }
            }
        }
    }

    all_apps
}
