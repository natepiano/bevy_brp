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
use serde::{Deserialize, Serialize};

/// Resource to track keyboard input history
#[derive(Resource, Default)]
struct KeyboardInputHistory {
    /// Currently pressed keys
    active_keys:          Vec<String>,
    /// Last pressed keys (for display after release)
    last_keys:            Vec<String>,
    /// Active modifier keys
    modifiers:            Vec<String>,
    /// Complete key combination (all keys that were pressed together)
    complete_combination: Vec<String>,
    /// Complete modifiers from the last combination
    complete_modifiers:   Vec<String>,
    /// Time when the last key was pressed
    press_time:           Option<Instant>,
    /// Duration between press and release in milliseconds
    last_duration_ms:     Option<u64>,
    /// Whether the last key press has completed
    completed:            bool,
}

/// Marker component for the keyboard input display text
#[derive(Component)]
struct KeyboardDisplayText;

/// Test resource WITH Serialize/Deserialize support for BRP operations
#[derive(Resource, Default, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
struct TestConfigResource {
    pub setting_a: f32,
    pub setting_b: String,
    pub enabled:   bool,
}

/// Test resource WITHOUT Serialize/Deserialize support (only Reflect)
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct RuntimeStatsResource {
    pub frame_count: u32,
    pub total_time:  f32,
    pub debug_mode:  bool,
}

/// Test component struct WITH Serialize/Deserialize
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct TestStructWithSerDe {
    pub value:   f32,
    pub name:    String,
    pub enabled: bool,
}

/// Test component struct WITHOUT Serialize/Deserialize (only Reflect)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestStructNoSerDe {
    pub value:   f32,
    pub name:    String,
    pub enabled: bool,
}

/// Test component enum WITH Serialize/Deserialize
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
enum TestEnumWithSerDe {
    Active,
    #[default]
    Inactive,
    Special(String, u32),
}

/// Test component enum WITHOUT Serialize/Deserialize (only Reflect)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum TestEnumNoSerDe {
    Active,
    #[default]
    Inactive,
    Special(String, u32),
}

fn main() {
    let brp_plugin = BrpExtrasPlugin::new();
    let (port, _) = brp_plugin.get_effective_port();

    info!("Starting BRP Extras Test on port {}", port);

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
            primary_window: Some(bevy::window::Window {
                title: format!("BRP Extras Test - Port {port}"),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_plugin)
        .init_resource::<KeyboardInputHistory>()
        .insert_resource(CurrentPort(port))
        // Register test resources
        .register_type::<TestConfigResource>()
        .register_type::<RuntimeStatsResource>()
        // Register test components
        .register_type::<TestStructWithSerDe>()
        .register_type::<TestStructNoSerDe>()
        .register_type::<TestEnumWithSerDe>()
        .register_type::<TestEnumNoSerDe>()
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
            rotation:    Quat::from_rotation_y(std::f32::consts::PI / 4.0),
            scale:       Vec3::new(0.5, 1.5, 2.0),
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
    mut events: EventReader<KeyboardInput>,
    mut history: ResMut<KeyboardInputHistory>,
) {
    for event in events.read() {
        let key_str = format!("{:?}", event.key_code);

        match event.state {
            bevy::input::ButtonState::Pressed => {
                info!("Key pressed: {key_str}");
                history.completed = false;

                // If this is the first key in a new combination, reset the combination tracking
                if history.active_keys.is_empty() {
                    history.complete_combination.clear();
                    history.press_time = Some(Instant::now());
                }

                if !history.active_keys.contains(&key_str) {
                    history.active_keys.push(key_str.clone());
                }

                // Add to complete combination if not already there
                if !history.complete_combination.contains(&key_str) {
                    history.complete_combination.push(key_str.clone());
                }
            }
            bevy::input::ButtonState::Released => {
                info!("Key released: {key_str}");

                history.active_keys.retain(|k| k != &key_str);

                // When all keys are released, finalize the combination
                if history.active_keys.is_empty() {
                    if let Some(press_time) = history.press_time {
                        let duration = Instant::now().duration_since(press_time);
                        history.last_duration_ms = duration.as_millis().try_into().ok();
                    }

                    // Save the complete combination as last_keys
                    history.last_keys = history.complete_combination.clone();

                    // Extract modifiers from the complete combination
                    let mut modifiers = Vec::new();
                    for key in &history.complete_combination {
                        if key.contains("Control") && !modifiers.contains(&"Ctrl".to_string()) {
                            modifiers.push("Ctrl".to_string());
                        } else if key.contains("Shift") && !modifiers.contains(&"Shift".to_string())
                        {
                            modifiers.push("Shift".to_string());
                        } else if key.contains("Alt") && !modifiers.contains(&"Alt".to_string()) {
                            modifiers.push("Alt".to_string());
                        } else if key.contains("Super") && !modifiers.contains(&"Cmd".to_string()) {
                            modifiers.push("Cmd".to_string());
                        }
                    }
                    history.complete_modifiers = modifiers;

                    history.completed = true;
                }
            }
        }

        // Remove this - we now update last_keys only when all keys are released
    }

    // Update modifiers based on currently active keys
    let mut new_modifiers = Vec::new();
    for key in &history.active_keys {
        if key.contains("Control") && !new_modifiers.contains(&"Ctrl".to_string()) {
            new_modifiers.push("Ctrl".to_string());
        } else if key.contains("Shift") && !new_modifiers.contains(&"Shift".to_string()) {
            new_modifiers.push("Shift".to_string());
        } else if key.contains("Alt") && !new_modifiers.contains(&"Alt".to_string()) {
            new_modifiers.push("Alt".to_string());
        } else if key.contains("Super") && !new_modifiers.contains(&"Cmd".to_string()) {
            new_modifiers.push("Cmd".to_string());
        }
    }
    history.modifiers = new_modifiers;
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
        let keys_display = if !history.active_keys.is_empty() {
            // Show current active keys
            history.active_keys.join(", ")
        } else if !history.last_keys.is_empty() {
            // Show last completed combination
            history.last_keys.join(", ")
        } else {
            "None".to_string()
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
            "Last keys: [{keys_display}]\nDuration: {duration_display}\nStatus: {status}\n\nUse curl to send keys:\ncurl -X POST http://localhost:{}/brp_extras/send_keys \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"keys\": [\"KeyA\", \"Space\"]}}'",
            port.0
        );
    }
}
