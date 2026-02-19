//! Plugin implementation for extra BRP methods

use bevy::prelude::*;
use bevy_remote::RemotePlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy_remote::http::RemoteHttpPlugin;

#[cfg(not(target_arch = "wasm32"))]
use crate::DEFAULT_REMOTE_PORT;
use crate::keyboard;
use crate::mouse;
use crate::screenshot;
use crate::shutdown;
use crate::window_title;

/// Command prefix for `brp_extras` methods
const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";

/// Plugin that adds extra BRP methods to a Bevy app
///
/// Currently provides:
/// - `brp_extras/screenshot`: Capture screenshots
/// - `brp_extras/shutdown`: Gracefully shutdown the app
/// - `brp_extras/send_keys`: Send keyboard input
/// - `brp_extras/set_window_title`: Change the window title
///
/// On native targets, this also adds `RemoteHttpPlugin` for HTTP transport.
/// On WASM, only the methods are registered - you need to add your own
/// transport (e.g. a WebSocket relay).
#[allow(non_upper_case_globals)]
pub const BrpExtrasPlugin: BrpExtrasPlugin = BrpExtrasPlugin::new();

/// Plugin type for adding extra BRP methods
pub struct BrpExtrasPlugin {
    #[cfg(not(target_arch = "wasm32"))]
    port: Option<u16>,
}

impl Default for BrpExtrasPlugin {
    fn default() -> Self { Self::new() }
}

impl BrpExtrasPlugin {
    /// Create a new plugin instance with default port
    #[must_use]
    pub const fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            port: None,
        }
    }

    /// Create plugin with custom port (native only, ignored on WASM)
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub const fn with_port(port: u16) -> Self { Self { port: Some(port) } }

    /// Get the effective port, checking environment variable first
    ///
    /// Priority order:
    /// 1. `BRP_EXTRAS_PORT` environment variable (highest priority)
    /// 2. Explicitly set port via `with_port()`
    /// 3. Default port (15702)
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn get_effective_port(&self) -> (u16, String) {
        let env_port = std::env::var("BRP_EXTRAS_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok());

        let final_port = env_port.unwrap_or_else(|| self.port.unwrap_or(DEFAULT_REMOTE_PORT));

        let source_description = match (env_port, self.port) {
            (Some(_), Some(with_port_value)) => {
                format!("environment override from with_port {with_port_value}")
            },
            (Some(_), None) => {
                format!("environment override from default {DEFAULT_REMOTE_PORT}")
            },
            (None, Some(_)) => "with_port".to_string(),
            (None, None) => "default".to_string(),
        };

        (final_port, source_description)
    }
}

impl Plugin for BrpExtrasPlugin {
    fn build(&self, app: &mut App) {
        let remote_plugin = RemotePlugin::default()
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}click_mouse"),
                mouse::click_mouse_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}double_click_mouse"),
                mouse::double_click_mouse_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}double_tap_gesture"),
                mouse::double_tap_gesture_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}drag_mouse"),
                mouse::drag_mouse_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}move_mouse"),
                mouse::move_mouse_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}pinch_gesture"),
                mouse::pinch_gesture_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}rotation_gesture"),
                mouse::rotation_gesture_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}screenshot"),
                screenshot::handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}scroll_mouse"),
                mouse::scroll_mouse_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}send_keys"),
                keyboard::send_keys_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}send_mouse_button"),
                mouse::send_mouse_button_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}set_window_title"),
                window_title::handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}shutdown"),
                shutdown::handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}type_text"),
                keyboard::type_text_handler,
            );

        // On native, add HTTP transport. On WASM, the user adds their own transport.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (effective_port, source_description) = self.get_effective_port();
            let http_plugin = RemoteHttpPlugin::default().with_port(effective_port);
            app.add_plugins((remote_plugin, http_plugin));
            app.add_systems(Startup, move |_world: &mut World| {
                log_initialization(effective_port, &source_description);
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            app.add_plugins(remote_plugin);
        }

        // Initialize mouse resources
        app.init_resource::<mouse::SimulatedCursorPosition>();

        // Add systems for keyboard input simulation
        app.add_systems(Update, keyboard::process_timed_key_releases);
        app.add_systems(Update, keyboard::process_text_typing);

        // Add systems for mouse input simulation
        app.add_systems(Update, mouse::sync_cursor_position);
        app.add_systems(Update, mouse::process_timed_button_releases);
        app.add_systems(Update, mouse::process_scheduled_clicks);
        app.add_systems(Update, mouse::process_drag_operations);

        // Add the system to handle deferred shutdown
        app.add_systems(Update, shutdown::deferred_shutdown_system);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn log_initialization(port: u16, source_description: &str) {
    info!("BRP extras enabled on http://localhost:{port} ({source_description})");
}
