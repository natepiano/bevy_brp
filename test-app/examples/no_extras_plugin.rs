//! BRP test example WITHOUT `bevy_brp_extras` plugin
//!
//! This example demonstrates basic BRP functionality without extras plugin.
//! Used for testing fallback behavior when `bevy_brp_extras` is not available.
//!
//! Run with: cargo run --example `no_extras_plugin`

use bevy::prelude::*;
use bevy::remote::RemotePlugin;
use bevy::remote::http::RemoteHttpPlugin;

/// Hard-coded port for this example (to avoid conflicts)
const FIXED_PORT: u16 = 25000;

fn main() {
    info!("Starting BRP No Plugin Test on port {FIXED_PORT}");

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
            primary_window: Some(bevy::window::Window {
                title: format!("BRP No Plugin Test - Port {FIXED_PORT}"),
                resolution: (800.0, 600.0).into(),
                window_level: bevy::window::WindowLevel::AlwaysOnBottom,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default().with_port(FIXED_PORT),
        ))
        .add_systems(Startup, (setup_test_entities, setup_ui))
        .run();
}

/// Setup test entities for BRP testing
fn setup_test_entities(mut commands: Commands) {
    info!("Setting up test entities...");

    // Basic entity with Transform and Name
    commands.spawn((Transform::from_xyz(1.0, 2.0, 3.0), Name::new("TestEntity1")));

    // Entity with different transform
    commands.spawn((
        Transform::from_xyz(10.0, 20.0, 30.0),
        Name::new("TestEntity2"),
    ));

    info!("Test entities spawned. BRP server running on http://localhost:{FIXED_PORT}");
}

/// Setup minimal UI
fn setup_ui(mut commands: Commands) {
    // Camera for rendering
    commands.spawn(Camera2d);

    // Simple text showing app status
    commands.spawn((
        Text::new(format!(
            "BRP No Plugin Test\nPort: {FIXED_PORT}\n\nBasic BRP only (no extras)"
        )),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
    ));
}
