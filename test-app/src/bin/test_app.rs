//! Test app with `BrpExtrasPlugin` for testing app launch and extras functionality

use bevy::log::debug;
use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PrimaryWindow;
use bevy::window::WindowPosition;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

const BRP_EXTRAS_PORT_DEFAULT: &str = "15702";
const BRP_EXTRAS_PORT_ENV_VAR: &str = "BRP_EXTRAS_PORT";
const MARKER_FLAG: &str = "--marker";
const SPRITE_COLOR: Color = Color::srgb(0.5, 0.7, 0.9);
const SPRITE_SIZE: Vec2 = Vec2::new(100.0, 100.0);
const TEXT_FONT_SIZE: f32 = 24.0;
const TEXT_POSITION: Vec3 = Vec3::new(-100.0, 120.0, 0.0);
const TEST_APP_TITLE: &str = "Test Extras Plugin App";
const WINDOW_HEIGHT: u32 = 300;
const WINDOW_WIDTH: u32 = 400;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: TEST_APP_TITLE.to_string(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                focused: false,
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BrpExtrasPlugin::new().port_in_title(PortDisplay::Always))
        .add_systems(Startup, (setup, minimize_window_on_start, log_startup))
        .add_systems(Update, rotate_sprite)
        .run();
}

#[derive(Component)]
struct Rotator {
    speed: f32,
}

/// Minimize the window immediately on startup
fn minimize_window_on_start(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d);

    // Simple sprite that rotates
    commands.spawn((
        Sprite {
            color: SPRITE_COLOR,
            custom_size: Some(SPRITE_SIZE),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Rotator { speed: 1.0 },
    ));

    // Text showing app name
    commands.spawn((
        Text::new(TEST_APP_TITLE),
        TextFont {
            font_size: FontSize::Px(TEXT_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(TEXT_POSITION),
    ));
}

fn log_startup() {
    let port = std::env::var(BRP_EXTRAS_PORT_ENV_VAR)
        .unwrap_or_else(|_| BRP_EXTRAS_PORT_DEFAULT.to_string());
    debug!("test_app starting on port {port}");

    // Log --marker value if provided (used by args integration test)
    let command_line_arguments: Vec<String> = std::env::args().collect();
    if let Some(pos) = command_line_arguments.iter().position(|a| a == MARKER_FLAG)
        && let Some(value) = command_line_arguments.get(pos + 1)
    {
        info!("MARKER:{value}");
    }
}

fn rotate_sprite(time: Res<Time>, mut query: Query<(&mut Transform, &Rotator)>) {
    for (mut transform, rotator) in &mut query {
        transform.rotate_z(rotator.speed * time.delta_secs());
    }
}
