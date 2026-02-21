//! Minimal WASM test app for `bevy_brp_extras`
//!
//! Validates that `BrpExtrasPlugin` compiles and runs on `wasm32-unknown-unknown`.
//! Uses `BrpWebSocketRelayPlugin` to bridge BRP requests from the relay server
//! so the MCP tool can make BRP calls against the running WASM app.

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_websocket_relay::BrpWebSocketRelayPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin)
        .add_plugins(BrpWebSocketRelayPlugin::default())
        .register_type::<Name>()
        .register_type::<Transform>()
        .add_systems(Startup, setup)
        .run();
}

/// Spawn entities to confirm the app is running and BRP can query them
fn setup(mut commands: Commands) {
    info!("BRP Extras WASM test app started");

    // Simple entity with Transform + Name for BRP query testing
    commands.spawn((
        Transform::from_xyz(1.0, 2.0, 3.0),
        Name::new("wasm-brp-test"),
    ));

    commands.spawn(Camera2d);
}
