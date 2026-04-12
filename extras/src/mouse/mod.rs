//! Mouse input simulation for Bevy Remote Protocol
//!
//! This module provides comprehensive mouse input simulation including:
//! - Cursor movement (delta and absolute positioning)
//! - Mouse button presses (single click, double click)
//! - Drag operations with interpolation
//! - Scroll wheel events
//! - Trackpad gestures (pinch, rotation, double tap)
//!
//! All operations support multi-window targeting.

mod button;
mod click;
mod cursor;
mod drag;
mod gestures;
mod scroll;

use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy_remote::BrpError;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use self::button::TimedButtonRelease;
pub(super) use self::button::process_timed_button_releases;
pub(super) use self::button::send_mouse_button_handler;
pub(super) use self::click::click_mouse_handler;
pub(super) use self::click::double_click_mouse_handler;
pub(super) use self::click::process_scheduled_clicks;
pub(super) use self::cursor::SimulatedCursorPosition;
pub(super) use self::cursor::move_mouse_handler;
pub(super) use self::cursor::sync_cursor_position;
pub(super) use self::drag::drag_mouse_handler;
pub(super) use self::drag::process_drag_operations;
pub(super) use self::gestures::double_tap_gesture_handler;
pub(super) use self::gestures::pinch_gesture_handler;
pub(super) use self::gestures::rotation_gesture_handler;
pub(super) use self::scroll::scroll_mouse_handler;
use crate::window_event;

// ============================================================================
// Constants
// ============================================================================

/// Maximum duration for timed mouse button releases (60 seconds)
const MAX_MOUSE_DURATION_MS: u32 = 60_000;

/// Default duration for mouse button presses (100 milliseconds)
const DEFAULT_MOUSE_DURATION_MS: u32 = 100;

/// Default delay between clicks for double click (250 milliseconds)
const DEFAULT_DOUBLE_CLICK_DELAY_MS: u32 = 250;

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse BRP request parameters into strongly typed request struct
///
/// Handles parameter extraction, validation, and error conversion for all handlers.
/// Provides consistent error messages across the module.
///
/// # Arguments
/// * `params` - Optional JSON value from BRP request
/// * `allow_empty` - If true, allows None params (creates empty object for deserialization)
///
/// # Returns
/// Parsed request struct or BRP error with `INVALID_PARAMS` code
fn parse_request<T: serde::de::DeserializeOwned>(
    params: Option<Value>,
    allow_empty: bool,
) -> Result<T, BrpError> {
    if allow_empty && params.is_none() {
        // For requests with no required fields (e.g., `DoubleTapGestureRequest`)
        return serde_json::from_value(Value::Object(Map::default())).map_err(|e| BrpError {
            code:    INVALID_PARAMS,
            message: format!("Failed to parse parameters: {e}"),
            data:    None,
        });
    }

    let params = params.ok_or_else(|| BrpError {
        code:    INVALID_PARAMS,
        message: "Missing request parameters".to_string(),
        data:    None,
    })?;

    serde_json::from_value(params).map_err(|e| BrpError {
        code:    INVALID_PARAMS,
        message: format!("Failed to parse parameters: {e}"),
        data:    None,
    })
}

/// Serialize BRP response with standardized error handling
///
/// Provides consistent serialization error handling and logging across all handlers.
///
/// # Arguments
/// * `response` - Response struct to serialize
/// * `handler_name` - Name of the handler (for logging)
///
/// # Returns
/// Serialized JSON value or BRP error with `INTERNAL_ERROR` code
fn serialize_response<T: Serialize>(response: T, handler_name: &str) -> bevy_remote::BrpResult {
    serde_json::to_value(response).map_err(|e| {
        warn!("Failed to serialize {handler_name} response: {e}");
        BrpError {
            code:    bevy_remote::error_codes::INTERNAL_ERROR,
            message: format!("Failed to serialize response: {e}"),
            data:    None,
        }
    })
}

/// Get window entity with fallback to placeholder
///
/// Standardizes window entity unwrapping across all systems.
///
/// # Arguments
/// * `window` - Optional window entity
///
/// # Returns
/// Window entity or `Entity::PLACEHOLDER` if None
fn resolve_window_entity(window: Option<Entity>) -> Entity { window.unwrap_or(Entity::PLACEHOLDER) }

/// Send mouse button press with automatic timed release
///
/// Handles the common pattern of sending a button press event followed by
/// spawning a timed release component. Used by click and `send_mouse_button` handlers.
///
/// # Arguments
/// * `world` - Mutable world reference
/// * `button` - Mouse button to press
/// * `window` - Target window entity
/// * `duration_ms` - Duration in milliseconds before automatic release
fn send_timed_button_press(
    world: &mut World,
    button: MouseButton,
    window: Entity,
    duration_ms: u32,
) {
    // Send button press event to both individual and `WindowEvent` channels
    window_event::write_input_event(
        world,
        MouseButtonInput {
            button,
            state: ButtonState::Pressed,
            window,
        },
    );

    // Spawn timed release component
    world.spawn(TimedButtonRelease {
        button,
        window: Some(window),
        timer: Timer::new(
            std::time::Duration::from_millis(duration_ms.into()),
            TimerMode::Once,
        ),
    });
}

/// Send coordinated mouse motion events
///
/// Sends both device-level `MouseMotion` (delta) and window-level `CursorMoved` (position)
/// events together, and updates the `Window` component's internal cursor position.
///
/// The `Window` component update is critical because `window.cursor_position()` reads from
/// `Window.physical_cursor_position`, which is normally only set by winit's OS-level cursor
/// handler. Without this update, systems that check `window.cursor_position()` (e.g.,
/// `OrbitCam`, UI hit-testing) would see `None` when the app is unfocused and ignore
/// all BRP-injected input.
///
/// # Arguments
/// * `world` - Mutable world reference
/// * `window` - Target window entity
/// * `position` - New cursor position in window coordinates (logical pixels)
/// * `delta` - Delta movement from previous position
fn send_motion_events(world: &mut World, window: Entity, position: Vec2, delta: Vec2) {
    window_event::write_input_event(world, MouseMotion { delta });
    window_event::write_input_event(
        world,
        CursorMoved {
            window,
            position,
            delta: Some(delta),
        },
    );

    // Update the `Window` component's cursor position so that
    // `window.cursor_position()` returns the correct value even when unfocused
    if let Some(mut window_component) = world.get_mut::<Window>(window) {
        window_component.set_cursor_position(Some(position));
    }
}

/// Resolve window entity from optional u64 ID
///
/// Resolution order when `window_id` is None:
/// 1. Last window the cursor was moved to (from `SimulatedCursorPosition`)
/// 2. Primary window (fallback)
fn resolve_window(world: &mut World, window_id: Option<u64>) -> Result<Entity, BrpError> {
    if let Some(id) = window_id {
        let entity = Entity::from_bits(id);
        // Verify entity exists and is a window
        if world.get_entity(entity).is_err() {
            return Err(BrpError {
                code:    INVALID_PARAMS,
                message: format!("Invalid window entity: {id}"),
                data:    None,
            });
        }
        return Ok(entity);
    }

    // Default to the last window the cursor was moved to
    if let Some(cursor_pos) = world.get_resource::<SimulatedCursorPosition>()
        && let Some(last_window) = cursor_pos.last_window
    {
        return Ok(last_window);
    }

    // Fall back to primary window
    let entity = {
        let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
        let mut iter = query.iter(world);
        iter.next()
    };

    entity.ok_or_else(|| BrpError {
        code:    INVALID_PARAMS,
        message: "No primary window found".to_string(),
        data:    None,
    })
}
