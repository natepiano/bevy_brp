//! Debug mode control for `bevy_brp_extras`
//!
//! This module provides a global debug mode flag that controls whether
//! detailed debug information is included in BRP responses.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;
use bevy::remote::{BrpError, BrpResult, error_codes};
use serde_json::{Value, json};

/// Global debug mode flag
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if debug mode is currently enabled
pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

/// Handler for the `set_debug_mode` BRP method
pub fn handler(In(params): In<Option<Value>>, _world: &mut World) -> BrpResult {
    // Parse the enabled parameter
    let enabled = if let Some(params) = params {
        if let Some(enabled_value) = params.get("enabled") {
            enabled_value.as_bool().ok_or_else(|| BrpError {
                code:    error_codes::INVALID_PARAMS,
                message: "Parameter 'enabled' must be a boolean".to_string(),
                data:    None,
            })?
        } else {
            return Err(BrpError {
                code:    error_codes::INVALID_PARAMS,
                message: "Missing required 'enabled' parameter".to_string(),
                data:    None,
            });
        }
    } else {
        return Err(BrpError {
            code:    error_codes::INVALID_PARAMS,
            message: "Missing required 'enabled' parameter".to_string(),
            data:    None,
        });
    };

    // Update the debug state
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);

    let message = if enabled {
        "Debug mode enabled - detailed discovery information will be included in responses"
    } else {
        "Debug mode disabled - detailed discovery information will be excluded from responses"
    };

    Ok(json!({
        "success": true,
        "debug_enabled": enabled,
        "message": message
    }))
}
