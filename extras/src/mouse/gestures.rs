//! Trackpad gesture events (pinch, rotation, double tap)

use bevy::ecs::system::In;
use bevy::input::gestures::DoubleTapGesture;
use bevy::input::gestures::PinchGesture;
use bevy::input::gestures::RotationGesture;
use bevy::prelude::*;
use bevy_remote::BrpResult;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_request;
use crate::brp_request::EmptyParamsPolicy;
use crate::constants::METHOD_DOUBLE_TAP_GESTURE;
use crate::constants::METHOD_PINCH_GESTURE;
use crate::constants::METHOD_ROTATION_GESTURE;
use crate::window_event;

// ============================================================================
// Types
// ============================================================================

/// Request structure for `pinch_gesture`
#[derive(Deserialize)]
struct PinchGestureRequest {
    /// Pinch delta (positive = zoom in, negative = zoom out)
    delta: f32,
}

/// Response structure for `pinch_gesture`
#[derive(Serialize)]
struct PinchGestureResponse {
    /// Pinch delta that was applied
    delta: f32,
}

/// Request structure for `rotation_gesture`
#[derive(Deserialize)]
struct RotationGestureRequest {
    /// Rotation delta in radians
    delta: f32,
}

/// Response structure for `rotation_gesture`
#[derive(Serialize)]
struct RotationGestureResponse {
    /// Rotation delta that was applied
    delta: f32,
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
pub(crate) fn pinch_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: PinchGestureRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;

    window_event::write_input_event(world, PinchGesture(request.delta));

    brp_request::serialize_response(
        PinchGestureResponse {
            delta: request.delta,
        },
        METHOD_PINCH_GESTURE,
    )
}

/// Handler for `rotation_gesture` BRP method
pub(crate) fn rotation_gesture_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let request: RotationGestureRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;

    window_event::write_input_event(world, RotationGesture(request.delta));

    brp_request::serialize_response(
        RotationGestureResponse {
            delta: request.delta,
        },
        METHOD_ROTATION_GESTURE,
    )
}

/// Handler for `double_tap_gesture` BRP method
pub(crate) fn double_tap_gesture_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let _: DoubleTapGestureRequest = brp_request::parse_request(params, EmptyParamsPolicy::Allow)?;

    window_event::write_input_event(world, DoubleTapGesture);

    brp_request::serialize_response(DoubleTapGestureResponse {}, METHOD_DOUBLE_TAP_GESTURE)
}
