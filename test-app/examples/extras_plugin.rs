//! BRP extras test example with keyboard input display
//!
//! This example demonstrates `bevy_brp_extras` functionality including:
//! - Format discovery
//! - Screenshot capture
//! - Keyboard input simulation
//! - Window title changes
//! - Debug mode toggling
//!
//! Used by the test suite to validate all extras functionality.

use std::time::Instant;

use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::input::gamepad::{Gamepad, GamepadSettings};
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::render::camera::ManualTextureViewHandle;
use bevy::render::experimental::occlusion_culling::OcclusionCulling;
use bevy::render::mesh::{Mesh2d, Mesh3d};
use bevy::render::primitives::CascadesFrusta;
use bevy::render::render_resource::{TextureUsages, TextureViewDescriptor, TextureViewDimension};
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy_mesh::skinning::SkinnedMesh;
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

/// Test component with array field
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct TestArrayField {
    /// Fixed-size array field
    pub vertices: [Vec2; 3],
    /// Another array field
    pub values:   [f32; 4],
}

/// Test component with tuple field
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct TestTupleField {
    /// Tuple field with two elements
    pub coords:    (f32, f32),
    /// Tuple field with three elements
    pub color_rgb: (u8, u8, u8),
}

/// Test tuple struct component
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct TestTupleStruct(pub f32, pub String, pub bool);

/// Complex nested component with various field types
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct TestComplexComponent {
    /// Nested struct field (will have .transform.translation.x paths)
    pub transform:      Transform,
    /// Enum field
    pub mode:           TestEnumWithSerDe,
    /// Array field
    pub points:         [Vec3; 2],
    /// Tuple field
    pub range:          (f32, f32),
    /// Option field
    pub optional_value: Option<f32>,
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
        .register_type::<TestArrayField>()
        .register_type::<TestTupleField>()
        .register_type::<TestTupleStruct>()
        .register_type::<TestComplexComponent>()
        // Register gamepad types for BRP access
        .register_type::<Gamepad>()
        .register_type::<GamepadSettings>()
        .add_systems(Startup, (setup_test_entities, setup_ui))
        .add_systems(PostStartup, setup_skybox_test)
        .add_systems(Update, (track_keyboard_input, update_keyboard_display))
        .run();
}

/// Resource to store the current port
#[derive(Resource)]
struct CurrentPort(u16);

/// Setup a skybox with a simple cube texture for testing mutations
fn setup_skybox_test(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Create a simple 1x6 pixel image (6 faces stacked vertically)
    // This will be reinterpreted as a cube texture
    let size = 1;
    let mut data = Vec::new();
    for _ in 0..6 {
        // Each face is 1x1 pixel, gray color
        data.extend_from_slice(&[128, 128, 128, 255]);
    }

    let mut image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width:                 size,
            height:                size * 6, // Stack 6 faces vertically
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    // Reinterpret as cube texture (height/width = 6)
    image.reinterpret_stacked_2d_as_array(image.height() / image.width());
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    let image_handle = images.add(image);

    // Spawn an entity with Skybox for testing mutations
    commands.spawn((
        Skybox {
            image:      image_handle,
            brightness: 1000.0,
            rotation:   Quat::IDENTITY,
        },
        Name::new("SkyboxTestEntity"),
    ));

    info!("Skybox test entity created with cube texture");
}

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
        bevy::render::view::visibility::VisibilityRange {
            start_margin: 0.0..10.0,
            end_margin: 90.0..100.0,
            use_aabb: false,
        },
    ));

    // Entity with Sprite component for testing mutation paths
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.5, 0.25),
            custom_size: Some(Vec2::new(64.0, 64.0)),
            flip_x: false,
            flip_y: false,
            anchor: bevy::sprite::Anchor::Center,
            ..default()
        },
        Transform::from_xyz(100.0, 100.0, 0.0),
        Name::new("TestSprite"),
        bevy::render::view::visibility::RenderLayers::layer(1),
    ));

    // Entity with TestArrayField component
    commands.spawn((
        TestArrayField {
            vertices: [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.5, 1.0),
            ],
            values:   [1.0, 2.0, 3.0, 4.0],
        },
        Name::new("TestArrayFieldEntity"),
    ));

    // Entity with TestTupleField component
    commands.spawn((
        TestTupleField {
            coords:    (10.0, 20.0),
            color_rgb: (255, 128, 64),
        },
        Name::new("TestTupleFieldEntity"),
    ));

    // Entity with TestTupleStruct component
    commands.spawn((
        TestTupleStruct(42.0, "test".to_string(), true),
        Name::new("TestTupleStructEntity"),
    ));

    // Entity with TestComplexComponent
    commands.spawn((
        TestComplexComponent {
            transform:      Transform::from_xyz(5.0, 10.0, 15.0),
            mode:           TestEnumWithSerDe::Active,
            points:         [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)],
            range:          (0.0, 100.0),
            optional_value: Some(50.0),
        },
        Name::new("TestComplexEntity"),
    ));

    // Entity with SMAA for testing mutations (separate from cameras to avoid conflicts)
    commands.spawn((
        bevy::core_pipeline::smaa::Smaa::default(),
        Name::new("SmaaTestEntity"),
    ));

    // Entity with Gamepad for testing mutations
    commands.spawn((Gamepad::default(), Name::new("GamepadTestEntity")));

    // Entity with GamepadSettings for testing mutations
    commands.spawn((
        GamepadSettings::default(),
        Name::new("GamepadSettingsTestEntity"),
    ));

    // Entity with MeshMorphWeights for testing mutations
    if let Ok(morph_weights) = MeshMorphWeights::new(vec![0.5, 1.0, 0.75]) {
        commands.spawn((morph_weights, Name::new("MeshMorphWeightsTestEntity")));
    } else {
        error!("Failed to create MeshMorphWeights with test values");
    }

    // Entity with MorphWeights for testing mutations
    commands.spawn((MorphWeights::default(), Name::new("MorphWeightsTestEntity")));

    // Entity with SkinnedMesh for testing mutations
    commands.spawn((SkinnedMesh::default(), Name::new("SkinnedMeshTestEntity")));

    // Entity with Mesh2d for testing mutations
    commands.spawn((Mesh2d(Handle::default()), Name::new("Mesh2dTestEntity")));

    // Entity with Mesh3d for testing mutations
    commands.spawn((Mesh3d(Handle::default()), Name::new("Mesh3dTestEntity")));

    // Entity with MeshMaterial2d<ColorMaterial> for testing mutations
    commands.spawn((
        bevy::prelude::MeshMaterial2d::<bevy::prelude::ColorMaterial>(Handle::default()),
        Name::new("MeshMaterial2dTestEntity"),
    ));

    // Entity with CascadesFrusta for testing mutations
    commands.spawn((
        CascadesFrusta::default(),
        Name::new("CascadesFrustaTestEntity"),
    ));

    // Entity with PointLight which will automatically get CubemapFrusta due to required components
    commands.spawn((
        bevy::pbr::PointLight {
            intensity: 1500.0,
            color: Color::WHITE,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        Name::new("PointLightTestEntity"),
    ));

    info!(
        "Test entities spawned (including Sprite and test components). BRP server running on http://localhost:{}",
        port.0
    );
}

/// Setup UI for keyboard input display
fn setup_ui(mut commands: Commands, port: Res<CurrentPort>) {
    // Camera with minimal effects to avoid visual artifacts
    commands.spawn((
        Camera2d,
        // Removed tested components: Bloom, ContrastAdaptiveSharpening, Fxaa, ChromaticAberration
    ));

    // 3D Camera with minimal components
    commands.spawn((
        Camera3d {
            depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING)
                .into(),
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ManualTextureViewHandle(0), // For testing mutations
        bevy::render::view::window::screenshot::Screenshot(bevy::render::camera::RenderTarget::Window(bevy::window::WindowRef::Primary)),
        DepthPrepass,               // Required for OcclusionCulling
        OcclusionCulling, /* For testing mutations */
                                    /* Removed tested components: DepthOfField, tonemapping */
    ));

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
            "Last keys: [{keys_display}]\nDuration: {duration_display}\nStatus: {status}\n\nUse curl to send keys:\ncurl -X POST http://localhost:{}/brp_extras/send_keys \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"keys\": [\"KeyA\", \"Space\"]}}\'\n\nUse curl to change window title:\ncurl -X POST http://localhost:{}/brp_extras/set_window_title \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"title\": \"My Custom Title\"}}'",
            port.0, port.0
        );
    }
}
