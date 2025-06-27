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
//! - `brp_extras/discover_format`: Discover component format information
//! - `brp_extras/send_keys`: Send keyboard input

mod debug_mode;
mod discovery;
mod format;
mod keyboard;

mod plugin;
mod screenshot;
mod shutdown;

pub use discovery::{
    discover_component_format, discover_multiple_formats, get_common_component_types,
};
pub use format::{FieldInfo, FormatInfo, MutationInfo, SpawnInfo};
pub use keyboard::{
    KeyCodeInfo, KeyCodeWrapper, SendKeysRequest, SendKeysResponse, TimedKeyRelease,
};
pub use plugin::BrpExtrasPlugin;

/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;
