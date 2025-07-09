// Local support modules for app_tools

pub mod cargo_detector;
pub mod collection_strategy;
pub mod generic_listing_handler;
pub mod launch_common;
pub mod logging;
pub mod process;
pub mod scanning;

use serde_json::json;

/// Creates a minimal disambiguation error response for launch handlers
///
/// This is used when multiple apps or examples with the same name are found
/// and the user needs to specify which path to use.
///
/// # Arguments
/// * `item_type` - Either "app" or "example"
/// * `item_name` - The name of the app or example
/// * `duplicate_paths` - List of paths where the item was found
pub fn create_disambiguation_error(
    item_type: &str,
    item_name: &str,
    duplicate_paths: Vec<String>,
) -> serde_json::Value {
    let field_name = format!("{item_type}_name");
    json!({
        "status": "error",
        "message": format!("Found multiple {item_type}s named '{item_name}'. Please specify which path to use."),
        field_name: item_name,
        "duplicate_paths": duplicate_paths,
    })
}
