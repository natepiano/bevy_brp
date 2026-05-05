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
mod constants;
mod cursor;
mod drag;
mod gestures;
mod scroll;
mod support;

use bevy::prelude::*;
use cursor::SimulatedCursorPosition;

pub(crate) use self::button::send_mouse_button_handler;
pub(crate) use self::click::click_mouse_handler;
pub(crate) use self::click::double_click_mouse_handler;
pub(crate) use self::cursor::move_mouse_handler;
pub(crate) use self::drag::drag_mouse_handler;
pub(crate) use self::gestures::double_tap_gesture_handler;
pub(crate) use self::gestures::pinch_gesture_handler;
pub(crate) use self::gestures::rotation_gesture_handler;
pub(crate) use self::scroll::scroll_mouse_handler;

pub(super) struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulatedCursorPosition>();
        app.add_systems(Update, cursor::sync_cursor_position);
        app.add_systems(Update, button::process_timed_button_releases);
        app.add_systems(Update, click::process_scheduled_clicks);
        app.add_systems(Update, drag::process_drag_operations);
    }
}
