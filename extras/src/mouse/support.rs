//! Shared helper functions for mouse input simulation

use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy_remote::BrpError;

use super::button::TimedButtonRelease;
use super::cursor::SimulatedCursorPosition;
use super::modifier;
use super::modifier::Modifier;
use crate::brp_request;
use crate::window_event;

/// Get window entity with fallback to placeholder
///
/// Standardizes window entity unwrapping across all systems.
///
/// # Arguments
/// * `window` - Optional window entity
///
/// # Returns
/// `Window` entity or `Entity::PLACEHOLDER` if None
pub(super) fn resolve_window_entity(window: Option<Entity>) -> Entity {
    window.unwrap_or(Entity::PLACEHOLDER)
}

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
/// * `modifiers` - Keyboard modifiers held until the frame after the button release
pub(super) fn send_timed_button_press(
    world: &mut World,
    button: MouseButton,
    window: Entity,
    duration_ms: u32,
    modifiers: Vec<Modifier>,
) {
    modifier::send_modifier_events(world, &modifiers, ButtonState::Pressed, window);

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
        modifiers,
        button,
        window: Some(window),
        timer: Timer::new(
            std::time::Duration::from_millis(duration_ms.into()),
            TimerMode::Once,
        ),
    });
}

/// Record the simulated cursor position without changing the window's change flags.
pub(super) fn record_cursor_position(mut window: Mut<'_, Window>, position: Vec2) {
    window
        .bypass_change_detection()
        .set_cursor_position(Some(position));
}

/// Send motion events and record the simulated cursor position.
///
/// Record the position for `window.cursor_position()` readers, including `OrbitCam` and UI hit
/// tests. Bypass change detection so `bevy_winit` does not warp the OS cursor for each simulated
/// move.
///
/// `bevy_winit`'s `CachedWindow` retains the previous cursor position.
/// A later change to another `Window` field can cause one warp to the last simulated position.
/// On Wayland, that change can produce one cursor position error line.
///
/// # Arguments
/// * `world` - Mutable world reference
/// * `window` - Target window entity
/// * `position` - New cursor position in window coordinates (logical pixels)
/// * `delta` - Delta movement from previous position
pub(super) fn send_motion_events(world: &mut World, window: Entity, position: Vec2, delta: Vec2) {
    window_event::write_input_event(world, MouseMotion { delta });
    window_event::write_input_event(
        world,
        CursorMoved {
            window,
            position,
            delta: Some(delta),
        },
    );

    if let Some(window_component) = world.get_mut::<Window>(window) {
        record_cursor_position(window_component, position);
    }
}

/// Resolve window entity from optional u64 ID
///
/// Resolution order when `window_id` is None:
/// 1. Last window the cursor was moved to (from `SimulatedCursorPosition`)
/// 2. Primary window (fallback)
pub(super) fn resolve_window(
    world: &mut World,
    window_id: Option<u64>,
) -> Result<Entity, BrpError> {
    if let Some(id) = window_id {
        let entity = Entity::from_bits(id);
        // Verify entity exists and is a window
        if world.get_entity(entity).is_err() {
            return Err(brp_request::invalid_params(format!(
                "Invalid window entity: {id}"
            )));
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

    entity.ok_or_else(|| brp_request::invalid_params("No primary window found".to_string()))
}
