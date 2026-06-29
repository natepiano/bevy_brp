//! BRP test example WITHOUT `bevy_brp_extras` plugin
//!
//! This example demonstrates basic BRP functionality without extras plugin.
//! Used for testing fallback behavior when `bevy_brp_extras` is not available.
//!
//! Run with: cargo run --example `no_extras_plugin`

use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PrimaryWindow;
use bevy::window::WindowPlugin;
use bevy::window::WindowPosition;
use bevy_remote::RemotePlugin;
use bevy_remote::http::RemoteHttpPlugin;

const ENTITY_ONE_NAME: &str = "TestEntity1";
const ENTITY_TWO_NAME: &str = "TestEntity2";
/// Hard-coded port for this example (to avoid conflicts)
const FIXED_PORT: u16 = 25000;
const STATUS_FONT_SIZE: f32 = 24.0;
const STATUS_LEFT: f32 = 20.0;
const STATUS_TOP: f32 = 20.0;
const TEST_ENTITY_ONE_TRANSLATION: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const TEST_ENTITY_TWO_TRANSLATION: Vec3 = Vec3::new(10.0, 20.0, 30.0);
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_WIDTH: u32 = 800;

fn main() {
    info!("Starting BRP No Plugin Test on port {FIXED_PORT}");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("BRP No Plugin Test - Port {FIXED_PORT}"),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                focused: false,
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default().with_port(FIXED_PORT),
        ))
        .add_systems(
            Startup,
            (setup_test_entities, setup_ui, minimize_window_on_start),
        )
        .run();
}

/// Minimize the window immediately on startup
fn minimize_window_on_start(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}

/// Setup test entities for BRP testing
fn setup_test_entities(mut commands: Commands) {
    info!("Setting up test entities...");

    // Basic entity with Transform and Name
    commands.spawn((
        Transform::from_translation(TEST_ENTITY_ONE_TRANSLATION),
        Name::new(ENTITY_ONE_NAME),
    ));

    // Entity with different transform
    commands.spawn((
        Transform::from_translation(TEST_ENTITY_TWO_TRANSLATION),
        Name::new(ENTITY_TWO_NAME),
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
            font_size: FontSize::Px(STATUS_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(STATUS_LEFT),
            top: Val::Px(STATUS_TOP),
            ..default()
        },
    ));
}
