//! Mouse scroll wheel events

use bevy::ecs::system::In;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_remote::BrpResult;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::support;
use crate::window_event;

// ============================================================================
// Types
// ============================================================================

/// Request structure for `scroll_mouse`
#[derive(Deserialize)]
struct ScrollMouseRequest {
    /// Horizontal scroll amount
    pub x:      f32,
    /// Vertical scroll amount
    pub y:      f32,
    /// Scroll unit
    pub unit:   MouseScrollUnit,
    /// Target window entity (None = primary window)
    #[serde(default)]
    pub window: Option<u64>,
}

/// Response structure for `scroll_mouse`
#[derive(Serialize)]
struct ScrollMouseResponse {
    /// Horizontal scroll amount
    pub x:    f32,
    /// Vertical scroll amount
    pub y:    f32,
    /// Scroll unit that was used
    pub unit: MouseScrollUnit,
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `scroll_mouse` BRP method
pub fn scroll_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: ScrollMouseRequest =
        support::parse_request(params, support::EmptyParamsPolicy::Reject)?;
    let window = support::resolve_window(world, request.window)?;

    window_event::write_input_event(
        world,
        MouseWheel {
            unit: request.unit,
            x: request.x,
            y: request.y,
            window,
        },
    );

    support::serialize_response(
        ScrollMouseResponse {
            x:    request.x,
            y:    request.y,
            unit: request.unit,
        },
        "scroll_mouse",
    )
}
