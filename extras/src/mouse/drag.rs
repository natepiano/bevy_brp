//! Mouse drag operations with interpolation

use bevy::ecs::system::In;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy::window::WindowEvent;
use bevy_kana::ToF32;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::cursor::SimulatedCursorPosition;
use super::support;

// ============================================================================
// Types
// ============================================================================

/// Drag operation state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DragState {
    /// Button pressed, cursor moved to start position
    Pressed,
    /// Actively dragging, interpolating between start and end
    Dragging,
    /// Button released, operation complete
    Released,
}

/// Request structure for `drag_mouse`
#[derive(Deserialize)]
struct DragMouseRequest {
    /// Button to hold during drag
    pub button: MouseButton,
    /// Starting position
    pub start:  Vec2,
    /// Ending position
    pub end:    Vec2,
    /// Number of frames to interpolate over
    pub frames: u32,
    /// Target window entity (None = primary window)
    #[serde(default)]
    pub window: Option<u64>,
}

/// Response structure for `drag_mouse`
#[derive(Serialize)]
struct DragMouseResponse {
    /// Button that was used for dragging
    pub button: MouseButton,
    /// Starting position
    pub start:  Vec2,
    /// Ending position
    pub end:    Vec2,
    /// Number of frames for interpolation
    pub frames: u32,
}

// ============================================================================
// Components
// ============================================================================

/// Component for drag operations
///
/// Manages multi-frame drag operations with linear interpolation between
/// start and end positions. Runs a state machine: Pressed -> Dragging -> Released.
#[derive(Component)]
pub(super) struct DragOperation {
    /// Which button is pressed during drag
    pub button:        MouseButton,
    /// Which window to target (None = primary)
    pub window:        Option<Entity>,
    /// Starting position
    pub start:         Vec2,
    /// Ending position
    pub end:           Vec2,
    /// Total number of frames for the drag
    pub total_frames:  u32,
    /// Current frame index
    pub current_frame: u32,
    /// Current state of the drag operation
    pub state:         DragState,
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `drag_mouse` BRP method
pub fn drag_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: DragMouseRequest = support::parse_request(params, false)?;

    // Validate frames
    if request.frames == 0 {
        return Err(bevy_remote::BrpError {
            code:    INVALID_PARAMS,
            message: "Frames must be greater than 0".to_string(),
            data:    None,
        });
    }

    let window = support::resolve_window(world, request.window)?;

    // Spawn drag operation component
    world.spawn(DragOperation {
        button:        request.button,
        window:        Some(window),
        start:         request.start,
        end:           request.end,
        total_frames:  request.frames,
        current_frame: 0,
        state:         DragState::Pressed,
    });

    support::serialize_response(
        DragMouseResponse {
            button: request.button,
            start:  request.start,
            end:    request.end,
            frames: request.frames,
        },
        "drag_mouse",
    )
}

// ============================================================================
// Systems
// ============================================================================

/// System to process drag operations
///
/// Runs a state machine for each `DragOperation`:
/// - Pressed: Send button press, move to start, transition to Dragging
/// - Dragging: Interpolate position, send motion events, advance frame
/// - Released: Send button release, despawn entity
pub(super) fn process_drag_operations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DragOperation)>,
    mut cursor_res: ResMut<SimulatedCursorPosition>,
    mut motion_events: MessageWriter<MouseMotion>,
    mut cursor_events: MessageWriter<CursorMoved>,
    mut button_events: MessageWriter<MouseButtonInput>,
    mut window_events: MessageWriter<WindowEvent>,
    mut windows: Query<&mut Window>,
) {
    for (entity, mut drag) in &mut query {
        let window = support::resolve_window_entity(drag.window);

        match drag.state {
            DragState::Pressed => {
                // Send button press
                let btn_event = MouseButtonInput {
                    button: drag.button,
                    state: ButtonState::Pressed,
                    window,
                };
                window_events.write(WindowEvent::from(btn_event));
                button_events.write(btn_event);

                // Move cursor to start position
                let delta = cursor_res.update_position(window, drag.start);

                // Send motion events
                let motion = MouseMotion { delta };
                window_events.write(WindowEvent::from(motion));
                motion_events.write(motion);
                let cursor = CursorMoved {
                    window,
                    position: drag.start,
                    delta: Some(delta),
                };
                window_events.write(WindowEvent::from(cursor.clone()));
                cursor_events.write(cursor);

                // Update `Window` component so `cursor_position()` works when unfocused
                if let Ok(mut win) = windows.get_mut(window) {
                    win.set_cursor_position(Some(drag.start));
                }

                // Transition to dragging
                drag.state = DragState::Dragging;
            },
            DragState::Dragging => {
                // Calculate interpolation factor
                let t = drag.current_frame.to_f32() / drag.total_frames.to_f32();
                let new_position = drag.start.lerp(drag.end, t);

                // Update position
                let delta = cursor_res.update_position(window, new_position);

                // Send motion events
                let motion = MouseMotion { delta };
                window_events.write(WindowEvent::from(motion));
                motion_events.write(motion);
                let cursor = CursorMoved {
                    window,
                    position: new_position,
                    delta: Some(delta),
                };
                window_events.write(WindowEvent::from(cursor.clone()));
                cursor_events.write(cursor);

                // Update `Window` component so `cursor_position()` works when unfocused
                if let Ok(mut win) = windows.get_mut(window) {
                    win.set_cursor_position(Some(new_position));
                }

                // Advance frame
                drag.current_frame += 1;

                // Check if done (use > to ensure we interpolate to t=1.0)
                if drag.current_frame > drag.total_frames {
                    drag.state = DragState::Released;
                }
            },
            DragState::Released => {
                // Send button release
                let btn_event = MouseButtonInput {
                    button: drag.button,
                    state: ButtonState::Released,
                    window,
                };
                window_events.write(WindowEvent::from(btn_event));
                button_events.write(btn_event);

                // Despawn entity
                commands.entity(entity).despawn();
            },
        }
    }
}
