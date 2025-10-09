//! BRP extras test example with keyboard input display
//!
//! This example demonstrates `bevy_brp_extras` functionality including:
//! - Format discovery
//! - Screenshot capture
//! - Keyboard input simulation
//! - Debug mode toggling
//!
//! Used by the test suite to validate all extras functionality.

use std::time::Instant;

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

/// Resource to track keyboard input history
#[derive(Resource, Default)]
struct KeyboardInputHistory {
    /// Currently pressed keys
    active_keys: Vec<String>,
    /// Last pressed keys (for display after release)
    last_keys: Vec<String>,
    /// Active modifier keys
    modifiers: Vec<String>,
    /// Time when the last key was pressed
    press_time: Option<Instant>,
    /// Duration between press and release in milliseconds
    last_duration_ms: Option<u64>,
    /// Whether the last key press has completed
    completed: bool,
}

/// Marker component for the keyboard input display text
#[derive(Component)]
struct KeyboardDisplayText;

fn main() {
    let brp_plugin = BrpExtrasPlugin::new();
    let (port, _) = brp_plugin.get_effective_port();

    info!("Starting BRP Extras Test on port {}", port);

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
            primary_window: Some(bevy::window::Window {
                title: format!("BRP Extras Test - Port {port}"),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_plugin)
        .init_resource::<KeyboardInputHistory>()
        .insert_resource(CurrentPort(port))
        .add_systems(Startup, (setup_test_entities, setup_ui))
        .add_systems(Update, (track_keyboard_input, update_keyboard_display))
        .run();
}

/// Resource to store the current port
#[derive(Resource)]
struct CurrentPort(u16);

/// Setup test entities for format discovery
fn setup_test_entities(mut commands: Commands, port: Res<CurrentPort>) {
    info!("Setting up test entities...");

    // Entity with Transform and Name
    commands.spawn((Transform::from_xyz(1.0, 2.0, 3.0), Name::new("TestEntity1")));

    // Entity with scaled transform
    commands.spawn((
        Transform::from_scale(Vec3::splat(2.0)),
        Name::new("ScaledEntity"),
    ));

    // Entity with complex transform
    commands.spawn((
        Transform {
            translation: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::from_rotation_y(std::f32::consts::PI / 4.0),
            scale: Vec3::new(0.5, 1.5, 2.0),
        },
        Name::new("ComplexTransformEntity"),
    ));

    // Entity with visibility component
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("VisibleEntity"),
        Visibility::default(),
    ));

    info!(
        "Test entities spawned. BRP server running on http://localhost:{}",
        port.0
    );
}

/// Setup UI for keyboard input display
fn setup_ui(mut commands: Commands, port: Res<CurrentPort>) {
    // Camera
    commands.spawn(Camera2d);

    // Background
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .with_children(|parent| {
            // Text container
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                ))
                .with_children(|parent| {
                    // Keyboard display text
                    parent.spawn((
                        Text::new(format!(
                            "Waiting for keyboard input...\n\nUse curl to send keys:\ncurl -X POST http://localhost:{}/brp_extras/send_keys \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"keys\": [\"KeyA\", \"Space\"]}}'",
                            port.0
                        )),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        KeyboardDisplayText,
                    ));
                });
        });
}

/// Track keyboard input events
#[allow(clippy::assigning_clones)] // clone_from doesn't work due to borrow checker
fn track_keyboard_input(
    mut events: MessageReader<KeyboardInput>,
    mut history: ResMut<KeyboardInputHistory>,
) {
    for event in events.read() {
        let key_str = format!("{:?}", event.key_code);

        match event.state {
            bevy::input::ButtonState::Pressed => {
                info!("Key pressed: {key_str}");
                history.completed = false;
                history.press_time = Some(Instant::now());

                if !history.active_keys.contains(&key_str) {
                    history.active_keys.push(key_str.clone());
                }

                // Track modifiers
                if key_str.contains("Control") && !history.modifiers.contains(&"Ctrl".to_string()) {
                    history.modifiers.push("Ctrl".to_string());
                } else if key_str.contains("Shift")
                    && !history.modifiers.contains(&"Shift".to_string())
                {
                    history.modifiers.push("Shift".to_string());
                } else if key_str.contains("Alt") && !history.modifiers.contains(&"Alt".to_string())
                {
                    history.modifiers.push("Alt".to_string());
                }
            }
            bevy::input::ButtonState::Released => {
                info!("Key released: {key_str}");

                if let Some(press_time) = history.press_time {
                    let duration = Instant::now().duration_since(press_time);
                    history.last_duration_ms = duration.as_millis().try_into().ok();
                }

                history.active_keys.retain(|k| k != &key_str);

                // Update modifiers
                if key_str.contains("Control") {
                    history.modifiers.retain(|m| m != "Ctrl");
                } else if key_str.contains("Shift") {
                    history.modifiers.retain(|m| m != "Shift");
                } else if key_str.contains("Alt") {
                    history.modifiers.retain(|m| m != "Alt");
                }

                if history.active_keys.is_empty() && !history.last_keys.is_empty() {
                    history.completed = true;
                }
            }
        }

        if !history.active_keys.is_empty() {
            history.last_keys = history.active_keys.clone();
        }
    }
}

/// Update the keyboard display
fn update_keyboard_display(
    history: Res<KeyboardInputHistory>,
    mut query: Query<&mut Text, With<KeyboardDisplayText>>,
    port: Res<CurrentPort>,
) {
    if !history.is_changed() {
        return;
    }

    for mut text in &mut query {
        let keys_display = if history.last_keys.is_empty() {
            "None".to_string()
        } else {
            history.last_keys.join(", ")
        };

        let modifiers_display = if history.modifiers.is_empty() {
            "None".to_string()
        } else {
            history.modifiers.join(", ")
        };

        let duration_display = if let Some(ms) = history.last_duration_ms {
            format!("{ms}ms")
        } else if history.active_keys.is_empty() {
            "N/A".to_string()
        } else {
            "In progress...".to_string()
        };

        let status = if history.completed {
            "Completed"
        } else if !history.active_keys.is_empty() {
            "Keys pressed"
        } else {
            "Ready"
        };

        text.0 = format!(
            "Last keys: [{keys_display}]\nModifiers: [{modifiers_display}]\nDuration: {duration_display}\nStatus: {status}\n\nUse curl to send keys:\ncurl -X POST http://localhost:{}/brp_extras/send_keys \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"keys\": [\"KeyA\", \"Space\"]}}'",
            port.0
        );
    }
}
