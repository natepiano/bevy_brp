//! BRP test example WITHOUT `bevy_brp_extras` plugin
//!
//! This example demonstrates basic BRP functionality without extras plugin.
//! Used for testing fallback behavior when `bevy_brp_extras` is not available.
//!
//! Run with: cargo run --example `no_extras_plugin`

use std::env;

use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PrimaryWindow;
use bevy::window::WindowPlugin;
use bevy::window::WindowPosition;
use bevy_remote::RemotePlugin;
use bevy_remote::http::RemoteHttpPlugin;

const BRP_EXTRAS_PORT_ENV: &str = "BRP_EXTRAS_PORT";
const DUPLICATE_NAME: &str = "NoExtrasDuplicate";
const ENTITY_ONE_NAME: &str = "TestEntity1";
const ENTITY_TWO_NAME: &str = "TestEntity2";
const FALLBACK_PORT: u16 = 25000;
const NATES_LIST_NAME: &str = "NatesList";
const STATUS_FONT_SIZE: f32 = 24.0;
const STATUS_LEFT: f32 = 20.0;
const STATUS_TOP: f32 = 20.0;
const TEST_ENTITY_ONE_TRANSLATION: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const TEST_ENTITY_TWO_TRANSLATION: Vec3 = Vec3::new(10.0, 20.0, 30.0);
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_WIDTH: u32 = 800;

fn main() {
    let port = match configured_port() {
        Ok(port) => port,
        Err(error) => {
            eprintln!("Cannot start BRP No Plugin Test: {error}");
            return;
        },
    };

    info!("Starting BRP No Plugin Test on port {port}");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("BRP No Plugin Test - Port {port}"),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                focused: false,
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default().with_port(port),
        ))
        .insert_resource(CurrentPort(port))
        .add_systems(
            Startup,
            (setup_test_entities, setup_ui, minimize_window_on_start),
        )
        .run();
}

#[derive(Resource)]
struct CurrentPort(u16);

fn configured_port() -> Result<u16, String> {
    match env::var(BRP_EXTRAS_PORT_ENV) {
        Ok(value) => value.parse::<u16>().map_err(|error| {
            format!("{BRP_EXTRAS_PORT_ENV} must be a valid u16 port, got {value:?}: {error}")
        }),
        Err(env::VarError::NotPresent) => Ok(FALLBACK_PORT),
        Err(env::VarError::NotUnicode(_)) => {
            Err(format!("{BRP_EXTRAS_PORT_ENV} must contain Unicode digits"))
        },
    }
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

    commands.spawn(Name::new(NATES_LIST_NAME));
    commands.spawn(Name::new(DUPLICATE_NAME));
    commands.spawn(Name::new(DUPLICATE_NAME));
    commands.spawn(Transform::default());

    info!("Test entities spawned");
}

/// Setup minimal UI
fn setup_ui(mut commands: Commands, port: Res<CurrentPort>) {
    // Camera for rendering
    commands.spawn(Camera2d);

    // Simple text showing app status
    commands.spawn((
        Text::new(format!(
            "BRP No Plugin Test\nPort: {}\n\nBasic BRP only (no extras)",
            port.0
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
