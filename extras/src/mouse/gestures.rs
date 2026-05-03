//! Trackpad gesture events (pinch, rotation, double tap)

use bevy::ecs::system::In;
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

/// Request structure for `pinch_gesture`
#[derive(Deserialize)]
struct PinchGestureRequest {
    /// Pinch delta (positive = zoom in, negative = zoom out)
    pub delta: f32,
}

/// Response structure for `pinch_gesture`
#[derive(Serialize)]
struct PinchGestureResponse {
    /// Pinch delta that was applied
    pub delta: f32,
}

/// Request structure for `rotation_gesture`
#[derive(Deserialize)]
struct RotationGestureRequest {
    /// Rotation delta in radians
    pub delta: f32,
}

/// Response structure for `rotation_gesture`
#[derive(Serialize)]
struct RotationGestureResponse {
    /// Rotation delta that was applied
    pub delta: f32,
}

/// Request structure for `double_tap_gesture`
#[derive(Deserialize)]
struct DoubleTapGestureRequest {
    // No parameters needed
}

/// Response structure for `double_tap_gesture`
#[derive(Serialize)]
struct DoubleTapGestureResponse {
    // No fields needed - success is indicated by Ok result
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `pinch_gesture` BRP method
pub fn pinch_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: PinchGestureRequest =
        support::parse_request(params, support::EmptyParamsPolicy::Reject)?;

    window_event::write_input_event(world, bevy::input::gestures::PinchGesture(request.delta));

    support::serialize_response(
        PinchGestureResponse {
            delta: request.delta,
        },
        "pinch_gesture",
    )
}

/// Handler for `rotation_gesture` BRP method
pub fn rotation_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: RotationGestureRequest =
        support::parse_request(params, support::EmptyParamsPolicy::Reject)?;

    window_event::write_input_event(world, bevy::input::gestures::RotationGesture(request.delta));

    support::serialize_response(
        RotationGestureResponse {
            delta: request.delta,
        },
        "rotation_gesture",
    )
}

/// Handler for `double_tap_gesture` BRP method
pub fn double_tap_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let _: DoubleTapGestureRequest =
        support::parse_request(params, support::EmptyParamsPolicy::Allow)?;

    window_event::write_input_event(world, bevy::input::gestures::DoubleTapGesture);

    support::serialize_response(DoubleTapGestureResponse {}, "double_tap_gesture")
}
