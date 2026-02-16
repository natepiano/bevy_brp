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
//! This will add the following BRP methods to your app:
//! - `brp_extras/screenshot`: Capture a screenshot
//! - `brp_extras/shutdown`: Gracefully shutdown the app
//! - `brp_extras/send_keys`: Send keyboard input
//! - `brp_extras/set_window_title`: Change the window title

mod keyboard;
mod mouse;
mod plugin;
mod screenshot;
mod shutdown;
mod window_title;

pub use plugin::BrpExtrasPlugin;

/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;
