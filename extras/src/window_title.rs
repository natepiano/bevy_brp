//! Window title handler for BRP extras

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde_json::Value;
use serde_json::json;

use crate::constants::PARAM_TITLE;
use crate::constants::RESPONSE_MESSAGE_FIELD;
use crate::constants::RESPONSE_NEW_TITLE_FIELD;
use crate::constants::RESPONSE_OLD_TITLE_FIELD;
use crate::constants::RESPONSE_STATUS_FIELD;
use crate::constants::RESPONSE_STATUS_SUCCESS;

/// Handler for `set_window_title` requests
pub(crate) fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Extract title from params
    let title = params
        .as_ref()
        .and_then(|parameters| parameters.get(PARAM_TITLE))
        .and_then(Value::as_str)
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
    window.title = String::from(title);

    Ok(json!({
        RESPONSE_STATUS_FIELD: RESPONSE_STATUS_SUCCESS,
        RESPONSE_OLD_TITLE_FIELD: old_title,
        RESPONSE_NEW_TITLE_FIELD: title,
        RESPONSE_MESSAGE_FIELD: format!("Window title changed from '{old_title}' to '{title}'")
    }))
}
