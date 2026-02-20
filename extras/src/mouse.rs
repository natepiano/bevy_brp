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

use std::time::Duration;

use bevy::ecs::system::In;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy::window::WindowEvent;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::window_event::write_input_event;

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
        // For requests with no required fields (e.g., DoubleTapGestureRequest)
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
fn serialize_response<T: Serialize>(response: T, handler_name: &str) -> BrpResult {
    serde_json::to_value(response).map_err(|e| {
        warn!("Failed to serialize {handler_name} response: {e}");
        BrpError {
            code:    INTERNAL_ERROR,
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
    write_input_event(
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
        timer: Timer::new(Duration::from_millis(duration_ms.into()), TimerMode::Once),
    });
}

/// Send coordinated mouse motion events
///
/// Sends both device-level `MouseMotion` (delta) and window-level `CursorMoved` (position)
/// events together. These events must be paired for correct mouse behavior in Bevy.
///
/// # Arguments
/// * `world` - Mutable world reference
/// * `window` - Target window entity
/// * `position` - New cursor position in window coordinates
/// * `delta` - Delta movement from previous position
fn send_motion_events(world: &mut World, window: Entity, position: Vec2, delta: Vec2) {
    write_input_event(world, MouseMotion { delta });
    write_input_event(
        world,
        CursorMoved {
            window,
            position,
            delta: Some(delta),
        },
    );
}

// ============================================================================
// Type Definitions
// ============================================================================

/// Drag operation state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragState {
    /// Button pressed, cursor moved to start position
    Pressed,
    /// Actively dragging, interpolating between start and end
    Dragging,
    /// Button released, operation complete
    Released,
}

/// Request structure for `move_mouse`
#[derive(Deserialize)]
pub struct MoveMouseRequest {
    /// Delta movement (mutually exclusive with position)
    #[serde(default)]
    pub delta:    Option<Vec2>,
    /// Absolute position (mutually exclusive with delta)
    #[serde(default)]
    pub position: Option<Vec2>,
    /// Target window entity (None = primary window)
    #[serde(default)]
    pub window:   Option<u64>,
}

/// Response structure for `move_mouse`
#[derive(Serialize)]
pub struct MoveMouseResponse {
    /// New cursor position
    pub new_position: Vec2,
    /// Delta that was applied
    pub delta:        Vec2,
}

// ============================================================================
// Resources
// ============================================================================

/// Tracks cursor position for delta calculation
///
/// This resource maintains the last known cursor position **per window** to enable
/// delta-based movement calculations. When moving by delta, the new position is
/// calculated relative to the stored position for that specific window.
///
/// ## Synchronization
///
/// The resource is synchronized with both simulated and real mouse input:
/// - **Simulated input**: Updated by `move_mouse_handler` and `process_drag_operations`
/// - **Real input**: Updated by `sync_cursor_position` system which listens to actual `CursorMoved`
///   events
///
/// This dual-sync approach ensures delta calculations work correctly even in hybrid
/// scenarios where both BRP commands and physical mouse movements occur. Without
/// real input sync, delta commands could cause unexpected jumps after physical
/// mouse movement.
#[derive(Resource, Default)]
pub struct SimulatedCursorPosition {
    /// Per-window cursor positions
    pub positions:   std::collections::HashMap<Entity, Vec2>,
    /// The last window the cursor was moved to (used as default for click/scroll operations)
    pub last_window: Option<Entity>,
}

impl SimulatedCursorPosition {
    /// Get cursor position for window, defaulting to origin if not set
    ///
    /// # Arguments
    /// * `window` - Window entity to get position for
    ///
    /// # Returns
    /// Current cursor position or `Vec2::ZERO` if no position stored
    pub fn get_position(&self, window: Entity) -> Vec2 {
        self.positions.get(&window).copied().unwrap_or(Vec2::ZERO)
    }

    /// Update cursor position and return the delta from previous position
    ///
    /// # Arguments
    /// * `window` - Window entity to update
    /// * `new_pos` - New cursor position
    ///
    /// # Returns
    /// Delta from previous position (or from origin if no previous position)
    pub fn update_position(&mut self, window: Entity, new_pos: Vec2) -> Vec2 {
        let old_pos = self.get_position(window);
        self.positions.insert(window, new_pos);
        new_pos - old_pos
    }
}

// ============================================================================
// Components
// ============================================================================

/// Component for timed mouse button releases
///
/// Attached to entities to track button press duration. When the timer expires,
/// the button release event is sent and the entity is despawned.
#[derive(Component)]
pub struct TimedButtonRelease {
    /// Which button to release
    pub button: MouseButton,
    /// Which window the button was pressed in (None = primary)
    pub window: Option<Entity>,
    /// Timer tracking remaining duration
    pub timer:  Timer,
}

/// Component for drag operations
///
/// Manages multi-frame drag operations with linear interpolation between
/// start and end positions. Runs a state machine: Pressed -> Dragging -> Released.
#[derive(Component)]
pub struct DragOperation {
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
// Request/Response Types - Send Mouse Button
// ============================================================================

/// Request structure for `send_mouse_button`
#[derive(Deserialize)]
pub struct SendMouseButtonRequest {
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
pub struct SendMouseButtonResponse {
    /// Button that was pressed
    pub button:      MouseButton,
    /// Duration in milliseconds the button was held
    pub duration_ms: u32,
}

// ============================================================================
// Request/Response Types - Click Mouse
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

// ============================================================================
// Request/Response Types - Double Click Mouse
// ============================================================================

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
// Request/Response Types - Drag Mouse
// ============================================================================

/// Request structure for `drag_mouse`
#[derive(Deserialize)]
pub struct DragMouseRequest {
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
pub struct DragMouseResponse {
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
// Request/Response Types - Scroll Mouse
// ============================================================================

/// Request structure for `scroll_mouse`
#[derive(Deserialize)]
pub struct ScrollMouseRequest {
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
pub struct ScrollMouseResponse {
    /// Horizontal scroll amount
    pub x:    f32,
    /// Vertical scroll amount
    pub y:    f32,
    /// Scroll unit that was used
    pub unit: MouseScrollUnit,
}

// ============================================================================
// Request/Response Types - Pinch Gesture
// ============================================================================

/// Request structure for `pinch_gesture`
#[derive(Deserialize)]
pub struct PinchGestureRequest {
    /// Pinch delta (positive = zoom in, negative = zoom out)
    pub delta: f32,
}

/// Response structure for `pinch_gesture`
#[derive(Serialize)]
pub struct PinchGestureResponse {
    /// Pinch delta that was applied
    pub delta: f32,
}

// ============================================================================
// Request/Response Types - Rotation Gesture
// ============================================================================

/// Request structure for `rotation_gesture`
#[derive(Deserialize)]
pub struct RotationGestureRequest {
    /// Rotation delta in radians
    pub delta: f32,
}

/// Response structure for `rotation_gesture`
#[derive(Serialize)]
pub struct RotationGestureResponse {
    /// Rotation delta that was applied
    pub delta: f32,
}

// ============================================================================
// Request/Response Types - Double Tap Gesture
// ============================================================================

/// Request structure for `double_tap_gesture`
#[derive(Deserialize)]
pub struct DoubleTapGestureRequest {
    // No parameters needed
}

/// Response structure for `double_tap_gesture`
#[derive(Serialize)]
pub struct DoubleTapGestureResponse {
    // No fields needed - success is indicated by Ok result
}

// ============================================================================
// Helper Functions
// ============================================================================

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

// ============================================================================
// Handler Functions
// ============================================================================

/// Handler for `move_mouse` BRP method
pub fn move_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: MoveMouseRequest = parse_request(params, false)?;

    // Validate that exactly one of delta or position is provided
    if request.delta.is_none() && request.position.is_none() {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: "Must provide either 'delta' or 'position'".to_string(),
            data:    None,
        });
    }

    if request.delta.is_some() && request.position.is_some() {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: "Cannot provide both 'delta' and 'position'".to_string(),
            data:    None,
        });
    }

    // Resolve window entity
    let window = resolve_window(world, request.window)?;

    // Get or create simulated cursor position resource
    if !world.contains_resource::<SimulatedCursorPosition>() {
        world.init_resource::<SimulatedCursorPosition>();
    }

    let mut cursor_res = world.resource_mut::<SimulatedCursorPosition>();

    // Get current position for this window (default to origin if not set)
    let current_pos = cursor_res.get_position(window);

    // Calculate new position and delta
    let (new_position, delta) = request.delta.map_or_else(
        || {
            #[allow(clippy::expect_used)]
            let new_pos = request
                .position
                .expect("Position is required when delta is not provided");
            let delta = new_pos - current_pos;
            (new_pos, delta)
        },
        |delta| {
            let new_pos = current_pos + delta;
            (new_pos, delta)
        },
    );

    // Update resource and send motion events
    cursor_res.positions.insert(window, new_position);
    cursor_res.last_window = Some(window);
    send_motion_events(world, window, new_position, delta);

    serialize_response(
        MoveMouseResponse {
            new_position,
            delta,
        },
        "move_mouse",
    )
}

/// Handler for `click_mouse` BRP method
///
/// Performs a simple click (press and release) with default timing
pub fn click_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
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

/// Handler for `send_mouse_button` BRP method
///
/// Sends a mouse button press with configurable hold duration before automatic release
pub fn send_mouse_button_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: SendMouseButtonRequest = parse_request(params, false)?;

    // Validate duration
    let duration_ms = request.duration_ms.unwrap_or(DEFAULT_MOUSE_DURATION_MS);
    if duration_ms > MAX_MOUSE_DURATION_MS {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: format!(
                "Duration exceeds maximum: {duration_ms}ms > {MAX_MOUSE_DURATION_MS}ms"
            ),
            data:    None,
        });
    }

    let window = resolve_window(world, request.window)?;
    send_timed_button_press(world, request.button, window, duration_ms);

    serialize_response(
        SendMouseButtonResponse {
            button: request.button,
            duration_ms,
        },
        "send_mouse_button",
    )
}

/// Handler for `double_click_mouse` BRP method
pub fn double_click_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: DoubleClickMouseRequest = parse_request(params, false)?;
    let delay_ms = request.delay_ms.unwrap_or(DEFAULT_DOUBLE_CLICK_DELAY_MS);
    let window = resolve_window(world, request.window)?;

    // First click: press + immediate release
    write_input_event(
        world,
        MouseButtonInput {
            button: request.button,
            state: ButtonState::Pressed,
            window,
        },
    );
    write_input_event(
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

/// Handler for `scroll_mouse` BRP method
pub fn scroll_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: ScrollMouseRequest = parse_request(params, false)?;
    let window = resolve_window(world, request.window)?;

    write_input_event(
        world,
        MouseWheel {
            unit: request.unit,
            x: request.x,
            y: request.y,
            window,
        },
    );

    serialize_response(
        ScrollMouseResponse {
            x:    request.x,
            y:    request.y,
            unit: request.unit,
        },
        "scroll_mouse",
    )
}

/// Handler for `drag_mouse` BRP method
pub fn drag_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: DragMouseRequest = parse_request(params, false)?;

    // Validate frames
    if request.frames == 0 {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: "Frames must be greater than 0".to_string(),
            data:    None,
        });
    }

    let window = resolve_window(world, request.window)?;

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

    serialize_response(
        DragMouseResponse {
            button: request.button,
            start:  request.start,
            end:    request.end,
            frames: request.frames,
        },
        "drag_mouse",
    )
}

/// Handler for `pinch_gesture` BRP method
pub fn pinch_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: PinchGestureRequest = parse_request(params, false)?;

    write_input_event(world, bevy::input::gestures::PinchGesture(request.delta));

    serialize_response(
        PinchGestureResponse {
            delta: request.delta,
        },
        "pinch_gesture",
    )
}

/// Handler for `rotation_gesture` BRP method
pub fn rotation_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: RotationGestureRequest = parse_request(params, false)?;

    write_input_event(world, bevy::input::gestures::RotationGesture(request.delta));

    serialize_response(
        RotationGestureResponse {
            delta: request.delta,
        },
        "rotation_gesture",
    )
}

/// Handler for `double_tap_gesture` BRP method
pub fn double_tap_gesture_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let _request: DoubleTapGestureRequest = parse_request(params, true)?;

    write_input_event(world, bevy::input::gestures::DoubleTapGesture);

    serialize_response(DoubleTapGestureResponse {}, "double_tap_gesture")
}

// ============================================================================
// Systems
// ============================================================================

/// System to process timed button releases
///
/// Ticks timers on `TimedButtonRelease` components. When a timer finishes,
/// sends the button release event and despawns the entity.
pub fn process_timed_button_releases(
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
                window: resolve_window_entity(release.window),
            };
            window_events.write(WindowEvent::from(event));
            button_events.write(event);

            // Despawn the entity
            commands.entity(entity).despawn();
        }
    }
}

/// System to process scheduled clicks (for double-click timing)
///
/// When the delay timer finishes:
/// - Sends the second press event
/// - Spawns a `TimedButtonRelease` for the release
/// - Despawns the scheduled click entity
pub fn process_scheduled_clicks(
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

/// System to process drag operations
///
/// Runs a state machine for each `DragOperation`:
/// - Pressed: Send button press, move to start, transition to Dragging
/// - Dragging: Interpolate position, send motion events, advance frame
/// - Released: Send button release, despawn entity
pub fn process_drag_operations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DragOperation)>,
    mut cursor_res: ResMut<SimulatedCursorPosition>,
    mut motion_events: MessageWriter<MouseMotion>,
    mut cursor_events: MessageWriter<CursorMoved>,
    mut button_events: MessageWriter<MouseButtonInput>,
    mut window_events: MessageWriter<WindowEvent>,
) {
    for (entity, mut drag) in &mut query {
        let window = resolve_window_entity(drag.window);

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

                // Transition to dragging
                drag.state = DragState::Dragging;
            },
            DragState::Dragging => {
                // Calculate interpolation factor
                #[allow(clippy::cast_precision_loss)]
                let t = drag.current_frame as f32 / drag.total_frames as f32;
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

/// System to sync `SimulatedCursorPosition` with real mouse input
///
/// This system listens to actual `CursorMoved` events (from physical mouse movement)
/// and updates the `SimulatedCursorPosition` resource to reflect the real cursor
/// position.
///
/// ## Purpose
///
/// Without this sync, delta-based BRP commands would use stale position data if the
/// user physically moved their mouse between commands, causing unexpected cursor
/// jumps or incorrect movement.
///
/// ## Example Scenario
///
/// ```text
/// 1. BRP: move_mouse(position: [100, 100])
///    → SimulatedCursorPosition stores [100, 100]
///
/// 2. User physically moves mouse to [300, 300]
///    → WITHOUT sync: SimulatedCursorPosition still [100, 100] ❌
///    → WITH sync: SimulatedCursorPosition updated to [300, 300] ✅
///
/// 3. BRP: move_mouse(delta: [50, 50])
///    → WITHOUT sync: Moves to [150, 150] (jumps from real position) ❌
///    → WITH sync: Moves to [350, 350] (correct relative movement) ✅
/// ```
///
/// ## Use Cases
///
/// - **Hybrid testing**: BRP automation mixed with manual interaction
/// - **Debugging**: Developer moves mouse while running BRP commands
/// - **Recovery**: Syncs state after unexpected manual input
pub fn sync_cursor_position(
    mut cursor_res: ResMut<SimulatedCursorPosition>,
    mut cursor_events: MessageReader<CursorMoved>,
) {
    for event in cursor_events.read() {
        cursor_res.positions.insert(event.window, event.position);
        cursor_res.last_window = Some(event.window);
    }
}
