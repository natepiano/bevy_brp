//! Test app with `BrpExtrasPlugin` for testing app launch and extras functionality

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    let port = std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(15702);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Test Extras Plugin App - BRP Port {port}"),
                resolution: (400.0, 300.0).into(),
                focused: false,
                position: bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Primary,
                ),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BrpExtrasPlugin)
        .add_systems(Startup, (setup, minimize_window_on_start))
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
            color: Color::srgb(0.5, 0.7, 0.9),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Rotator { speed: 1.0 },
    ));

    // Text showing app name
    commands.spawn((
        Text::new("Test Extras Plugin App"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(-100.0, 120.0, 0.0),
    ));
}

fn rotate_sprite(time: Res<Time>, mut query: Query<(&mut Transform, &Rotator)>) {
    for (mut transform, rotator) in &mut query {
        transform.rotate_z(rotator.speed * time.delta_secs());
    }
}
