//! Minimal BRP example named `test_app` for `search_order` integration testing.
//!
//! This example intentionally shares its name with the `test_app` binary in the
//! `bevy_brp_test_apps` package (`test-app/src/bin/test_app.rs`). The cross-package
//! name collision lets integration tests verify that `brp_launch`'s `search_order`
//! parameter correctly prioritizes apps vs examples:
//!
//!   - `search_order="app"`     → launches the binary (`launched_as: "app"`)
//!   - `search_order="example"` → launches this example (`launched_as: "example"`)
//!
//! The two targets produce no output collision because Cargo places binaries in
//! `target/<profile>/test_app` and examples in `target/<profile>/examples/test_app`.

use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PrimaryWindow;
use bevy::window::WindowPosition;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

const MARKER_FLAG: &str = "--marker";
const SEARCH_ORDER_EXAMPLE_TEXT: &str = "search_order test example";
const SEARCH_ORDER_FONT_SIZE: f32 = 24.0;
const WINDOW_HEIGHT: u32 = 300;
const WINDOW_WIDTH: u32 = 400;

fn main() {
    let brp_extras_plugin = BrpExtrasPlugin::new().port_in_title(PortDisplay::Always);
    let (port, _) = brp_extras_plugin.get_effective_port();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("search_order test example - Port {port}"),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                focused: false,
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_extras_plugin)
        .add_systems(Startup, (setup, minimize_window_on_start))
        .run();
}

/// Minimize the window immediately on startup (no-op on Linux/Wayland)
#[cfg(target_os = "linux")]
fn minimize_window_on_start(windows: Query<&mut Window, With<PrimaryWindow>>) {
    let _ = windows.iter().count();
}

/// Minimize the window immediately on startup
#[cfg(not(target_os = "linux"))]
fn minimize_window_on_start(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new(SEARCH_ORDER_EXAMPLE_TEXT),
        TextFont {
            font_size: FontSize::Px(SEARCH_ORDER_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
    ));

    // Log --marker value if provided (used by args integration test)
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == MARKER_FLAG)
        && let Some(value) = args.get(pos + 1)
    {
        info!("MARKER:{value}");
    }
}
