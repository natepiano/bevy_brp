//! Window title handler for BRP extras

use bevy::prelude::*;
use bevy::remote::error_codes::{INTERNAL_ERROR, INVALID_PARAMS};
use bevy::remote::{BrpError, BrpResult};
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

/// Handler for `set_window_title` requests
pub fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Extract title from params
    let title = params
        .as_ref()
        .and_then(|p| p.get("title"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| BrpError {
            code:    INVALID_PARAMS,
            message: "Missing or invalid 'title' parameter".to_string(),
            data:    None,
        })?;

    // Query for primary window
    let mut query = world.query_filtered::<&mut Window, With<PrimaryWindow>>();

    // Get mutable window reference
    let mut window = query.single_mut(world).map_err(|_| BrpError {
        code:    INTERNAL_ERROR,
        message: "No primary window found".to_string(),
        data:    None,
    })?;

    // Store old title for response
    let old_title = window.title.clone();

    // Set new title
    window.title = title.to_string();

    Ok(json!({
        "status": "success",
        "old_title": old_title,
        "new_title": title,
        "message": format!("Window title changed from '{old_title}' to '{title}'")
    }))
}
