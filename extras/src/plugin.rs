//! Plugin implementation for extra BRP methods

use std::sync::Mutex;

#[cfg(feature = "diagnostics")]
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::PrimaryWindow;
use bevy_remote::RemoteMethodSystemId;
use bevy_remote::RemoteMethods;
use bevy_remote::RemotePlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy_remote::http::RemoteHttpPlugin;

#[cfg(not(target_arch = "wasm32"))]
use crate::DEFAULT_REMOTE_PORT;
use crate::constants::EXTRAS_COMMAND_PREFIX;
#[cfg(feature = "diagnostics")]
use crate::diagnostics;
use crate::keyboard;
use crate::mouse;
use crate::screenshot;
use crate::shutdown;
use crate::window_title;

// ---------------------------------------------------------------------------
// Port display configuration
// ---------------------------------------------------------------------------

/// Controls whether the BRP port is appended to the window title.
///
/// Used with [`BrpExtrasPlugin::port_in_title`] to display the port in the
/// primary window's title bar.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug)]
pub enum PortDisplay {
    /// Always append `(port: XXXXX)` to the window title.
    Always,
    /// Only append when not using the default port (15702).
    NonDefault,
}

// ---------------------------------------------------------------------------
// HTTP configuration state types
// ---------------------------------------------------------------------------

/// No HTTP configuration specified — uses `BRP_EXTRAS_PORT` env var or default port.
pub struct Unconfigured;

/// HTTP transport configured with an explicit port.
#[cfg(not(target_arch = "wasm32"))]
pub struct PortConfigured(u16);

/// HTTP transport configured with a user-provided `RemoteHttpPlugin`.
#[cfg(not(target_arch = "wasm32"))]
pub struct HttpPluginConfigured(Mutex<Option<RemoteHttpPlugin>>);

// ---------------------------------------------------------------------------
// Port resolution trait
// ---------------------------------------------------------------------------

/// Trait for HTTP configuration states that can resolve an effective port.
///
/// Implemented for [`Unconfigured`] and [`PortConfigured`] — both states where
/// `BrpExtrasPlugin` manages the HTTP transport and the port is knowable.
///
/// Not implemented for [`HttpPluginConfigured`] because the user provides their
/// own `RemoteHttpPlugin` and already knows the port they configured.
#[cfg(not(target_arch = "wasm32"))]
pub trait HasEffectivePort {
    /// The fallback port when `BRP_EXTRAS_PORT` env var is not set.
    fn fallback_port(&self) -> u16;

    /// Whether the port was explicitly configured via `with_port()`.
    fn is_explicit(&self) -> bool;
}

#[cfg(not(target_arch = "wasm32"))]
impl HasEffectivePort for Unconfigured {
    fn fallback_port(&self) -> u16 { DEFAULT_REMOTE_PORT }
    fn is_explicit(&self) -> bool { false }
}

#[cfg(not(target_arch = "wasm32"))]
impl HasEffectivePort for PortConfigured {
    fn fallback_port(&self) -> u16 { self.0 }
    fn is_explicit(&self) -> bool { true }
}

// ---------------------------------------------------------------------------
// Plugin struct and const shorthand
// ---------------------------------------------------------------------------

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
///
/// # HTTP transport configuration
///
/// On native targets, HTTP transport can be configured in three ways
/// (mutually exclusive, enforced at compile time):
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_brp_extras::BrpExtrasPlugin;
/// // 1. Default — uses BRP_EXTRAS_PORT env var or port 15702
/// App::new().add_plugins((DefaultPlugins, BrpExtrasPlugin::default()));
///
/// // 2. Explicit port
/// App::new().add_plugins((DefaultPlugins, BrpExtrasPlugin::with_port(9000)));
/// ```
///
/// ```ignore
/// // 3. Full control — provide your own RemoteHttpPlugin
/// App::new().add_plugins((DefaultPlugins, BrpExtrasPlugin::with_http_plugin(
///     RemoteHttpPlugin::default()
///         .with_port(9000)
///         .with_address([0, 0, 0, 0])
/// )));
/// ```
#[allow(
    non_upper_case_globals,
    reason = "const shares struct name for ergonomic plugin construction"
)]
pub const BrpExtrasPlugin: BrpExtrasPlugin = BrpExtrasPlugin::new();

/// Plugin type for adding extra BRP methods.
///
/// The `HttpConfig` type parameter controls how HTTP transport is configured.
/// See the [module-level documentation](BrpExtrasPlugin) for usage examples.
pub struct BrpExtrasPlugin<HttpConfig = Unconfigured> {
    http_config:  HttpConfig,
    #[cfg(not(target_arch = "wasm32"))]
    port_display: Option<PortDisplay>,
}

impl Default for BrpExtrasPlugin<Unconfigured> {
    fn default() -> Self { Self::new() }
}

impl BrpExtrasPlugin<Unconfigured> {
    /// Create a new plugin instance with default HTTP configuration.
    ///
    /// Uses `BRP_EXTRAS_PORT` environment variable if set, otherwise port 15702.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            http_config:                                      Unconfigured,
            #[cfg(not(target_arch = "wasm32"))]
            port_display:                                     None,
        }
    }

    /// Create plugin with a custom port (native only, ignored on WASM).
    ///
    /// The `BRP_EXTRAS_PORT` environment variable takes precedence if set.
    ///
    /// This is mutually exclusive with [`with_http_plugin`](Self::with_http_plugin)
    /// — the compiler enforces that only one can be used.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub const fn with_port(port: u16) -> BrpExtrasPlugin<PortConfigured> {
        BrpExtrasPlugin {
            http_config:  PortConfigured(port),
            port_display: None,
        }
    }

    /// Provide a fully configured `RemoteHttpPlugin` (native only).
    ///
    /// When using this method, `BrpExtrasPlugin` adds the provided plugin as-is.
    /// The `BRP_EXTRAS_PORT` environment variable and `with_port()` are not used.
    ///
    /// This is mutually exclusive with [`with_port`](Self::with_port)
    /// — the compiler enforces that only one can be used.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub const fn with_http_plugin(
        plugin: RemoteHttpPlugin,
    ) -> BrpExtrasPlugin<HttpPluginConfigured> {
        BrpExtrasPlugin {
            http_config:  HttpPluginConfigured(Mutex::new(Some(plugin))),
            port_display: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Port resolution
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
impl<H: HasEffectivePort> BrpExtrasPlugin<H> {
    /// Get the effective port that will be used for HTTP transport.
    ///
    /// Priority order:
    /// 1. `BRP_EXTRAS_PORT` environment variable (highest priority)
    /// 2. Configured port (via [`with_port`](BrpExtrasPlugin::with_port)) or default (15702)
    ///
    /// Returns `(port, source_description)` for logging.
    #[must_use]
    pub fn get_effective_port(&self) -> (u16, String) {
        let fallback = self.http_config.fallback_port();

        let env_port = std::env::var("BRP_EXTRAS_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok());

        let effective_port = env_port.unwrap_or(fallback);

        let explicit = self.http_config.is_explicit();

        let source_description = match (env_port, explicit) {
            (Some(_), false) => {
                format!("environment override from default {DEFAULT_REMOTE_PORT}")
            },
            (Some(_), true) => {
                format!("environment override from with_port {fallback}")
            },
            (None, false) => "default".to_string(),
            (None, true) => "with_port".to_string(),
        };

        (effective_port, source_description)
    }

    /// Append `(port: XXXXX)` to the primary window's title at startup.
    ///
    /// - [`PortDisplay::Always`] — always appends the port
    /// - [`PortDisplay::NonDefault`] — only appends when the effective port differs from the
    ///   default (15702)
    ///
    /// If not called, the window title is left unchanged.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_brp_extras::BrpExtrasPlugin;
    /// # use bevy_brp_extras::PortDisplay;
    /// App::new()
    ///     .add_plugins(DefaultPlugins)
    ///     .add_plugins(BrpExtrasPlugin::with_port(9000).port_in_title(PortDisplay::NonDefault))
    ///     .run();
    /// ```
    #[must_use]
    pub const fn port_in_title(mut self, display: PortDisplay) -> Self {
        self.port_display = Some(display);
        self
    }
}

// ---------------------------------------------------------------------------
// Plugin implementations
// ---------------------------------------------------------------------------

impl Plugin for BrpExtrasPlugin<Unconfigured> {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            add_managed_http_transport(app, None);
            maybe_add_port_title_system(app, &self.http_config, self.port_display);
        }

        build_shared(app);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Plugin for BrpExtrasPlugin<PortConfigured> {
    fn build(&self, app: &mut App) {
        add_managed_http_transport(app, Some(self.http_config.0));
        maybe_add_port_title_system(app, &self.http_config, self.port_display);
        build_shared(app);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Plugin for BrpExtrasPlugin<HttpPluginConfigured> {
    fn build(&self, app: &mut App) {
        let Some(plugin) = self
            .http_config
            .0
            .lock()
            .ok()
            .and_then(|mut guard| guard.take())
        else {
            error!("failed to retrieve `RemoteHttpPlugin` configuration");
            build_shared(app);
            return;
        };

        if app.is_plugin_added::<RemoteHttpPlugin>() {
            warn!(
                "`RemoteHttpPlugin` is already added — the `RemoteHttpPlugin` provided to \
                 `BrpExtrasPlugin::with_http_plugin()` will be ignored. The existing HTTP \
                 transport will be used as-is."
            );
        } else {
            app.add_plugins(plugin);
        }

        build_shared(app);
    }
}

// ---------------------------------------------------------------------------
// Shared build logic
// ---------------------------------------------------------------------------

/// Common plugin setup shared across all HTTP configuration states.
fn build_shared(app: &mut App) {
    // Add `RemotePlugin` if not already present
    if !app.is_plugin_added::<RemotePlugin>() {
        app.add_plugins(RemotePlugin::default());
    }

    // Register extras methods into the existing `RemoteMethods` resource
    register_extras_methods(app.world_mut());

    // Defensively add `FrameTimeDiagnosticsPlugin` if not already installed
    #[cfg(feature = "diagnostics")]
    if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
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

/// Add managed HTTP transport, using env var / optional port / default.
#[cfg(not(target_arch = "wasm32"))]
fn add_managed_http_transport(app: &mut App, configured_port: Option<u16>) {
    if app.is_plugin_added::<RemoteHttpPlugin>() {
        warn!(
            "`RemoteHttpPlugin` is already added — `BrpExtrasPlugin` port configuration \
             (with_port / BRP_EXTRAS_PORT) will be ignored. The existing HTTP transport \
             will be used as-is."
        );
        return;
    }

    let env_port = std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok());

    let effective_port = env_port.unwrap_or_else(|| configured_port.unwrap_or(DEFAULT_REMOTE_PORT));

    let source_description = match (env_port, configured_port) {
        (Some(_), Some(with_port_value)) => {
            format!("environment override from with_port {with_port_value}")
        },
        (Some(_), None) => {
            format!("environment override from default {DEFAULT_REMOTE_PORT}")
        },
        (None, Some(_)) => "with_port".to_string(),
        (None, None) => "default".to_string(),
    };

    let http_plugin = RemoteHttpPlugin::default().with_port(effective_port);
    app.add_plugins(http_plugin);
    app.add_systems(Startup, move |_world: &mut World| {
        log_initialization(effective_port, &source_description);
    });
}

/// Register all extras BRP methods into the world's `RemoteMethods` resource.
fn register_extras_methods(world: &mut World) {
    let mut methods = vec![
        (
            format!("{EXTRAS_COMMAND_PREFIX}click_mouse"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::click_mouse_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}double_click_mouse"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::double_click_mouse_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}double_tap_gesture"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::double_tap_gesture_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}drag_mouse"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::drag_mouse_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}move_mouse"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::move_mouse_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}pinch_gesture"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::pinch_gesture_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}rotation_gesture"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::rotation_gesture_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}screenshot"),
            RemoteMethodSystemId::Instant(world.register_system(screenshot::handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}scroll_mouse"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::scroll_mouse_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}send_keys"),
            RemoteMethodSystemId::Instant(world.register_system(keyboard::send_keys_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}send_mouse_button"),
            RemoteMethodSystemId::Instant(world.register_system(mouse::send_mouse_button_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}set_window_title"),
            RemoteMethodSystemId::Instant(world.register_system(window_title::handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}shutdown"),
            RemoteMethodSystemId::Instant(world.register_system(shutdown::handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}type_text"),
            RemoteMethodSystemId::Instant(world.register_system(keyboard::type_text_handler)),
        ),
    ];

    #[cfg(feature = "diagnostics")]
    methods.push((
        format!("{EXTRAS_COMMAND_PREFIX}get_diagnostics"),
        RemoteMethodSystemId::Instant(world.register_system(diagnostics::handler)),
    ));

    let mut remote_methods = world.resource_mut::<RemoteMethods>();
    for (name, system_id) in methods {
        remote_methods.insert(name, system_id);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn log_initialization(port: u16, source_description: &str) {
    info!("BRP extras enabled on http://localhost:{port} ({source_description})");
}

/// Conditionally adds a `Startup` system that appends the port to the primary
/// window's title, based on the [`PortDisplay`] policy.
#[cfg(not(target_arch = "wasm32"))]
fn maybe_add_port_title_system(
    app: &mut App,
    http_config: &impl HasEffectivePort,
    port_display: Option<PortDisplay>,
) {
    let Some(display) = port_display else {
        return;
    };

    let fallback = http_config.fallback_port();

    let env_port = std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok());

    let effective_port = env_port.unwrap_or(fallback);

    let should_display = match display {
        PortDisplay::Always => true,
        PortDisplay::NonDefault => effective_port != DEFAULT_REMOTE_PORT,
    };

    if should_display {
        app.add_systems(
            Startup,
            move |mut query: Query<&mut Window, With<PrimaryWindow>>| {
                if let Ok(mut window) = query.single_mut() {
                    window.title = format!("{} (port: {effective_port})", window.title);
                }
            },
        );
    }
}
