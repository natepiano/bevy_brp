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

pub(crate) use self::button::process_timed_button_releases;
pub(crate) use self::button::send_mouse_button_handler;
pub(crate) use self::click::click_mouse_handler;
pub(crate) use self::click::double_click_mouse_handler;
pub(crate) use self::click::process_scheduled_clicks;
pub(crate) use self::cursor::SimulatedCursorPosition;
pub(crate) use self::cursor::move_mouse_handler;
pub(crate) use self::cursor::sync_cursor_position;
pub(crate) use self::drag::drag_mouse_handler;
pub(crate) use self::drag::process_drag_operations;
pub(crate) use self::gestures::double_tap_gesture_handler;
pub(crate) use self::gestures::pinch_gesture_handler;
pub(crate) use self::gestures::rotation_gesture_handler;
pub(crate) use self::scroll::scroll_mouse_handler;
