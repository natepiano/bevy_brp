//! Plugin implementation for extra BRP methods

use bevy::prelude::*;
use bevy::remote::RemotePlugin;
use bevy::remote::http::RemoteHttpPlugin;

use crate::{DEFAULT_REMOTE_PORT, debug_mode, discovery, keyboard, screenshot, shutdown};

/// Command prefix for `brp_extras` methods
const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";

/// Plugin that adds extra BRP methods to a Bevy app
///
/// Currently provides:
/// - `brp_extras/screenshot`: Capture screenshots
/// - `brp_extras/shutdown`: Gracefully shutdown the app
/// - `brp_extras/discover_format`: Discover component format information
/// - `brp_extras/send_keys`: Send keyboard input
/// - `brp_extras/set_debug_mode`: Enable/disable debug information in responses
#[allow(non_upper_case_globals)]
pub const BrpExtrasPlugin: BrpExtrasPlugin = BrpExtrasPlugin::new();

/// Plugin type for adding extra BRP methods
pub struct BrpExtrasPlugin {
    port: Option<u16>,
}

impl Default for BrpExtrasPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BrpExtrasPlugin {
    /// Create a new plugin instance with default port
    #[must_use]
    pub const fn new() -> Self {
        Self { port: None }
    }

    /// Create plugin with custom port
    #[must_use]
    pub const fn with_port(port: u16) -> Self {
        Self { port: Some(port) }
    }

    /// Get the effective port, checking environment variable first
    ///
    /// Priority order:
    /// 1. `BRP_PORT` environment variable (highest priority)
    /// 2. Explicitly set port via `with_port()`
    /// 3. Default port (15702)
    #[must_use]
    pub fn get_effective_port(&self) -> (u16, String) {
        let env_port = std::env::var("BRP_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok());

        let final_port = env_port.unwrap_or_else(|| self.port.unwrap_or(DEFAULT_REMOTE_PORT));

        let source_description = match (env_port, self.port) {
            (Some(_), Some(with_port_value)) => {
                format!("environment override from with_port {with_port_value}")
            }
            (Some(_), None) => {
                format!("environment override from default {DEFAULT_REMOTE_PORT}")
            }
            (None, Some(_)) => "with_port".to_string(),
            (None, None) => "default".to_string(),
        };

        (final_port, source_description)
    }
}

impl Plugin for BrpExtrasPlugin {
    fn build(&self, app: &mut App) {
        // Get the effective port and source description
        let (effective_port, source_description) = self.get_effective_port();

        // Add Bevy's remote plugins with our custom methods
        info!(
            "Registering BRP extras methods with prefix: {}",
            EXTRAS_COMMAND_PREFIX
        );

        let remote_plugin = RemotePlugin::default()
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}screenshot"),
                screenshot::handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}shutdown"),
                shutdown::handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}discover_format"),
                discovery::factual_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}send_keys"),
                keyboard::send_keys_handler,
            )
            .with_method(
                format!("{EXTRAS_COMMAND_PREFIX}set_debug_mode"),
                debug_mode::handler,
            );

        let http_plugin = RemoteHttpPlugin::default().with_port(effective_port);

        app.add_plugins((remote_plugin, http_plugin));

        // Add the system to process timed key releases
        app.add_systems(Update, keyboard::process_timed_key_releases);

        // Add the system to handle deferred shutdown
        app.add_systems(Update, shutdown::deferred_shutdown_system);

        app.add_systems(Startup, move |_world: &mut World| {
            log_initialization(effective_port, &source_description);
        });
    }
}

fn log_initialization(port: u16, source_description: &str) {
    info!("BRP extras enabled on http://localhost:{port} ({source_description})");
    trace!("Additional BRP methods available:");
    trace!("  - brp_extras/screenshot - Take a screenshot");
    trace!("  - brp_extras/shutdown - Shutdown the app");
    trace!("  - brp_extras/discover_format - Discover component format information");
    trace!("  - brp_extras/send_keys - Send keyboard input");
    trace!("  - brp_extras/set_debug_mode - Enable/disable debug information");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_precedence_default() {
        unsafe { std::env::remove_var("BRP_PORT") };
        let plugin = BrpExtrasPlugin::new();
        let (port, description) = plugin.get_effective_port();
        assert_eq!(port, DEFAULT_REMOTE_PORT);
        assert_eq!(description, "default");
    }

    #[test]
    fn test_port_precedence_with_port() {
        unsafe { std::env::remove_var("BRP_PORT") };
        let plugin = BrpExtrasPlugin::with_port(8080);
        let (port, description) = plugin.get_effective_port();
        assert_eq!(port, 8080);
        assert_eq!(description, "with_port");
    }

    #[test]
    fn test_port_precedence_env_override_default() {
        unsafe { std::env::set_var("BRP_PORT", "25000") };
        let plugin = BrpExtrasPlugin::new();
        let (port, description) = plugin.get_effective_port();
        assert_eq!(port, 25000);
        assert_eq!(description, "environment override from default 15702");
        unsafe { std::env::remove_var("BRP_PORT") };
    }

    #[test]
    fn test_port_precedence_env_override_with_port() {
        unsafe { std::env::set_var("BRP_PORT", "25000") };
        let plugin = BrpExtrasPlugin::with_port(8080);
        let (port, description) = plugin.get_effective_port();
        assert_eq!(port, 25000);
        assert_eq!(description, "environment override from with_port 8080");
        unsafe { std::env::remove_var("BRP_PORT") };
    }

    #[test]
    fn test_port_precedence_invalid_env_var() {
        unsafe { std::env::set_var("BRP_PORT", "invalid") };
        let plugin = BrpExtrasPlugin::with_port(8080);
        let (port, description) = plugin.get_effective_port();
        assert_eq!(port, 8080);
        assert_eq!(description, "with_port");
        unsafe { std::env::remove_var("BRP_PORT") };
    }
}
