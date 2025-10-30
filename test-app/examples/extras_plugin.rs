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

#![allow(clippy::used_underscore_binding)] // False positive on enum struct variant fields

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use bevy::animation::AnimationPlayer;
use bevy::animation::AnimationTarget;
use bevy::animation::graph::AnimationGraph;
use bevy::animation::graph::AnimationGraphHandle;
use bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::smaa::Smaa;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::RenderAssetUsages;
use bevy::audio::PlaybackSettings;
use bevy::audio::SpatialListener;
use bevy::camera::ManualTextureViewHandle;
use bevy::camera::primitives::Aabb;
use bevy::camera::primitives::CascadesFrusta;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::visibility::VisibilityRange;
use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::gizmos::GizmoAsset;
use bevy::gizmos::aabb::ShowAabbGizmo;
use bevy::gizmos::config::GizmoLineConfig;
use bevy::gizmos::light::ShowLightGizmo;
use bevy::gizmos::retained::Gizmo;
use bevy::input::gamepad::Gamepad;
use bevy::input::gamepad::GamepadSettings;
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::light::CascadeShadowConfig;
use bevy::light::Cascades;
use bevy::light::ClusteredDecal;
use bevy::light::DirectionalLightTexture;
use bevy::light::FogVolume;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::light::IrradianceVolume;
use bevy::light::NotShadowCaster;
use bevy::light::NotShadowReceiver;
use bevy::light::PointLightTexture;
use bevy::light::ShadowFilteringMethod;
use bevy::light::SpotLightTexture;
use bevy::light::VolumetricFog;
use bevy::light::VolumetricLight;
use bevy::light::cluster::ClusterConfig;
use bevy::mesh::morph::MeshMorphWeights;
use bevy::mesh::morph::MorphWeights;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::pbr::ExtendedMaterial;
use bevy::pbr::Lightmap;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceReflections;
use bevy::pbr::decal::ForwardDecalMaterialExt;
use bevy::pbr::wireframe::WireframeConfig;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::post_process::auto_exposure::AutoExposure;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::DepthOfField;
use bevy::post_process::effect_stack::ChromaticAberration;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::ChildOf;
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::camera::TemporalJitter;
use bevy::render::experimental::occlusion_culling::OcclusionCulling;
use bevy::render::globals::GlobalsUniform;
use bevy::render::render_resource::TextureViewDescriptor;
use bevy::render::render_resource::TextureViewDimension;
use bevy::render::view::ColorGrading;
use bevy::render::view::Msaa;
use bevy::render::view::window::screenshot::Screenshot;
use bevy::scene::Scene;
use bevy::scene::SceneRoot;
use bevy::sprite::Text2dShadow;
use bevy::sprite_render::Wireframe2dColor;
use bevy::ui::CalculatedClip;
use bevy::ui::FocusPolicy;
use bevy::ui::Interaction;
use bevy::ui::Outline;
use bevy::ui::UiTargetCamera;
use bevy::ui::ZIndex;
use bevy::ui::widget::Button;
use bevy::ui::widget::Label;
use bevy::window::CursorIcon;
use bevy::window::PrimaryWindow;
use bevy_brp_extras::BrpExtrasPlugin;

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

/// Test resource for BRP operations
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct TestConfigResource {
    pub setting_a: f32,
    pub setting_b: String,
    pub enabled:   bool,
}

/// Test resource for runtime statistics
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct RuntimeStatsResource {
    pub frame_count: u32,
    pub total_time:  f32,
    pub debug_mode:  bool,
}

/// Simple `HashSet` test component with just strings
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct SimpleSetComponent {
    pub string_set: HashSet<String>,
}

/// Test component with `HashMap` for testing map mutations
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMapComponent {
    /// String to String map
    pub strings:    HashMap<String, String>,
    /// String to f32 map
    pub values:     HashMap<String, f32>,
    /// String to Transform map (complex nested type)
    pub transforms: HashMap<String, Transform>,
}

/// Test component with enum-keyed `HashMap` (`NotMutable` due to complex key)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestEnumKeyedMap {
    /// Enum to String map (should be `NotMutable` due to complex key)
    pub enum_keyed: HashMap<SimpleTestEnum, String>,
}

/// Simple test enum for `HashMap` key testing
#[derive(Reflect, Hash, Eq, PartialEq, Clone)]
enum SimpleTestEnum {
    Variant1,
    Variant2,
    Variant3,
}

impl Default for SimpleTestEnum {
    fn default() -> Self {
        Self::Variant1
    }
}

/// Test component struct for testing
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestStructNoSerDe {
    pub value:   f32,
    pub name:    String,
    pub enabled: bool,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum NestedConfigEnum {
    /// Unit Variant 1
    #[default]
    Always,
    /// Unit Variant 2
    Never,
    /// Tuple Variant
    Conditional(u32),
}

/// Simple nested enum for testing enum recursion - like Option<Vec2>
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum SimpleNestedEnum {
    #[default]
    None,
    /// This variant contains a Vec2 - should generate nested paths
    WithVec2(Vec2),
    /// This variant contains a Transform - should generate deeply nested paths
    WithTransform(Transform),
    /// Struct variant - should generate field-based nested paths
    WithStruct { position: Vec3, scale: f32 },
}

/// Test enum with Option variant (generic enum)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum OptionTestEnum {
    #[default]
    Nothing,
    /// Option<Vec2> - should generate nested paths through Some variant
    MaybeVec2(Option<Vec2>),
    /// Option<Transform> - should generate deeply nested paths through Some variant
    MaybeTransform(Option<Transform>),
}

/// Test concrete enum that wraps other enums (simulating generics)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum WrapperEnum {
    #[default]
    Empty,
    /// Wrapper with nested enum - should recurse into `SimpleNestedEnum`'s paths
    WithSimpleEnum(SimpleNestedEnum),
    /// Option wrapper - should recurse through Option<SimpleNestedEnum>
    WithOptionalEnum(Option<SimpleNestedEnum>),
}

/// Test enum for verifying variant chain propagation through non-enum intermediate levels
/// This tests: Enum -> Struct (no variant requirement) -> Enum (needs variant)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum TestVariantChainEnum {
    #[default]
    Empty,
    /// Variant containing a struct that itself contains an enum
    WithMiddleStruct { middle_struct: MiddleStruct },
}

/// Intermediate struct that contains an enum but doesn't require any variant itself
#[derive(Default, Reflect)]
struct MiddleStruct {
    /// Regular field with no special requirements
    some_field:  String,
    /// Another regular field
    some_value:  f32,
    /// Nested enum that will require variant selection
    nested_enum: BottomEnum,
}

/// Bottom-level enum that requires variant selection for its fields
#[derive(Default, Reflect)]
enum BottomEnum {
    VariantA(u32),
    VariantB {
        value: f32,
        name:  String,
    },
    #[default]
    VariantC,
    /// Second tuple variant with same signature as `VariantA`
    VariantD(u32),
}

/// Test enum with array field for testing array wrapping in enum variants
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum TestEnumWithArray {
    #[default]
    Empty,
    /// Variant with array of Vec2
    WithVec2Array([Vec2; 3]),
    /// Variant with array of f32
    WithFloatArray([f32; 4]),
    /// Struct variant with array field
    WithStructArray { points: [Vec3; 2], scale: f32 },
}

/// Test component with array field
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestArrayField {
    /// Fixed-size array field
    pub vertices: [Vec2; 3],
    /// Another array field
    pub values:   [f32; 4],
}

/// Test component with array of Transforms
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestArrayTransforms {
    /// Array of Transform components
    pub transforms: [Transform; 2],
}

/// Test component with tuple field
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestTupleField {
    /// Tuple field with two elements
    pub coords:    (f32, f32),
    /// Tuple field with three elements
    pub color_rgb: (u8, u8, u8),
}

/// Test tuple struct component
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestTupleStruct(pub f32, pub String, pub bool);

/// Test component with complex tuple types for testing tuple recursion
#[derive(Component, Reflect)]
#[reflect(Component)]
struct TestComplexTuple {
    /// Tuple with complex types that should recurse
    pub complex_tuple: (Transform, Vec3),
    /// Nested tuple with both simple and complex types
    pub nested_tuple:  (Vec2, (f32, String)),
}

/// Core type with mixed mutability for `mutability_reason` testing
/// Simplified version with reduced nesting depth
#[derive(Default, Reflect)]
struct TestMixedMutabilityCore {
    /// Mutable string field
    pub mutable_string: String,

    /// Mutable float field
    pub mutable_float: f32,

    /// Not mutable field - Arc type
    pub not_mutable_arc: std::sync::Arc<String>,

    /// Partially mutable field - simple nested struct
    pub partially_mutable_nested: TestPartiallyMutableNested,
}

/// Vec parent containing mixed mutability items
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMixedMutabilityVec {
    /// Vec of mixed mutability items
    pub items: Vec<TestMixedMutabilityCore>,
}

/// Array parent containing mixed mutability items
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMixedMutabilityArray {
    /// Fixed-size array of mixed mutability items
    pub items: [TestMixedMutabilityCore; 2],
}

/// `TupleStruct` parent containing mixed mutability item
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMixedMutabilityTuple(pub TestMixedMutabilityCore, pub f32, pub String);

/// Enum parent containing mixed mutability variants
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum TestMixedMutabilityEnum {
    #[default]
    None,
    /// Variant with mixed mutability struct
    WithMixed(TestMixedMutabilityCore),
    /// Variant with multiple fields including mixed
    Multiple {
        name:  String,
        mixed: TestMixedMutabilityCore,
        value: f32,
    },
}

/// Simplified nested type - partially mutable with just one level
#[derive(Default, Reflect)]
struct TestPartiallyMutableNested {
    /// Mutable field in nested struct
    pub nested_mutable_value: f32,

    /// Not mutable - Arc type without serialization
    pub nested_not_mutable_arc: std::sync::Arc<Vec<u8>>,
}

impl Default for TestComplexTuple {
    fn default() -> Self {
        Self {
            complex_tuple: (Transform::default(), Vec3::ZERO),
            nested_tuple:  (Vec2::ZERO, (0.0, String::new())),
        }
    }
}

/// Complex nested component with various field types
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestComplexComponent {
    /// Nested struct field (will have .transform.translation.x paths)
    pub transform:      Transform,
    /// Enum field
    pub mode:           SimpleNestedEnum,
    /// Array field
    pub points:         [Vec3; 2],
    /// Tuple field
    pub range:          (f32, f32),
    /// Option field
    pub optional_value: Option<f32>,
}

/// Test component with List and Set collection types containing complex elements
#[derive(Component, Reflect)]
#[reflect(Component)]
struct TestCollectionComponent {
    /// Vec<Transform> - should trigger `ListMutationBuilder` with complex recursion
    pub transform_list: Vec<Transform>,
    /// `HashSet`<String> - should trigger `SetMutationBuilder`
    pub struct_set:     HashSet<String>,
}

impl Default for TestCollectionComponent {
    fn default() -> Self {
        let mut struct_set = HashSet::new();
        struct_set.insert("first_item".to_string());
        struct_set.insert("second_item".to_string());
        struct_set.insert("third_item".to_string());

        Self {
            transform_list: vec![
                Transform::from_xyz(10.0, 20.0, 30.0),
                Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0)),
                Transform::from_scale(Vec3::new(0.5, 1.5, 2.0)),
            ],
            struct_set,
        }
    }
}

fn main() {
    let brp_plugin = BrpExtrasPlugin::new();
    let (port, _) = brp_plugin.get_effective_port();

    info!("Starting BRP Extras Test on port {}", port);

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
            primary_window: Some(bevy::window::Window {
                title: format!("BRP Extras Test - Port {port}"),
                resolution: (800, 600).into(),
                focused: false,
                position: bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Primary,
                ),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_plugin)
        .add_plugins(MeshPickingPlugin)
        .init_resource::<KeyboardInputHistory>()
        .init_resource::<GlobalsUniform>()
        .insert_resource(CurrentPort(port))
        .insert_resource(WireframeConfig {
            global:        true,
            default_color: Color::WHITE,
        })
        .insert_resource(TestConfigResource {
            setting_a: 100.0,
            setting_b: "test config".to_string(),
            enabled:   true,
        })
        .insert_resource(RuntimeStatsResource {
            frame_count: 0,
            total_time:  0.0,
            debug_mode:  false,
        })
        .insert_resource(InputFocus::default())
        .add_systems(
            Startup,
            (setup_test_entities, setup_ui, minimize_window_on_start),
        )
        .add_systems(PostStartup, (setup_skybox_test, setup_scene_test))
        .add_systems(Update, (track_keyboard_input, update_keyboard_display))
        .run();
}

/// Resource to store the current port
#[derive(Resource)]
struct CurrentPort(u16);

/// Minimize the window immediately on startup
fn minimize_window_on_start(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}

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
        RenderAssetUsages::RENDER_WORLD,
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

/// Setup a simple scene with `SceneRoot` for testing
fn setup_scene_test(mut commands: Commands, mut scenes: ResMut<Assets<Scene>>) {
    // Create a simple scene with a few test entities
    let mut scene_world = World::new();

    // Add some simple entities to the scene
    scene_world.spawn((
        Transform::from_xyz(1.0, 2.0, 3.0),
        Name::new("SceneEntity1"),
    ));

    scene_world.spawn((
        Transform::from_xyz(4.0, 5.0, 6.0),
        Name::new("SceneEntity2"),
    ));

    let scene = Scene::new(scene_world);
    let scene_handle = scenes.add(scene);

    // Spawn entity with SceneRoot for testing
    commands.spawn((SceneRoot(scene_handle), Name::new("SceneRootTestEntity")));

    info!("SceneRoot test entity created");
}

/// Setup test entities for format discovery
fn setup_test_entities(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    port: Res<CurrentPort>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
) {
    info!("Setting up test entities...");

    spawn_transform_entities(&mut commands);
    spawn_visual_entities(&mut commands, &asset_server);
    spawn_test_component_entities(&mut commands);
    spawn_animation_and_audio_entities(&mut commands, &mut animation_graphs);
    spawn_render_entities(&mut commands);
    spawn_retained_gizmo_entities(&mut commands, &mut gizmo_assets);

    info!(
        "Test entities spawned (including Sprite and test components). BRP server running on http://localhost:{}",
        port.0
    );
}

fn spawn_transform_entities(commands: &mut Commands) {
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
        VisibilityRange {
            start_margin: 0.0..10.0,
            end_margin:   90.0..100.0,
            use_aabb:     false,
        },
    ));
}

fn spawn_visual_entities(commands: &mut Commands, asset_server: &AssetServer) {
    spawn_sprite_and_ui_components(commands);
    spawn_light_entities(commands, asset_server);
    spawn_shadow_test_entities(commands, asset_server);
}

fn spawn_sprite_and_ui_components(commands: &mut Commands) {
    // Entity with Sprite component for testing mutation paths (includes Anchor enum)
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.5, 0.25),
            custom_size: Some(Vec2::new(64.0, 64.0)),
            flip_x: false,
            flip_y: false,
            ..default()
        },
        Transform::from_xyz(100.0, 100.0, 0.0),
        Name::new("TestSprite"),
        RenderLayers::layer(1),
    ));

    // Entity with Wireframe2dColor for testing mutations
    commands.spawn((
        Wireframe2dColor {
            color: Color::hsla(180.0, 0.5, 0.5, 1.0),
        },
        Name::new("Wireframe2dColorTestEntity"),
    ));

    // Entity with SMAA for testing mutations (separate from cameras to avoid conflicts)
    commands.spawn((Smaa::default(), Name::new("SmaaTestEntity")));

    // Entity with BorderRadius for testing mutations
    commands.spawn((
        BorderRadius::all(Val::Px(10.0)),
        Name::new("BorderRadiusTestEntity"),
    ));

    // Entity with CursorIcon for testing mutations
    commands.spawn((
        CursorIcon::System(bevy::window::SystemCursorIcon::Default),
        Name::new("CursorIconTestEntity"),
    ));

    // Entity with BorderGradient for testing mutations
    commands.spawn((
        bevy::ui::BorderGradient(vec![bevy::ui::Gradient::Linear(bevy::ui::LinearGradient {
            color_space: bevy::ui::InterpolationColorSpace::Srgba,
            angle:       std::f32::consts::FRAC_PI_4,
            stops:       vec![
                bevy::ui::gradients::ColorStop::percent(Color::srgb(1.0, 0.0, 0.0), 0.0),
                bevy::ui::gradients::ColorStop::percent(Color::srgb(0.0, 0.0, 1.0), 100.0),
            ],
        })]),
        Name::new("BorderGradientTestEntity"),
    ));

    // Entity with BackgroundGradient for testing mutations
    commands.spawn((
        Node {
            width: Val::Px(200.0),
            height: Val::Px(200.0),
            ..default()
        },
        bevy::ui::BackgroundGradient(vec![bevy::ui::Gradient::Linear(bevy::ui::LinearGradient {
            color_space: bevy::ui::InterpolationColorSpace::Srgba,
            angle:       std::f32::consts::FRAC_PI_2,
            stops:       vec![
                bevy::ui::gradients::ColorStop::percent(Color::srgb(0.0, 1.0, 0.0), 0.0),
                bevy::ui::gradients::ColorStop::percent(Color::srgb(1.0, 0.0, 1.0), 100.0),
            ],
        })]),
        Name::new("BackgroundGradientTestEntity"),
    ));
}

fn spawn_light_entities(commands: &mut Commands, asset_server: &AssetServer) {
    // Entity with PointLight which will automatically get CubemapFrusta and shadow maps when
    // shadows enabled
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            color: Color::WHITE,
            shadows_enabled: true, /* Enable shadows to trigger CubemapFrusta and
                                    * PointLightShadowMap */
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        Name::new("PointLightTestEntity"),
        ShadowFilteringMethod::default(),
        PointLightTexture {
            image:          asset_server.load("lightmaps/caustic_directional_texture.png"),
            cubemap_layout: bevy::camera::primitives::CubemapLayout::CrossVertical,
        },
        ShowLightGizmo {
            color: Some(bevy::gizmos::light::LightGizmoColor::Manual(Color::srgb(
                1.0, 0.0, 1.0,
            ))),
        },
    ));

    // Entity with DirectionalLight for testing mutations
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        Name::new("DirectionalLightTestEntity"),
        CascadeShadowConfig::default(),
        Cascades::default(),
        VolumetricLight, // For testing mutations - enables light shafts/god rays
        DirectionalLightTexture {
            image: asset_server.load("lightmaps/caustic_directional_texture.png"),
            tiled: true,
        },
        ShowLightGizmo {
            color: Some(bevy::gizmos::light::LightGizmoColor::MatchLightColor),
        },
    ));

    // Entity with SpotLight for testing mutations
    commands.spawn((
        SpotLight {
            color: Color::WHITE,
            intensity: 2000.0,
            range: 10.0,
            radius: 0.1,
            shadows_enabled: true,
            inner_angle: 0.6,
            outer_angle: 0.8,
            ..default()
        },
        Transform::from_xyz(0.0, 4.0, 0.0),
        Name::new("SpotLightTestEntity"),
        SpotLightTexture {
            image: asset_server.load("lightmaps/caustic_directional_texture.png"),
        },
        ShowLightGizmo {
            color: Some(bevy::gizmos::light::LightGizmoColor::Varied),
        },
    ));

    // Entity with DistanceFog for testing mutations
    commands.spawn((
        bevy::pbr::DistanceFog {
            color:                      Color::srgba(0.35, 0.48, 0.66, 1.0),
            directional_light_color:    Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 8.0,
            falloff:                    bevy::pbr::FogFalloff::Linear {
                start: 5.0,
                end:   20.0,
            },
        },
        Name::new("DistanceFogTestEntity"),
    ));
}

fn spawn_shadow_test_entities(commands: &mut Commands, asset_server: &AssetServer) {
    // Entity with NotShadowCaster for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()),                             // Dummy mesh handle
        MeshMaterial3d::<StandardMaterial>(Handle::default()), // Dummy material handle
        Transform::from_xyz(-2.0, 1.0, 0.0),
        NotShadowCaster, // For testing mutations
        Aabb::from_min_max(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
        ShowAabbGizmo {
            color: Some(Color::srgb(1.0, 0.0, 0.0)),
        },
        Name::new("NotShadowCasterTestEntity"),
    ));

    // Entity with Lightmap for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()),                             // Dummy mesh handle
        MeshMaterial3d::<StandardMaterial>(Handle::default()), // Dummy material handle
        Transform::from_xyz(0.0, 0.0, 0.0),
        Lightmap {
            image:            asset_server.load("lightmaps/caustic_directional_texture.png"),
            uv_rect:          bevy::math::Rect::new(0.0, 0.0, 1.0, 1.0),
            bicubic_sampling: true,
        },
        Name::new("LightmapTestEntity"),
    ));

    // Entity with NotShadowReceiver for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()),                             // Dummy mesh handle
        MeshMaterial3d::<StandardMaterial>(Handle::default()), // Dummy material handle
        Transform::from_xyz(2.0, 1.0, 0.0),
        NotShadowReceiver, // For testing mutations
        Name::new("NotShadowReceiverTestEntity"),
    ));

    // Entity with ExtendedMaterial<StandardMaterial, ForwardDecalMaterialExt> for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()), // Dummy mesh handle
        MeshMaterial3d::<ExtendedMaterial<StandardMaterial, ForwardDecalMaterialExt>>(
            Handle::default(),
        ), // Dummy material handle
        Transform::from_xyz(0.0, 2.0, 0.0),
        Name::new("ExtendedDecalMaterialTestEntity"),
    ));
}

fn spawn_test_component_entities(commands: &mut Commands) {
    spawn_array_and_tuple_test_entities(commands);
    spawn_collection_test_entities(commands);
    spawn_enum_test_entities(commands);
    spawn_gltf_test_entities(commands);
    spawn_mixed_mutability_test_entities(commands);
}

fn spawn_array_and_tuple_test_entities(commands: &mut Commands) {
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

    commands.spawn((
        TestArrayTransforms {
            transforms: [
                Transform::from_xyz(1.0, 2.0, 3.0),
                Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ],
        },
        Name::new("TestArrayTransformsEntity"),
    ));

    commands.spawn((
        TestTupleField {
            coords:    (10.0, 20.0),
            color_rgb: (255, 128, 64),
        },
        Name::new("TestTupleFieldEntity"),
    ));

    commands.spawn((
        TestTupleStruct(42.0, "test".to_string(), true),
        Name::new("TestTupleStructEntity"),
    ));

    commands.spawn((
        TestComplexTuple {
            complex_tuple: (
                Transform::from_xyz(10.0, 20.0, 30.0),
                Vec3::new(1.0, 2.0, 3.0),
            ),
            nested_tuple:  (Vec2::new(5.0, 10.0), (3.0, "nested".to_string())),
        },
        Name::new("TestComplexTupleEntity"),
    ));
}

fn spawn_collection_test_entities(commands: &mut Commands) {
    let mut simple_set = SimpleSetComponent::default();
    simple_set.string_set.insert("hello".to_string());
    simple_set.string_set.insert("world".to_string());
    simple_set.string_set.insert("test".to_string());
    commands.spawn((simple_set, Name::new("SimpleSetEntity")));

    let mut test_map = TestMapComponent::default();
    test_map
        .strings
        .insert("key1".to_string(), "value1".to_string());
    test_map
        .strings
        .insert("key2".to_string(), "value2".to_string());
    test_map
        .strings
        .insert("key3".to_string(), "value3".to_string());

    test_map.values.insert("temperature".to_string(), 23.5);
    test_map.values.insert("humidity".to_string(), 65.0);
    test_map.values.insert("pressure".to_string(), 1013.25);

    test_map
        .transforms
        .insert("player".to_string(), Transform::from_xyz(10.0, 0.0, 5.0));
    test_map
        .transforms
        .insert("enemy".to_string(), Transform::from_xyz(-5.0, 0.0, -10.0));
    test_map.transforms.insert(
        "powerup".to_string(),
        Transform::from_xyz(0.0, 5.0, 0.0).with_scale(Vec3::splat(2.0)),
    );

    commands.spawn((test_map, Name::new("TestMapEntity")));

    let mut enum_keyed_map = TestEnumKeyedMap::default();
    enum_keyed_map
        .enum_keyed
        .insert(SimpleTestEnum::Variant1, "first".to_string());
    enum_keyed_map
        .enum_keyed
        .insert(SimpleTestEnum::Variant2, "second".to_string());
    enum_keyed_map
        .enum_keyed
        .insert(SimpleTestEnum::Variant3, "third".to_string());

    commands.spawn((enum_keyed_map, Name::new("TestEnumKeyedMapEntity")));

    commands.spawn((
        TestCollectionComponent::default(),
        Name::new("TestCollectionEntity"),
    ));
}

fn spawn_enum_test_entities(commands: &mut Commands) {
    commands.spawn((
        NestedConfigEnum::Always,
        Name::new("NestedConfigEnumAlwaysEntity"),
    ));

    commands.spawn((
        NestedConfigEnum::Never,
        Name::new("NestedConfigEnumNeverEntity"),
    ));

    commands.spawn((
        NestedConfigEnum::Conditional(42),
        Name::new("NestedConfigEnumConditionalEntity"),
    ));

    commands.spawn((
        TestComplexComponent {
            transform:      Transform::from_xyz(5.0, 10.0, 15.0),
            mode:           SimpleNestedEnum::WithVec2(Vec2::new(10.0, 20.0)),
            points:         [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)],
            range:          (0.0, 100.0),
            optional_value: Some(50.0),
        },
        Name::new("TestComplexEntity"),
    ));

    commands.spawn((
        TestVariantChainEnum::WithMiddleStruct {
            middle_struct: MiddleStruct {
                some_field:  "test_field".to_string(),
                some_value:  42.5,
                nested_enum: BottomEnum::VariantA(999),
            },
        },
        Name::new("TestVariantChainEntity"),
    ));

    commands.spawn((
        SimpleNestedEnum::WithVec2(Vec2::new(10.0, 20.0)),
        Name::new("SimpleNestedEnumEntity"),
    ));

    commands.spawn((
        TestEnumWithArray::WithVec2Array([
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
        ]),
        Name::new("TestEnumWithArrayEntity"),
    ));

    commands.spawn((
        SimpleNestedEnum::WithVec2(Vec2::new(10.0, 20.0)),
        Name::new("SimpleNestedEnumVec2Entity"),
    ));

    commands.spawn((
        SimpleNestedEnum::WithTransform(Transform::from_xyz(5.0, 10.0, 15.0)),
        Name::new("SimpleNestedEnumTransformEntity"),
    ));

    commands.spawn((
        SimpleNestedEnum::WithStruct {
            position: Vec3::new(1.0, 2.0, 3.0),
            scale:    2.5,
        },
        Name::new("SimpleNestedEnumStructEntity"),
    ));

    commands.spawn((
        OptionTestEnum::MaybeVec2(Some(Vec2::new(100.0, 200.0))),
        Name::new("OptionTestEnumVec2Entity"),
    ));

    commands.spawn((
        OptionTestEnum::MaybeTransform(Some(Transform::from_scale(Vec3::splat(3.0)))),
        Name::new("OptionTestEnumTransformEntity"),
    ));

    commands.spawn((
        WrapperEnum::WithSimpleEnum(SimpleNestedEnum::WithVec2(Vec2::new(50.0, 75.0))),
        Name::new("WrapperEnumSimpleEntity"),
    ));

    commands.spawn((
        WrapperEnum::WithOptionalEnum(Some(SimpleNestedEnum::WithTransform(
            Transform::from_rotation(Quat::from_rotation_y(1.0)),
        ))),
        Name::new("WrapperEnumOptionalEntity"),
    ));
}

fn spawn_gltf_test_entities(commands: &mut Commands) {
    commands.spawn((
        TestStructNoSerDe {
            value:   123.45,
            name:    "test_struct".to_string(),
            enabled: true,
        },
        Name::new("TestStructNoSerDeEntity"),
    ));

    commands.spawn((Gamepad::default(), Name::new("GamepadTestEntity")));

    commands.spawn((
        GamepadSettings::default(),
        Name::new("GamepadSettingsTestEntity"),
    ));

    commands.spawn((
        bevy::gltf::GltfExtras {
            value: "test gltf extras".to_string(),
        },
        Name::new("GltfExtrasTestEntity"),
    ));

    commands.spawn((
        bevy::gltf::GltfMaterialExtras {
            value: "test material extras".to_string(),
        },
        Name::new("GltfMaterialExtrasTestEntity"),
    ));

    commands.spawn((
        bevy::gltf::GltfMaterialName("test material name".to_string()),
        Name::new("GltfMaterialNameTestEntity"),
    ));

    commands.spawn((
        bevy::gltf::GltfMeshExtras {
            value: "test mesh extras".to_string(),
        },
        Name::new("GltfMeshExtrasTestEntity"),
    ));

    commands.spawn((
        bevy::gltf::GltfSceneExtras {
            value: "test scene extras".to_string(),
        },
        Name::new("GltfSceneExtrasTestEntity"),
    ));

    commands.spawn((Gamepad::default(), Name::new("TestGamepad")));
}

fn spawn_mixed_mutability_test_entities(commands: &mut Commands) {
    let create_mixed_core = |suffix: &str| TestMixedMutabilityCore {
        mutable_string:           format!("test_string_{suffix}"),
        mutable_float:            42.5,
        not_mutable_arc:          Arc::new(format!("arc_string_{suffix}")),
        partially_mutable_nested: TestPartiallyMutableNested {
            nested_mutable_value:   100.0,
            nested_not_mutable_arc: Arc::new(vec![1, 2, 3, 4, 5]),
        },
    };

    commands.spawn((
        TestMixedMutabilityVec {
            items: vec![
                create_mixed_core("vec_0"),
                create_mixed_core("vec_1"),
                create_mixed_core("vec_2"),
            ],
        },
        Name::new("TestMixedMutabilityVecEntity"),
    ));

    commands.spawn((
        TestMixedMutabilityArray {
            items: [create_mixed_core("array_0"), create_mixed_core("array_1")],
        },
        Name::new("TestMixedMutabilityArrayEntity"),
    ));

    commands.spawn((
        TestMixedMutabilityTuple(create_mixed_core("tuple"), 99.9, "tuple_string".to_string()),
        Name::new("TestMixedMutabilityTupleEntity"),
    ));

    commands.spawn((
        TestMixedMutabilityEnum::Multiple {
            name:  "enum_multiple".to_string(),
            mixed: create_mixed_core("enum"),
            value: 123.45,
        },
        Name::new("TestMixedMutabilityEnumEntity"),
    ));
}

fn spawn_retained_gizmo_entities(
    commands: &mut Commands,
    gizmo_assets: &mut ResMut<Assets<GizmoAsset>>,
) {
    // Create a gizmo asset with a simple sphere
    let mut gizmo_asset = GizmoAsset::default();
    gizmo_asset.sphere(Vec3::ZERO, 1.0, Color::srgb(1.0, 0.0, 0.0));

    let gizmo_handle = gizmo_assets.add(gizmo_asset);

    // Spawn entity with Gizmo component
    commands.spawn((
        Gizmo {
            handle:      gizmo_handle,
            line_config: GizmoLineConfig {
                width: 2.0,
                perspective: true,
                ..default()
            },
            depth_bias:  0.0,
        },
        Name::new("RetainedGizmoTestEntity"),
    ));

    info!("Retained Gizmo test entity created");
}

fn spawn_animation_and_audio_entities(
    commands: &mut Commands,
    animation_graphs: &mut ResMut<Assets<AnimationGraph>>,
) {
    // Entity with AnimationGraphHandle AND AnimationPlayer AND AnimationTransitions for testing
    // mutations (they must be on the same entity per Bevy's requirements)
    let animation_graph = AnimationGraph::default();
    let graph_handle = animation_graphs.add(animation_graph);
    commands.spawn((
        AnimationGraphHandle(graph_handle),
        AnimationPlayer::default(),
        AnimationTransitions::default(),
        Name::new("AnimationGraphHandleAndPlayerAndTransitionsTestEntity"),
    ));

    // Entity with AnimationTarget for testing mutations
    commands.spawn((
        AnimationTarget {
            id:     bevy::animation::AnimationTargetId::from_name(&Name::new("test_target")),
            player: Entity::PLACEHOLDER,
        },
        Name::new("AnimationTargetTestEntity"),
    ));

    // Entity with DenoiseCas for testing mutations
    // Note: DenoiseCas doesn't have a public constructor, but we can register it
    // It's automatically added when ContrastAdaptiveSharpening has denoise enabled

    // Entity with TemporalAntiAliasing for testing mutations
    commands.spawn((
        TemporalAntiAliasing::default(),
        Name::new("TemporalAntiAliasingTestEntity"),
    ));

    // Entity with SpatialListener for testing mutations
    commands.spawn((
        SpatialListener::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("SpatialListenerTestEntity"),
    ));

    // Entity with TabGroup for testing mutations
    commands.spawn((TabGroup::new(0), Name::new("TabGroupTestEntity")));

    // Entity with TabIndex for testing mutations
    commands.spawn((TabIndex(0), Name::new("TabIndexTestEntity")));

    // Entity with FogVolume for testing mutations
    commands.spawn((
        FogVolume::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("FogVolumeTestEntity"),
    ));

    // Entity with MainPassResolutionOverride for testing mutations
    // Try accessing it directly from bevy::camera even though the module is private
    commands.spawn((
        bevy::camera::MainPassResolutionOverride(bevy::math::UVec2::new(1920, 1080)),
        Name::new("MainPassResolutionOverrideTestEntity"),
    ));

    // Entity with GltfMeshName for testing mutations
    commands.spawn((
        bevy::gltf::GltfMeshName("test_mesh_name".to_string()),
        Name::new("GltfMeshNameTestEntity"),
    ));

    // Entity with PlaybackSettings - this is actually a component that can be spawned!
    commands.spawn((
        PlaybackSettings::default(),
        Name::new("PlaybackSettingsTestEntity"),
    ));

    // Note: DenoiseCas is automatically added when ContrastAdaptiveSharpening has denoise enabled
    // AnimationGraphHandle, DirectionalLightTexture, PointLightTexture, SpotLightTexture,
    // GeneratedEnvironmentMapLight are internal/generated components
    // AudioSink, SpatialAudioSink, AudioSourceHandle, SpatialAudioSourceHandle, GlobalVolume,
    // Volume are not components
}

fn spawn_render_entities(commands: &mut Commands) {
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

    // Entity with MeshMaterial2d<ColorMaterial> and Mesh2d for testing mutations
    commands.spawn((
        Mesh2d(Handle::default()),
        bevy::prelude::MeshMaterial2d::<bevy::prelude::ColorMaterial>(Handle::default()),
        Name::new("MeshMaterial2dTestEntity"),
    ));

    // Entity with CascadesFrusta for testing mutations
    commands.spawn((
        CascadesFrusta::default(),
        Name::new("CascadesFrustaTestEntity"),
    ));

    // Entity with Text2d for testing mutations
    commands.spawn((
        Text2d("Hello Text2d".to_string()),
        Text2dShadow::default(), // For testing mutations
        Transform::from_xyz(50.0, 50.0, 0.0),
        Name::new("Text2dTestEntity"),
    ));

    // Entity with ClusteredDecal for testing mutations
    commands.spawn((
        ClusteredDecal {
            image: Handle::default(),
            tag:   1,
        },
        Name::new("ClusteredDecalTestEntity"),
    ));

    // Entity with LightProbe for testing mutations
    commands.spawn((LightProbe, Name::new("LightProbeTestEntity")));

    // Entity with ClusterConfig for testing mutations
    commands.spawn((
        ClusterConfig::default(),
        Name::new("ClusterConfigTestEntity"),
    ));

    // Entity with EnvironmentMapLight for testing mutations
    commands.spawn((
        EnvironmentMapLight::default(),
        Name::new("EnvironmentMapLightTestEntity"),
    ));

    // Entity with GeneratedEnvironmentMapLight for testing mutations
    // Uses the same skybox image handle we created earlier
    commands.spawn((
        GeneratedEnvironmentMapLight {
            environment_map:                  Handle::default(), // Dummy handle for testing
            intensity:                        1000.0,
            rotation:                         Quat::IDENTITY,
            affects_lightmapped_mesh_diffuse: true,
        },
        Name::new("GeneratedEnvironmentMapLightTestEntity"),
    ));

    // Entity with IrradianceVolume for testing mutations
    commands.spawn((
        IrradianceVolume::default(),
        Name::new("IrradianceVolumeTestEntity"),
    ));

    // Entity with AmbientLight (requires Camera) for testing mutations
    // Also has Msaa since this camera is disabled and won't cause rendering conflicts
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 2,         // Unique order for this test camera
            is_active: false, // Disable this test camera to avoid rendering
            ..default()
        },
        AmbientLight::default(),
        Msaa::default(), // Safe to test here since camera is disabled
        Transform::from_xyz(100.0, 100.0, 100.0),
        Name::new("AmbientLightTestEntity"),
    ));

    // Entity with Screenshot for testing mutations
    commands.spawn((
        Screenshot::primary_window(),
        Name::new("ScreenshotTestEntity"),
    ));

    // Entity with OcclusionCulling for testing mutations
    commands.spawn((OcclusionCulling, Name::new("OcclusionCullingTestEntity")));

    // Entity with NoFrustumCulling for testing mutations
    commands.spawn((
        NoFrustumCulling,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("NoFrustumCullingTestEntity"),
    ));

    // Entity with ManualTextureViewHandle for testing mutations
    commands.spawn((
        ManualTextureViewHandle(42),
        Name::new("ManualTextureViewHandleTestEntity"),
    ));
}

/// Setup UI for keyboard input display
fn setup_ui(mut commands: Commands, port: Res<CurrentPort>) {
    spawn_cameras(&mut commands);
    spawn_ui_elements(&mut commands, &port);
}

fn spawn_cameras(commands: &mut Commands) {
    // Single 2D Camera that handles both UI and 2D sprites
    commands.spawn((
        Camera2d,
        Camera {
            order: 0, // Main camera
            ..default()
        },
        Bloom::default(),
        IsDefaultUiCamera, // This camera renders UI
    ));

    // 3D Camera for 3D test entities (disabled to avoid rendering conflicts)
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 1,         // Different order to avoid ambiguity
                is_active: false, // Disable to avoid rendering conflicts with deferred pipeline
                ..default()
            },
            Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            AutoExposure::default(), // For testing mutations
            ColorGrading::default(), // For testing mutations
            ContrastAdaptiveSharpening {
                enabled: true,
                denoise: true, // Enable denoise to trigger DenoiseCas auto-extraction
                ..default()
            },
            DepthOfField::default(),                // For testing mutations
            Fxaa::default(),                        // For testing mutations
            MipBias(0.0),                           // For testing mutations
            TemporalJitter::default(),              // For testing mutations
            ChromaticAberration::default(),         // For testing mutations
            ScreenSpaceAmbientOcclusion::default(), // For testing mutations
            ScreenSpaceReflections::default(),      // For testing mutations
            VolumetricFog::default(),               // For testing mutations
            MotionVectorPrepass,                    /* For testing mutations
                                                     * Msaa causes crashes with the deferred
                                                     * rendering setup - test separately */
        ))
        .insert(MotionBlur::default()); // For testing mutations (added separately due to bundle size limit)
}

fn spawn_ui_elements(commands: &mut Commands, port: &Res<CurrentPort>) {
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
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)), // Back to dark background
        ))
        .with_children(|parent| {
            spawn_text_container(parent, port);
        });
}

fn spawn_text_container(parent: &mut RelatedSpawnerCommands<ChildOf>, port: &Res<CurrentPort>) {
    // Text container with blue background
    parent
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.3, 0.5)), /* Blue background for the entire text
                                                          * area */
            BoxShadowSamples(4),
            CalculatedClip {
                clip: bevy::math::Rect::from_corners(Vec2::ZERO, Vec2::new(100.0, 100.0)),
            },
            Name::new("CalculatedClipTestEntity"),
        ))
        .with_children(|parent| {
            spawn_keyboard_display_text(parent, port);
            spawn_button_test(parent);
            spawn_label_test(parent);
        });
}

fn spawn_keyboard_display_text(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    port: &Res<CurrentPort>,
) {
    // Keyboard display text directly
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
        bevy::text::TextBounds {
            width: Some(400.0),
            height: Some(200.0),
        },
        bevy::text::TextSpan("Test TextSpan Component".to_string()),
        bevy::prelude::TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.5),
        },
        bevy::prelude::UiAntiAlias::On,
        bevy::prelude::UiTargetCamera(Entity::PLACEHOLDER),
        bevy::prelude::ImageNode {
            image: Handle::default(),
            color: Color::srgb(0.2, 0.3, 0.5),  // Blue background instead of white
            flip_x: false,
            flip_y: false,
            image_mode: bevy::prelude::NodeImageMode::Auto,
            rect: None,
            texture_atlas: None,
        },
        Name::new("TextBoundsTestEntity"),
    ));
}

fn spawn_button_test(parent: &mut RelatedSpawnerCommands<ChildOf>) {
    // Button component for testing mutations
    parent.spawn((
        Node {
            width: Val::Px(100.0),
            height: Val::Px(40.0),
            margin: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.4, 0.6, 0.8)),
        Button,
        Outline::new(Val::Px(2.0), Val::Px(0.0), Color::srgb(1.0, 1.0, 0.0)), /* Yellow outline
                                                                               * for testing */
        FocusPolicy::Block,                          // For testing mutations
        Interaction::None,                           // For testing mutations
        ZIndex(0),                                   // For testing mutations
        bevy::ui::RelativeCursorPosition::default(), // For testing mutations
        Name::new("ButtonTestEntity"),
    ));
}

fn spawn_label_test(parent: &mut RelatedSpawnerCommands<ChildOf>) {
    // Label component for testing mutations
    parent.spawn((
        Text::new("Test Label"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)), // Yellow color
        Label,
        UiTargetCamera(Entity::PLACEHOLDER), // For testing mutations
        Name::new("LabelTestEntity"),
    ));
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
