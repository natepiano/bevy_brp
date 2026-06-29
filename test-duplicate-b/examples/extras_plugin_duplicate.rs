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
use bevy::window::PrimaryWindow;
use bevy::window::WindowPlugin;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

const BACKGROUND_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);
const COMPLEX_ENTITY_NAME: &str = "ComplexTransformEntity";
const COMPLEX_ENTITY_SCALE: Vec3 = Vec3::new(0.5, 1.5, 2.0);
const COMPLEX_ENTITY_TRANSLATION: Vec3 = Vec3::new(10.0, 20.0, 30.0);
const COMPLEX_ROTATION_DIVISOR: f32 = 4.0;
const SCALED_ENTITY_NAME: &str = "ScaledEntity";
const SCALED_ENTITY_SCALE: Vec3 = Vec3::splat(2.0);
const TEST_ENTITY_NAME: &str = "TestEntity1";
const TEST_ENTITY_TRANSLATION: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const TEXT_CONTAINER_BACKGROUND: Color = Color::srgb(0.2, 0.2, 0.2);
const TEXT_CONTAINER_PADDING: f32 = 20.0;
const UI_FILL_PERCENT: f32 = 100.0;
const UI_FONT_SIZE: f32 = 20.0;
const VISIBLE_ENTITY_NAME: &str = "VisibleEntity";
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_WIDTH: u32 = 800;

/// Resource to track keyboard input history
#[derive(Resource, Default)]
struct KeyboardInputHistory {
    /// Currently pressed keys
    active_keys:      Vec<String>,
    /// Last pressed keys (for display after release)
    last_keys:        Vec<String>,
    /// Active modifier keys
    modifiers:        Vec<String>,
    /// Time when the last key was pressed
    press_time:       Option<Instant>,
    /// Duration between press and release in milliseconds
    last_duration_ms: Option<u64>,
    /// Completion state for the last key press
    completion_state: CompletionState,
}

#[derive(Default)]
enum CompletionState {
    Completed,
    #[default]
    Pending,
}

impl CompletionState {
    const fn is_completed(&self) -> bool { matches!(self, Self::Completed) }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ModifierKey {
    Alt,
    Control,
    Shift,
    Super,
}

impl ModifierKey {
    const fn label(self) -> &'static str {
        match self {
            Self::Alt => "Alt",
            Self::Control => "Ctrl",
            Self::Shift => "Shift",
            Self::Super => "Cmd",
        }
    }
}

impl TryFrom<&str> for ModifierKey {
    type Error = ();

    fn try_from(key: &str) -> Result<Self, Self::Error> {
        if key.contains("Control") {
            Ok(Self::Control)
        } else if key.contains("Shift") {
            Ok(Self::Shift)
        } else if key.contains("Alt") {
            Ok(Self::Alt)
        } else if key.contains("Super") {
            Ok(Self::Super)
        } else {
            Err(())
        }
    }
}

/// Marker component for the keyboard input display text
#[derive(Component)]
struct KeyboardDisplayText;

fn main() {
    let brp_extras_plugin = BrpExtrasPlugin::new().port_in_title(PortDisplay::Always);
    let (port, _) = brp_extras_plugin.get_effective_port();

    info!("Starting BRP Extras Test on port {port}");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("BRP Extras Test - Port {port}"),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_extras_plugin)
        .init_resource::<KeyboardInputHistory>()
        .insert_resource(CurrentPort(port))
        .add_systems(
            Startup,
            (setup_test_entities, setup_ui, minimize_window_on_start),
        )
        .add_systems(Update, (track_keyboard_input, update_keyboard_display))
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

/// Resource to store the current port
#[derive(Resource)]
struct CurrentPort(u16);

/// Setup test entities for format discovery
fn setup_test_entities(mut commands: Commands, port: Res<CurrentPort>) {
    info!("Setting up test entities...");

    // Entity with Transform and Name
    commands.spawn((
        Transform::from_translation(TEST_ENTITY_TRANSLATION),
        Name::new(TEST_ENTITY_NAME),
    ));

    // Entity with scaled transform
    commands.spawn((
        Transform::from_scale(SCALED_ENTITY_SCALE),
        Name::new(SCALED_ENTITY_NAME),
    ));

    // Entity with complex transform
    commands.spawn((
        Transform {
            translation: COMPLEX_ENTITY_TRANSLATION,
            rotation:    Quat::from_rotation_y(std::f32::consts::PI / COMPLEX_ROTATION_DIVISOR),
            scale:       COMPLEX_ENTITY_SCALE,
        },
        Name::new(COMPLEX_ENTITY_NAME),
    ));

    // Entity with visibility component
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new(VISIBLE_ENTITY_NAME),
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
                width: Val::Percent(UI_FILL_PERCENT),
                height: Val::Percent(UI_FILL_PERCENT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BACKGROUND_COLOR),
        ))
        .with_children(|parent| {
            // Text container
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(TEXT_CONTAINER_PADDING)),
                        ..default()
                    },
                    BackgroundColor(TEXT_CONTAINER_BACKGROUND),
                ))
                .with_children(|parent| {
                    // Keyboard display text
                    parent.spawn((
                        Text::new(format!(
                            "Waiting for keyboard input...\n\nUse curl to send keys:\ncurl -X POST http://localhost:{}/brp_extras/send_keys \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"keys\": [\"KeyA\", \"Space\"]}}'",
                            port.0
                        )),
                        TextFont {
                            font_size: FontSize::Px(UI_FONT_SIZE),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        KeyboardDisplayText,
                    ));
                });
        });
}

/// Track keyboard input events
fn track_keyboard_input(
    mut events: MessageReader<KeyboardInput>,
    mut history: ResMut<KeyboardInputHistory>,
) {
    for event in events.read() {
        let key_str = format!("{:?}", event.key_code);

        match event.state {
            bevy::input::ButtonState::Pressed => {
                info!("Key pressed: {key_str}");
                history.completion_state = CompletionState::Pending;
                history.press_time = Some(Instant::now());

                if !history.active_keys.contains(&key_str) {
                    history.active_keys.push(key_str.clone());
                }

                // Track modifiers
                if let Ok(modifier) = ModifierKey::try_from(key_str.as_str()) {
                    let label = modifier.label();
                    if !history.modifiers.iter().any(|existing| existing == label) {
                        history.modifiers.push(label.to_string());
                    }
                }
            },
            bevy::input::ButtonState::Released => {
                info!("Key released: {key_str}");

                if let Some(press_time) = history.press_time {
                    let duration = Instant::now().duration_since(press_time);
                    history.last_duration_ms = duration.as_millis().try_into().ok();
                }

                history.active_keys.retain(|k| k != &key_str);

                // Update modifiers
                if let Ok(modifier) = ModifierKey::try_from(key_str.as_str()) {
                    history
                        .modifiers
                        .retain(|existing| existing != modifier.label());
                }

                if history.active_keys.is_empty() && !history.last_keys.is_empty() {
                    history.completion_state = CompletionState::Completed;
                }
            },
        }

        if !history.active_keys.is_empty() {
            let keys = history.active_keys.clone();
            history.last_keys = keys;
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

        let status = if history.completion_state.is_completed() {
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
