//! Cursor position tracking and movement

use bevy::ecs::system::In;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::parse_request;
use super::resolve_window;
use super::send_motion_events;
use super::serialize_response;

// ============================================================================
// Types
// ============================================================================

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
    pub(crate) fn get_position(&self, window: Entity) -> Vec2 {
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
    pub(crate) fn update_position(&mut self, window: Entity, new_pos: Vec2) -> Vec2 {
        let old_pos = self.get_position(window);
        self.positions.insert(window, new_pos);
        new_pos - old_pos
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `move_mouse` BRP method
pub(crate) fn move_mouse_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
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
    let (new_position, delta) = if let Some(delta) = request.delta {
        (current_pos + delta, delta)
    } else if let Some(pos) = request.position {
        (pos, pos - current_pos)
    } else {
        // Validation above already rejects this case
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: "Must provide either 'delta' or 'position'".to_string(),
            data:    None,
        });
    };

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

// ============================================================================
// Systems
// ============================================================================

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
///    -> SimulatedCursorPosition stores [100, 100]
///
/// 2. User physically moves mouse to [300, 300]
///    -> WITHOUT sync: SimulatedCursorPosition still [100, 100]
///    -> WITH sync: SimulatedCursorPosition updated to [300, 300]
///
/// 3. BRP: move_mouse(delta: [50, 50])
///    -> WITHOUT sync: Moves to [150, 150] (jumps from real position)
///    -> WITH sync: Moves to [350, 350] (correct relative movement)
/// ```
///
/// ## Use Cases
///
/// - **Hybrid testing**: BRP automation mixed with manual interaction
/// - **Debugging**: Developer moves mouse while running BRP commands
/// - **Recovery**: Syncs state after unexpected manual input
pub(crate) fn sync_cursor_position(
    mut cursor_res: ResMut<SimulatedCursorPosition>,
    mut cursor_events: MessageReader<CursorMoved>,
) {
    for event in cursor_events.read() {
        cursor_res.positions.insert(event.window, event.position);
        cursor_res.last_window = Some(event.window);
    }
}
