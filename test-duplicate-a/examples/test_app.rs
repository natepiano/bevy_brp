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
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    let brp_plugin = BrpExtrasPlugin::new();
    let (port, _) = brp_plugin.get_effective_port();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("search_order test example - Port {port}"),
                resolution: (400, 300).into(),
                focused: false,
                position: bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Primary,
                ),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_plugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("search_order test example"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}
