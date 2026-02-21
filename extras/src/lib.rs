//! Extra BRP methods for Bevy applications
//!
//! This crate provides additional Bevy Remote Protocol (BRP) methods that can be added
//! to your Bevy application for enhanced remote control capabilities.
//!
//! # Usage
//!
//! Add the plugin to your Bevy app:
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_brp_extras::BrpExtrasPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BrpExtrasPlugin::default())
//!     .run();
//! ```
//!
//! # Available BRP Methods
//!
//! ## App Lifecycle
//!
//! ### `brp_extras/screenshot`
//! Captures a screenshot of the primary window and saves it to a file.
//! - `path` (string, required): file path where the screenshot will be saved
//!
//! **Note**: Requires Bevy's `png` feature enabled, otherwise files will be 0 bytes.
//!
//! ### `brp_extras/shutdown`
//! Schedules a graceful application shutdown. No parameters.
//!
//! ### `brp_extras/set_window_title`
//! Changes the title of the primary window.
//! - `title` (string, required): new window title
//!
//! ### `brp_extras/get_diagnostics`
//! Returns FPS and frame time diagnostics from Bevy's `DiagnosticsStore`.
//! No parameters. Requires the `diagnostics` cargo feature (enabled by default).
//!
//! Returns current, average, and smoothed values for FPS and frame time,
//! plus total frame count and history buffer metadata.
//!
//! ## Keyboard
//!
//! ### `brp_extras/send_keys`
//! Simulates keyboard input with a press-hold-release cycle. All keys are
//! pressed simultaneously and held for the specified duration.
//! - `keys` (array of strings, required): key codes (e.g., `["KeyA", "Space", "ShiftLeft"]`)
//! - `duration_ms` (u32, optional, default: 100, max: 60000): hold duration in milliseconds
//!
//! ### `brp_extras/type_text`
//! Types text sequentially, one character per frame, with proper shift handling
//! for uppercase and symbols.
//! - `text` (string, required): text to type (letters, numbers, symbols, newlines, tabs)
//!
//! ## Mouse
//!
//! All mouse methods accept an optional `window` parameter (entity ID) to target
//! a specific window. Defaults to the primary window.
//!
//! Button values: `"Left"`, `"Right"`, `"Middle"`, `"Back"`, `"Forward"`
//!
//! ### `brp_extras/click_mouse`
//! Performs a click (press and immediate release).
//! - `button` (string, required)
//! - `window` (u64, optional)
//!
//! ### `brp_extras/double_click_mouse`
//! Performs two rapid clicks with configurable delay.
//! - `button` (string, required)
//! - `delay_ms` (u32, optional, default: 250): delay between clicks
//! - `window` (u64, optional)
//!
//! ### `brp_extras/send_mouse_button`
//! Presses and holds a mouse button for a specified duration.
//! - `button` (string, required)
//! - `duration_ms` (u32, optional, default: 100, max: 60000)
//! - `window` (u64, optional)
//!
//! ### `brp_extras/move_mouse`
//! Moves the cursor by delta or to an absolute position. Exactly one must be provided.
//! - `delta` ([f32; 2], optional): relative movement
//! - `position` ([f32; 2], optional): absolute position
//! - `window` (u64, optional)
//!
//! ### `brp_extras/drag_mouse`
//! Performs a smooth drag with linear interpolation over a number of frames.
//! - `button` (string, required)
//! - `start` ([f32; 2], required): starting position
//! - `end` ([f32; 2], required): ending position
//! - `frames` (u32, required): number of frames to interpolate over
//! - `window` (u64, optional)
//!
//! ### `brp_extras/scroll_mouse`
//! Sends mouse wheel scroll events.
//! - `x` (f32, required): horizontal scroll amount
//! - `y` (f32, required): vertical scroll amount
//! - `unit` (string, required): `"Line"` or `"Pixel"`
//! - `window` (u64, optional)
//!
//! ## Trackpad Gestures (macOS)
//!
//! ### `brp_extras/double_tap_gesture`
//! Sends a double-tap gesture event. No parameters.
//!
//! ### `brp_extras/pinch_gesture`
//! Sends a pinch gesture for zoom operations.
//! - `delta` (f32, required): positive = zoom in, negative = zoom out
//!
//!
//! ### `brp_extras/rotation_gesture`
//! Sends a rotation gesture.
//! - `delta` (f32, required): rotation in radians

#[cfg(feature = "diagnostics")]
mod diagnostics;
mod keyboard;
mod mouse;
mod plugin;
mod screenshot;
mod shutdown;
mod window_event;
mod window_title;

pub use plugin::BrpExtrasPlugin;

/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;
