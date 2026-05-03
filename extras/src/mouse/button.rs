//! Mouse button press with configurable hold duration

use bevy::ecs::system::In;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::window::WindowEvent;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::constants::DEFAULT_MOUSE_DURATION_MS;
use super::constants::MAX_MOUSE_DURATION_MS;
use super::support;

// ============================================================================
// Types
// ============================================================================

/// Request structure for `send_mouse_button`
#[derive(Deserialize)]
struct SendMouseButtonRequest {
    /// Mouse button to press
    pub button:      MouseButton,
    /// Duration in milliseconds to hold button (default: 100ms, max: 60000ms)
    #[serde(default)]
    pub duration_ms: Option<u32>,
    /// Target window entity (None = primary window)
    #[serde(default)]
    pub window:      Option<u64>,
}

/// Response structure for `send_mouse_button`
#[derive(Serialize)]
struct SendMouseButtonResponse {
    /// Button that was pressed
    pub button:      MouseButton,
    /// Duration in milliseconds the button was held
    pub duration_ms: u32,
}

// ============================================================================
// Components
// ============================================================================

/// Component for timed mouse button releases
///
/// Attached to entities to track button press duration. When the timer expires,
/// the button release event is sent and the entity is despawned.
#[derive(Component)]
pub(super) struct TimedButtonRelease {
    /// Which button to release
    pub button: MouseButton,
    /// Which window the button was pressed in (None = primary)
    pub window: Option<Entity>,
    /// Timer tracking remaining duration
    pub timer:  Timer,
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `send_mouse_button` BRP method
///
/// Sends a mouse button press with configurable hold duration before automatic release
pub fn send_mouse_button_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: SendMouseButtonRequest =
        support::parse_request(params, support::EmptyParamsPolicy::Reject)?;

    // Validate duration
    let duration_ms = request.duration_ms.unwrap_or(DEFAULT_MOUSE_DURATION_MS);
    if duration_ms > MAX_MOUSE_DURATION_MS {
        return Err(bevy_remote::BrpError {
            code:    INVALID_PARAMS,
            message: format!(
                "Duration exceeds maximum: {duration_ms}ms > {MAX_MOUSE_DURATION_MS}ms"
            ),
            data:    None,
        });
    }

    let window = support::resolve_window(world, request.window)?;
    support::send_timed_button_press(world, request.button, window, duration_ms);

    support::serialize_response(
        SendMouseButtonResponse {
            button: request.button,
            duration_ms,
        },
        "send_mouse_button",
    )
}

// ============================================================================
// Systems
// ============================================================================

/// System to process timed button releases
///
/// Ticks timers on `TimedButtonRelease` components. When a timer finishes,
/// sends the button release event and despawns the entity.
pub(super) fn process_timed_button_releases(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimedButtonRelease)>,
    mut button_events: MessageWriter<MouseButtonInput>,
    mut window_events: MessageWriter<WindowEvent>,
) {
    for (entity, mut release) in &mut query {
        release.timer.tick(time.delta());

        if release.timer.is_finished() {
            let event = MouseButtonInput {
                button: release.button,
                state:  ButtonState::Released,
                window: support::resolve_window_entity(release.window),
            };
            window_events.write(WindowEvent::from(event));
            button_events.write(event);

            // Despawn the entity
            commands.entity(entity).despawn();
        }
    }
}
