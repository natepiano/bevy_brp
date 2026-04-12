//! Mouse click and double-click operations

use std::time::Duration;

use bevy::ecs::system::In;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::window::WindowEvent;
use bevy_remote::BrpResult;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::DEFAULT_DOUBLE_CLICK_DELAY_MS;
use super::DEFAULT_MOUSE_DURATION_MS;
use super::button::TimedButtonRelease;
use super::parse_request;
use super::resolve_window;
use super::resolve_window_entity;
use super::send_timed_button_press;
use super::serialize_response;
use crate::window_event;

// ============================================================================
// Types
// ============================================================================

/// Request structure for `click_mouse`
#[derive(Deserialize)]
pub struct ClickMouseRequest {
    /// Mouse button to click
    pub button: MouseButton,
    /// Target window entity (None = primary window)
    #[serde(default)]
    pub window: Option<u64>,
}

/// Response structure for `click_mouse`
#[derive(Serialize)]
pub struct ClickMouseResponse {
    /// Button that was clicked
    pub button: MouseButton,
}

/// Request structure for `double_click_mouse`
#[derive(Deserialize)]
pub struct DoubleClickMouseRequest {
    /// Mouse button to double click
    pub button:   MouseButton,
    /// Delay between clicks in milliseconds (default: 250ms)
    #[serde(default)]
    pub delay_ms: Option<u32>,
    /// Target window entity (None = primary window)
    #[serde(default)]
    pub window:   Option<u64>,
}

/// Response structure for `double_click_mouse`
#[derive(Serialize)]
pub struct DoubleClickMouseResponse {
    /// Button that was double-clicked
    pub button:   MouseButton,
    /// Delay between clicks in milliseconds
    pub delay_ms: u32,
}

// ============================================================================
// Components
// ============================================================================

/// Component for scheduled clicks (used in double-click implementation)
///
/// Delays the second click in a double-click operation to ensure proper
/// temporal separation between the two clicks.
#[derive(Component)]
pub struct ScheduledClick {
    /// Which button to click
    pub button:         MouseButton,
    /// Which window to target (None = primary)
    pub window:         Option<Entity>,
    /// Timer for delay before sending the click
    pub delay_timer:    Timer,
    /// Duration to hold the button pressed
    pub click_duration: u32,
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `click_mouse` BRP method
///
/// Performs a simple click (press and release) with default timing
pub(crate) fn click_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: ClickMouseRequest = parse_request(params, false)?;
    let window = resolve_window(world, request.window)?;

    send_timed_button_press(world, request.button, window, DEFAULT_MOUSE_DURATION_MS);

    serialize_response(
        ClickMouseResponse {
            button: request.button,
        },
        "click_mouse",
    )
}

/// Handler for `double_click_mouse` BRP method
pub(crate) fn double_click_mouse_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let request: DoubleClickMouseRequest = parse_request(params, false)?;
    let delay_ms = request.delay_ms.unwrap_or(DEFAULT_DOUBLE_CLICK_DELAY_MS);
    let window = resolve_window(world, request.window)?;

    // First click: press + immediate release
    window_event::write_input_event(
        world,
        MouseButtonInput {
            button: request.button,
            state: ButtonState::Pressed,
            window,
        },
    );
    window_event::write_input_event(
        world,
        MouseButtonInput {
            button: request.button,
            state: ButtonState::Released,
            window,
        },
    );

    // Schedule second click to happen after delay
    world.spawn(ScheduledClick {
        button:         request.button,
        window:         Some(window),
        delay_timer:    Timer::new(Duration::from_millis(delay_ms.into()), TimerMode::Once),
        click_duration: DEFAULT_MOUSE_DURATION_MS,
    });

    serialize_response(
        DoubleClickMouseResponse {
            button: request.button,
            delay_ms,
        },
        "double_click_mouse",
    )
}

// ============================================================================
// Systems
// ============================================================================

/// System to process scheduled clicks (for double-click timing)
///
/// When the delay timer finishes:
/// - Sends the second press event
/// - Spawns a `TimedButtonRelease` for the release
/// - Despawns the scheduled click entity
pub(crate) fn process_scheduled_clicks(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ScheduledClick)>,
    mut button_events: MessageWriter<MouseButtonInput>,
    mut window_events: MessageWriter<WindowEvent>,
) {
    for (entity, mut scheduled) in &mut query {
        scheduled.delay_timer.tick(time.delta());
        if scheduled.delay_timer.is_finished() {
            // Send press event
            let event = MouseButtonInput {
                button: scheduled.button,
                state:  ButtonState::Pressed,
                window: resolve_window_entity(scheduled.window),
            };
            window_events.write(WindowEvent::from(event));
            button_events.write(event);

            // Spawn timed release
            commands.spawn(TimedButtonRelease {
                button: scheduled.button,
                window: scheduled.window,
                timer:  Timer::new(
                    Duration::from_millis(scheduled.click_duration.into()),
                    TimerMode::Once,
                ),
            });

            commands.entity(entity).despawn();
        }
    }
}
