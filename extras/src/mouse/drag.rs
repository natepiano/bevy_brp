//! Mouse drag operations with interpolation

use bevy::ecs::system::In;
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy::window::WindowEvent;
use bevy_remote::BrpResult;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::constants::MIN_DRAG_FRAMES;
use super::cursor::SimulatedCursorPosition;
use super::modifier;
use super::modifier::Modifier;
use super::modifier::ModifierEvents;
use super::support;
use crate::brp_request;
use crate::brp_request::EmptyParamsPolicy;
use crate::constants::METHOD_DRAG_MOUSE;

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
    /// Release the mouse button.
    Released,
    /// Release the modifier keys on the next frame.
    ReleaseModifiers,
}

/// Request structure for `drag_mouse`
#[derive(Deserialize)]
struct DragMouseRequest {
    /// Button to hold during drag
    button:    MouseButton,
    /// Starting position
    start:     Vec2,
    /// Ending position
    end:       Vec2,
    /// Number of frames to interpolate over
    frames:    u32,
    /// Target window entity (None = primary window)
    #[serde(default)]
    window:    Option<u64>,
    /// Keyboard modifiers held throughout the drag.
    #[serde(default, deserialize_with = "modifier::deserialize_modifiers")]
    modifiers: Vec<Modifier>,
}

/// Response structure for `drag_mouse`
#[derive(Serialize)]
struct DragMouseResponse {
    /// Button that was used for dragging
    button:    MouseButton,
    /// Starting position
    start:     Vec2,
    /// Ending position
    end:       Vec2,
    /// Number of frames for interpolation
    frames:    u32,
    /// Keyboard modifiers held throughout the drag.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    modifiers: Vec<Modifier>,
}

// ============================================================================
// Components
// ============================================================================

/// Component for drag operations
///
/// Move the cursor between the start and end positions over several frames.
/// Release the mouse button before the modifier keys.
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
    pub drag_state:    DragState,
    /// Keyboard modifiers held until the frame after the button release.
    pub modifiers:     Vec<Modifier>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `drag_mouse` BRP method
pub(crate) fn drag_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: DragMouseRequest = brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;

    // Validate frames
    if request.frames < MIN_DRAG_FRAMES {
        return Err(brp_request::invalid_params(
            "Frames must be greater than 0".to_string(),
        ));
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
        drag_state:    DragState::Pressed,
        modifiers:     request.modifiers.clone(),
    });

    brp_request::serialize_response(
        DragMouseResponse {
            button:    request.button,
            start:     request.start,
            end:       request.end,
            frames:    request.frames,
            modifiers: request.modifiers,
        },
        METHOD_DRAG_MOUSE,
    )
}

// ============================================================================
// Systems
// ============================================================================

/// The channels and windows that receive input from active drags.
#[derive(SystemParam)]
pub(super) struct DragInput<'w, 's> {
    modifiers:     ModifierEvents<'w>,
    button_events: MessageWriter<'w, MouseButtonInput>,
    motion_events: MessageWriter<'w, MouseMotion>,
    cursor_events: MessageWriter<'w, CursorMoved>,
    windows:       Query<'w, 's, &'static mut Window>,
}

impl DragInput<'_, '_> {
    fn button(&mut self, button: MouseButton, state: ButtonState, window: Entity) {
        let event = MouseButtonInput {
            button,
            state,
            window,
        };
        self.modifiers.window_events.write(WindowEvent::from(event));
        self.button_events.write(event);
    }

    fn motion(&mut self, window: Entity, position: Vec2, delta: Vec2) {
        let motion = MouseMotion { delta };
        self.modifiers
            .window_events
            .write(WindowEvent::from(motion));
        self.motion_events.write(motion);
        let cursor = CursorMoved {
            window,
            position,
            delta: Some(delta),
        };
        self.modifiers
            .window_events
            .write(WindowEvent::from(cursor.clone()));
        self.cursor_events.write(cursor);
        if let Ok(mut window_component) = self.windows.get_mut(window) {
            window_component.set_cursor_position(Some(position));
        }
    }
}

/// Process each drag and release its modifiers after the button release.
///
/// Write input events directly so cursor synchronization sees this frame's position.
pub(super) fn process_drag_operations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DragOperation)>,
    mut cursor_res: ResMut<SimulatedCursorPosition>,
    mut input: DragInput,
) {
    for (entity, mut drag) in &mut query {
        let window = support::resolve_window_entity(drag.window);

        match drag.drag_state {
            DragState::Pressed => {
                input.modifiers.press(&drag.modifiers, window);
                input.button(drag.button, ButtonState::Pressed, window);
                let delta = cursor_res.update_position(window, drag.start);
                input.motion(window, drag.start, delta);
                drag.drag_state = DragState::Dragging;
            },
            DragState::Dragging => {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "drag frame counts are small; f32 loses integer precision only above \
                              2^24"
                )]
                let t = drag.current_frame as f32 / drag.total_frames as f32;
                let position = drag.start.lerp(drag.end, t);
                let delta = cursor_res.update_position(window, position);
                input.motion(window, position, delta);

                drag.current_frame += 1;
                if drag.current_frame > drag.total_frames {
                    drag.drag_state = DragState::Released;
                }
            },
            DragState::Released => {
                input.button(drag.button, ButtonState::Released, window);
                if drag.modifiers.is_empty() {
                    commands.entity(entity).despawn();
                } else {
                    drag.drag_state = DragState::ReleaseModifiers;
                }
            },
            DragState::ReleaseModifiers => {
                input.modifiers.release(&drag.modifiers, window);
                commands.entity(entity).despawn();
            },
        }
    }
}
