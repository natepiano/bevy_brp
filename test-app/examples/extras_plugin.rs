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

#![allow(
    clippy::used_underscore_binding,
    reason = "false positive on enum struct variant fields"
)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use bevy::animation::AnimatedBy;
use bevy::animation::AnimationPlayer;
use bevy::animation::AnimationTargetId;
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
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::gizmos::GizmoAsset;
use bevy::gizmos::aabb::ShowAabbGizmo;
use bevy::gizmos::config::GizmoLineConfig;
use bevy::gizmos::retained::Gizmo;
use bevy::input::gamepad::Gamepad;
use bevy::input::gamepad::GamepadSettings;
use bevy::input::keyboard::Key;
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
use bevy::light::Skybox;
use bevy::light::SpotLightTexture;
use bevy::light::VolumetricFog;
use bevy::light::VolumetricLight;
use bevy::light::cluster::ClusterConfig;
use bevy::light::gizmos::ShowLightGizmo;
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
use bevy::picking::mesh_picking::MeshPickingSettings;
use bevy::picking::mesh_picking::ray_cast::RayCastVisibility;
use bevy::post_process::auto_exposure::AutoExposure;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::DepthOfField;
use bevy::post_process::effect_stack::ChromaticAberration;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::ChildOf;
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::camera::TemporalJitter;
use bevy::render::globals::GlobalsUniform;
use bevy::render::occlusion_culling::OcclusionCulling;
use bevy::render::render_resource::TextureViewDescriptor;
use bevy::render::render_resource::TextureViewDimension;
use bevy::render::view::ColorGrading;
use bevy::render::view::Msaa;
use bevy::render::view::window::screenshot::Screenshot;
use bevy::sprite::SpritePickingMode;
use bevy::sprite::SpritePickingSettings;
use bevy::sprite::Text2dShadow;
use bevy::sprite_render::Wireframe2dColor;
use bevy::sprite_render::Wireframe2dConfig;
use bevy::ui::BackgroundGradient;
use bevy::ui::BorderGradient;
use bevy::ui::BoxShadow;
use bevy::ui::CalculatedClip;
use bevy::ui::FocusPolicy;
use bevy::ui::Gradient;
use bevy::ui::Interaction;
use bevy::ui::InterpolationColorSpace;
use bevy::ui::LinearGradient;
use bevy::ui::MaxTrackSizingFunction;
use bevy::ui::MinTrackSizingFunction;
use bevy::ui::Outline;
use bevy::ui::RepeatedGridTrack;
use bevy::ui::UiTargetCamera;
use bevy::ui::ZIndex;
use bevy::ui::gradients::ColorStop;
use bevy::ui::widget::Button;
use bevy::ui::widget::Label;
use bevy::window::CursorIcon;
use bevy::window::MonitorSelection;
use bevy::window::PrimaryWindow;
use bevy::window::SystemCursorIcon;
use bevy::window::WindowPlugin;
use bevy::window::WindowPosition;
use bevy::world_serialization::WorldAsset;
use bevy::world_serialization::WorldAssetRoot;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

// asset paths
const CAUSTIC_LIGHTMAP_PATH: &str = "lightmaps/caustic_directional_texture.png";

// collection fixtures
const ENUM_KEYED_FIRST_VALUE: &str = "first";
const ENUM_KEYED_SECOND_VALUE: &str = "second";
const ENUM_KEYED_THIRD_VALUE: &str = "third";
const MAP_ENEMY_KEY: &str = "enemy";
const MAP_ENEMY_TRANSFORM: Vec3 = Vec3::new(-5.0, 0.0, -10.0);
const MAP_HUMIDITY_KEY: &str = "humidity";
const MAP_HUMIDITY_VALUE: f32 = 65.0;
const MAP_KEY_ONE: &str = "key1";
const MAP_KEY_THREE: &str = "key3";
const MAP_KEY_TWO: &str = "key2";
const MAP_PLAYER_KEY: &str = "player";
const MAP_PLAYER_TRANSFORM: Vec3 = Vec3::new(10.0, 0.0, 5.0);
const MAP_POWERUP_KEY: &str = "powerup";
const MAP_POWERUP_SCALE: Vec3 = Vec3::splat(2.0);
const MAP_POWERUP_TRANSFORM: Vec3 = Vec3::new(0.0, 5.0, 0.0);
const MAP_PRESSURE_KEY: &str = "pressure";
const MAP_PRESSURE_VALUE: f32 = 1013.25;
const MAP_TEMPERATURE_KEY: &str = "temperature";
const MAP_TEMPERATURE_VALUE: f32 = 23.5;
const MAP_VALUE_ONE: &str = "value1";
const MAP_VALUE_THREE: &str = "value3";
const MAP_VALUE_TWO: &str = "value2";
const SIMPLE_SET_HELLO: &str = "hello";
const SIMPLE_SET_TEST: &str = "test";
const SIMPLE_SET_WORLD: &str = "world";
const STRUCT_SET_FIRST_ITEM: &str = "first_item";
const STRUCT_SET_SECOND_ITEM: &str = "second_item";
const STRUCT_SET_THIRD_ITEM: &str = "third_item";

// entity names
const AMBIENT_LIGHT_TEST_ENTITY_NAME: &str = "AmbientLightTestEntity";
const ANIMATION_GRAPH_HANDLE_AND_PLAYER_AND_TRANSITIONS_TEST_ENTITY_NAME: &str =
    "AnimationGraphHandleAndPlayerAndTransitionsTestEntity";
const ANIMATION_TARGET_NAME: &str = "test_target";
const ANIMATION_TARGET_TEST_ENTITY_NAME: &str = "AnimationTargetTestEntity";
const BACKGROUND_GRADIENT_TEST_ENTITY_NAME: &str = "BackgroundGradientTestEntity";
const BORDER_GRADIENT_TEST_ENTITY_NAME: &str = "BorderGradientTestEntity";
const BORDER_RADIUS_TEST_ENTITY_NAME: &str = "BorderRadiusTestEntity";
const BOX_SHADOW_TEST_ENTITY_NAME: &str = "BoxShadowTestEntity";
const BUTTON_TEST_ENTITY_NAME: &str = "ButtonTestEntity";
const CALCULATED_CLIP_TEST_ENTITY_NAME: &str = "CalculatedClipTestEntity";
const CASCADES_FRUSTA_TEST_ENTITY_NAME: &str = "CascadesFrustaTestEntity";
const CLUSTERED_DECAL_TEST_ENTITY_NAME: &str = "ClusteredDecalTestEntity";
const CLUSTER_CONFIG_TEST_ENTITY_NAME: &str = "ClusterConfigTestEntity";
const COMPLEX_TRANSFORM_ENTITY_NAME: &str = "ComplexTransformEntity";
const CURSOR_ICON_TEST_ENTITY_NAME: &str = "CursorIconTestEntity";
const DIRECTIONAL_LIGHT_TEST_ENTITY_NAME: &str = "DirectionalLightTestEntity";
const DISTANCE_FOG_TEST_ENTITY_NAME: &str = "DistanceFogTestEntity";
const ENVIRONMENT_MAP_LIGHT_TEST_ENTITY_NAME: &str = "EnvironmentMapLightTestEntity";
const EXTENDED_DECAL_MATERIAL_TEST_ENTITY_NAME: &str = "ExtendedDecalMaterialTestEntity";
const FOG_VOLUME_TEST_ENTITY_NAME: &str = "FogVolumeTestEntity";
const GAMEPAD_SETTINGS_TEST_ENTITY_NAME: &str = "GamepadSettingsTestEntity";
const GAMEPAD_TEST_ENTITY_NAME: &str = "GamepadTestEntity";
const GENERATED_ENVIRONMENT_MAP_LIGHT_TEST_ENTITY_NAME: &str =
    "GeneratedEnvironmentMapLightTestEntity";
const GLTF_EXTRAS_TEST_ENTITY_NAME: &str = "GltfExtrasTestEntity";
const GLTF_MATERIAL_EXTRAS_TEST_ENTITY_NAME: &str = "GltfMaterialExtrasTestEntity";
const GLTF_MATERIAL_NAME_TEST_ENTITY_NAME: &str = "GltfMaterialNameTestEntity";
const GLTF_MESH_EXTRAS_TEST_ENTITY_NAME: &str = "GltfMeshExtrasTestEntity";
const GLTF_MESH_NAME_TEST_ENTITY_NAME: &str = "GltfMeshNameTestEntity";
const GLTF_SCENE_EXTRAS_TEST_ENTITY_NAME: &str = "GltfSceneExtrasTestEntity";
const IRRADIANCE_VOLUME_TEST_ENTITY_NAME: &str = "IrradianceVolumeTestEntity";
const LABEL_TEST_ENTITY_NAME: &str = "LabelTestEntity";
const LIGHTMAP_TEST_ENTITY_NAME: &str = "LightmapTestEntity";
const LIGHT_PROBE_TEST_ENTITY_NAME: &str = "LightProbeTestEntity";
const MAIN_PASS_RESOLUTION_OVERRIDE_TEST_ENTITY_NAME: &str = "MainPassResolutionOverrideTestEntity";
const MANUAL_TEXTURE_VIEW_HANDLE_TEST_ENTITY_NAME: &str = "ManualTextureViewHandleTestEntity";
const MESH_MATERIAL2D_TEST_ENTITY_NAME: &str = "MeshMaterial2dTestEntity";
const MESH_MORPH_WEIGHTS_TEST_ENTITY_NAME: &str = "MeshMorphWeightsTestEntity";
const MORPH_WEIGHTS_TEST_ENTITY_NAME: &str = "MorphWeightsTestEntity";
const NESTED_CONFIG_ENUM_ALWAYS_ENTITY_NAME: &str = "NestedConfigEnumAlwaysEntity";
const NESTED_CONFIG_ENUM_CONDITIONAL_ENTITY_NAME: &str = "NestedConfigEnumConditionalEntity";
const NESTED_CONFIG_ENUM_NEVER_ENTITY_NAME: &str = "NestedConfigEnumNeverEntity";
const NOT_SHADOW_CASTER_TEST_ENTITY_NAME: &str = "NotShadowCasterTestEntity";
const NOT_SHADOW_RECEIVER_TEST_ENTITY_NAME: &str = "NotShadowReceiverTestEntity";
const NO_FRUSTUM_CULLING_TEST_ENTITY_NAME: &str = "NoFrustumCullingTestEntity";
const OCCLUSION_CULLING_TEST_ENTITY_NAME: &str = "OcclusionCullingTestEntity";
const OPTION_TEST_ENUM_TRANSFORM_ENTITY_NAME: &str = "OptionTestEnumTransformEntity";
const OPTION_TEST_ENUM_VEC2_ENTITY_NAME: &str = "OptionTestEnumVec2Entity";
const PLAYBACK_SETTINGS_TEST_ENTITY_NAME: &str = "PlaybackSettingsTestEntity";
const POINT_LIGHT_TEST_ENTITY_NAME: &str = "PointLightTestEntity";
const RETAINED_GIZMO_TEST_ENTITY_NAME: &str = "RetainedGizmoTestEntity";
const SCALED_ENTITY_NAME: &str = "ScaledEntity";
const SCENE_ENTITY1_NAME: &str = "SceneEntity1";
const SCENE_ENTITY2_NAME: &str = "SceneEntity2";
const SCREENSHOT_TEST_ENTITY_NAME: &str = "ScreenshotTestEntity";
const SIMPLE_NESTED_ENUM_ENTITY_NAME: &str = "SimpleNestedEnumEntity";
const SIMPLE_NESTED_ENUM_STRUCT_ENTITY_NAME: &str = "SimpleNestedEnumStructEntity";
const SIMPLE_NESTED_ENUM_TRANSFORM_ENTITY_NAME: &str = "SimpleNestedEnumTransformEntity";
const SIMPLE_NESTED_ENUM_VEC2_ENTITY_NAME: &str = "SimpleNestedEnumVec2Entity";
const SIMPLE_SET_ENTITY_NAME: &str = "SimpleSetEntity";
const SKINNED_MESH_TEST_ENTITY_NAME: &str = "SkinnedMeshTestEntity";
const SKYBOX_TEST_ENTITY_NAME: &str = "SkyboxTestEntity";
const SMAA_TEST_ENTITY_NAME: &str = "SmaaTestEntity";
const SPATIAL_LISTENER_TEST_ENTITY_NAME: &str = "SpatialListenerTestEntity";
const SPOT_LIGHT_TEST_ENTITY_NAME: &str = "SpotLightTestEntity";
const TAB_GROUP_TEST_ENTITY_NAME: &str = "TabGroupTestEntity";
const TAB_INDEX_TEST_ENTITY_NAME: &str = "TabIndexTestEntity";
const TEMPORAL_ANTI_ALIASING_TEST_ENTITY_NAME: &str = "TemporalAntiAliasingTestEntity";
const TEST_ARRAY_FIELD_ENTITY_NAME: &str = "TestArrayFieldEntity";
const TEST_ARRAY_TRANSFORMS_ENTITY_NAME: &str = "TestArrayTransformsEntity";
const TEST_COLLECTION_ENTITY_NAME: &str = "TestCollectionEntity";
const TEST_COMPLEX_ENTITY_NAME: &str = "TestComplexEntity";
const TEST_COMPLEX_TUPLE_ENTITY_NAME: &str = "TestComplexTupleEntity";
const TEST_ENTITY1_NAME: &str = "TestEntity1";
const TEST_ENUM_KEYED_MAP_ENTITY_NAME: &str = "TestEnumKeyedMapEntity";
const TEST_ENUM_WITH_ARRAY_ENTITY_NAME: &str = "TestEnumWithArrayEntity";
const TEST_GAMEPAD_NAME: &str = "TestGamepad";
const TEST_MAP_ENTITY_NAME: &str = "TestMapEntity";
const TEST_MIXED_MUTABILITY_ARRAY_ENTITY_NAME: &str = "TestMixedMutabilityArrayEntity";
const TEST_MIXED_MUTABILITY_ENUM_ENTITY_NAME: &str = "TestMixedMutabilityEnumEntity";
const TEST_MIXED_MUTABILITY_TUPLE_ENTITY_NAME: &str = "TestMixedMutabilityTupleEntity";
const TEST_MIXED_MUTABILITY_VEC_ENTITY_NAME: &str = "TestMixedMutabilityVecEntity";
const TEST_SPRITE_NAME: &str = "TestSprite";
const TEST_STRUCT_NO_SER_DE_ENTITY_NAME: &str = "TestStructNoSerDeEntity";
const TEST_TUPLE_FIELD_ENTITY_NAME: &str = "TestTupleFieldEntity";
const TEST_TUPLE_STRUCT_ENTITY_NAME: &str = "TestTupleStructEntity";
const TEST_VARIANT_CHAIN_ENTITY_NAME: &str = "TestVariantChainEntity";
const TEXT2D_TEST_ENTITY_NAME: &str = "Text2dTestEntity";
const TEXT_BOUNDS_TEST_ENTITY_NAME: &str = "TextBoundsTestEntity";
const TEXT_INPUT_TEST_ENTITY_NAME: &str = "TextInputTestEntity";
const VISIBLE_ENTITY_NAME: &str = "VisibleEntity";
const WIREFRAME2D_COLOR_TEST_ENTITY_NAME: &str = "Wireframe2dColorTestEntity";
const WORLD_ASSET_ROOT_TEST_ENTITY_NAME: &str = "WorldAssetRootTestEntity";
const WRAPPER_ENUM_OPTIONAL_ENTITY_NAME: &str = "WrapperEnumOptionalEntity";
const WRAPPER_ENUM_SIMPLE_ENTITY_NAME: &str = "WrapperEnumSimpleEntity";

// enum fixtures
const BOTTOM_ENUM_VARIANT_A_VALUE: u32 = 999;
const COMPLEX_COMPONENT_OPTIONAL_VALUE: f32 = 50.0;
const COMPLEX_COMPONENT_POINTS: [Vec3; 2] = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)];
const COMPLEX_COMPONENT_RANGE: (f32, f32) = (0.0, 100.0);
const COMPLEX_COMPONENT_TRANSFORM: Vec3 = Vec3::new(5.0, 10.0, 15.0);
const COMPLEX_TUPLE_NESTED_LABEL: &str = "nested";
const COMPLEX_TUPLE_NESTED_SCALAR: f32 = 3.0;
const COMPLEX_TUPLE_NESTED_VEC2: Vec2 = Vec2::new(5.0, 10.0);
const COMPLEX_TUPLE_TRANSFORM: Vec3 = Vec3::new(10.0, 20.0, 30.0);
const COMPLEX_TUPLE_VEC3: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const NESTED_CONFIG_CONDITIONAL_VALUE: u32 = 42;
const OPTION_ENUM_TRANSFORM_SCALE: Vec3 = Vec3::splat(3.0);
const OPTION_ENUM_VEC2: Vec2 = Vec2::new(100.0, 200.0);
const SIMPLE_NESTED_STRUCT_POSITION: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const SIMPLE_NESTED_STRUCT_SCALE: f32 = 2.5;
const SIMPLE_NESTED_VEC2: Vec2 = Vec2::new(10.0, 20.0);
const TEST_ENUM_ARRAY_POINTS: [Vec2; 3] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(2.0, 2.0),
];
const TEST_STRUCT_NO_SER_DE_NAME: &str = "test_struct";
const TEST_STRUCT_NO_SER_DE_VALUE: f32 = 123.45;
const VARIANT_CHAIN_LABEL: &str = "test_field";
const VARIANT_CHAIN_MAGNITUDE: f32 = 42.5;
const WRAPPER_OPTIONAL_ROTATION_RADIANS: f32 = 1.0;
const WRAPPER_SIMPLE_VEC2: Vec2 = Vec2::new(50.0, 75.0);

// gltf fixtures
const GLTF_EXTRAS_VALUE: &str = "test gltf extras";
const GLTF_MATERIAL_EXTRAS_VALUE: &str = "test material extras";
const GLTF_MATERIAL_NAME_VALUE: &str = "test material name";
const GLTF_MESH_EXTRAS_VALUE: &str = "test mesh extras";
const GLTF_MESH_NAME_VALUE: &str = "test_mesh_name";
const GLTF_SCENE_EXTRAS_VALUE: &str = "test scene extras";

// lighting
const AABB_MAX: Vec3 = Vec3::new(0.5, 0.5, 0.5);
const AABB_MIN: Vec3 = Vec3::new(-0.5, -0.5, -0.5);
const AABB_TRANSFORM: Vec3 = Vec3::new(-2.0, 1.0, 0.0);
const DIRECTIONAL_LIGHT_ILLUMINANCE: f32 = 10000.0;
const DISTANCE_FOG_COLOR: Color = Color::srgba(0.35, 0.48, 0.66, 1.0);
const DISTANCE_FOG_END: f32 = 20.0;
const DISTANCE_FOG_EXPONENT: f32 = 8.0;
const DISTANCE_FOG_LIGHT_COLOR: Color = Color::srgba(1.0, 0.95, 0.85, 0.5);
const DISTANCE_FOG_START: f32 = 5.0;
const EXTENDED_DECAL_TRANSFORM: Vec3 = Vec3::new(0.0, 2.0, 0.0);
const LIGHTMAP_UV_RECT: bevy::math::Rect = bevy::math::Rect::new(0.0, 0.0, 1.0, 1.0);
const NOT_SHADOW_RECEIVER_TRANSFORM: Vec3 = Vec3::new(2.0, 1.0, 0.0);
const POINT_LIGHT_GIZMO_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);
const POINT_LIGHT_INTENSITY: f32 = 1500.0;
const POINT_LIGHT_TRANSFORM: Vec3 = Vec3::new(4.0, 8.0, 4.0);
const SHADOW_GIZMO_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);
const SPOT_LIGHT_INNER_ANGLE: f32 = 0.6;
const SPOT_LIGHT_INTENSITY: f32 = 2000.0;
const SPOT_LIGHT_OUTER_ANGLE: f32 = 0.8;
const SPOT_LIGHT_RADIUS: f32 = 0.1;
const SPOT_LIGHT_RANGE: f32 = 10.0;
const SPOT_LIGHT_TRANSFORM: Vec3 = Vec3::new(0.0, 4.0, 0.0);

// mixed mutability fixtures
const MIXED_MUTABILITY_ARRAY_ONE_SUFFIX: &str = "array_1";
const MIXED_MUTABILITY_ARRAY_ZERO_SUFFIX: &str = "array_0";
const MIXED_MUTABILITY_ARC_ITEMS: [u8; 5] = [1, 2, 3, 4, 5];
const MIXED_MUTABILITY_ENUM_NAME: &str = "enum_multiple";
const MIXED_MUTABILITY_ENUM_SUFFIX: &str = "enum";
const MIXED_MUTABILITY_ENUM_VALUE: f32 = 123.45;
const MIXED_MUTABILITY_FLOAT: f32 = 42.5;
const MIXED_MUTABILITY_NESTED_VALUE: f32 = 100.0;
const MIXED_MUTABILITY_TUPLE_LABEL: &str = "tuple_string";
const MIXED_MUTABILITY_TUPLE_SUFFIX: &str = "tuple";
const MIXED_MUTABILITY_TUPLE_VALUE: f32 = 99.9;
const MIXED_MUTABILITY_VEC_ONE_SUFFIX: &str = "vec_1";
const MIXED_MUTABILITY_VEC_TWO_SUFFIX: &str = "vec_2";
const MIXED_MUTABILITY_VEC_ZERO_SUFFIX: &str = "vec_0";

// render fixtures
const AMBIENT_CAMERA_ORDER: isize = 2;
const AMBIENT_LIGHT_TRANSFORM: Vec3 = Vec3::new(100.0, 100.0, 100.0);
const GENERATED_ENVIRONMENT_INTENSITY: f32 = 1000.0;
const GIZMO_LINE_WIDTH: f32 = 2.0;
const GIZMO_SPHERE_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);
const GIZMO_SPHERE_RADIUS: f32 = 1.0;
const MAIN_PASS_RESOLUTION_OVERRIDE_SIZE: bevy::math::UVec2 = bevy::math::UVec2::new(1920, 1080);
const MANUAL_TEXTURE_VIEW_HANDLE_ID: u32 = 42;
const MESH_MORPH_WEIGHTS: [f32; 3] = [0.5, 1.0, 0.75];
const TEXT2D_CONTENT: &str = "Hello Text2d";
const TEXT2D_TRANSFORM: Vec3 = Vec3::new(50.0, 50.0, 0.0);

// scene fixtures
const BRP_EXTRAS_TEST_TITLE: &str = "BRP Extras Test";
const COMPLEX_ENTITY_SCALE: Vec3 = Vec3::new(0.5, 1.5, 2.0);
const COMPLEX_ENTITY_TRANSLATION: Vec3 = Vec3::new(10.0, 20.0, 30.0);
const COMPLEX_ROTATION_DIVISOR: f32 = 4.0;
const SCALED_ENTITY_SCALE: Vec3 = Vec3::splat(2.0);
const SCENE_ENTITY_ONE_TRANSLATION: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const SCENE_ENTITY_TWO_TRANSLATION: Vec3 = Vec3::new(4.0, 5.0, 6.0);
const SKYBOX_BRIGHTNESS: f32 = 1000.0;
const SKYBOX_FACE_COUNT: u32 = 6;
const SKYBOX_FACE_DATA: [u8; 4] = [128, 128, 128, 255];
const SKYBOX_FACE_SIZE: u32 = 1;
const SPRITE_ALPHA_THRESHOLD: f32 = 0.1;
const SPRITE_COLOR: Color = Color::srgb(1.0, 0.5, 0.25);
const SPRITE_LAYER: usize = 1;
const SPRITE_POSITION: Vec3 = Vec3::new(100.0, 100.0, 0.0);
const SPRITE_SIZE: Vec2 = Vec2::new(64.0, 64.0);
const TEST_ARRAY_FIELD_VALUES: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
const TEST_ARRAY_FIELD_VERTICES: [Vec2; 3] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(0.5, 1.0),
];
const TEST_CONFIG_LABEL: &str = "test config";
const TEST_CONFIG_LEVEL: f32 = 100.0;
const TEST_ENTITY_TRANSLATION: Vec3 = Vec3::new(1.0, 2.0, 3.0);
const TEST_TUPLE_COLOR_RGB: (u8, u8, u8) = (255, 128, 64);
const TEST_TUPLE_COORDS: (f32, f32) = (10.0, 20.0);
const TEST_TUPLE_STRUCT_LABEL: &str = "test";
const TEST_TUPLE_STRUCT_VALUE: f32 = 42.0;
const VISIBILITY_END_MARGIN: std::ops::Range<f32> = 90.0..100.0;
const VISIBILITY_START_MARGIN: std::ops::Range<f32> = 0.0..10.0;
const WIREFRAME_2D_COLOR: Color = Color::hsla(180.0, 0.5, 0.5, 1.0);

// ui fixtures
const BACKGROUND_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);
const BACKGROUND_GRADIENT_END_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);
const BACKGROUND_GRADIENT_START_COLOR: Color = Color::srgb(0.0, 1.0, 0.0);
const BORDER_GRADIENT_END_COLOR: Color = Color::srgb(0.0, 0.0, 1.0);
const BORDER_GRADIENT_START_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);
const BORDER_RADIUS: f32 = 10.0;
const BOX_SHADOW_BACKGROUND: Color = Color::srgb(0.8, 0.9, 1.0);
const BOX_SHADOW_BLUR_RADIUS: f32 = 10.0;
const BOX_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const BOX_SHADOW_HEIGHT: f32 = 100.0;
const BOX_SHADOW_OFFSET_X: f32 = 5.0;
const BOX_SHADOW_OFFSET_Y: f32 = 5.0;
const BOX_SHADOW_SAMPLES: u32 = 4;
const BOX_SHADOW_SPREAD_RADIUS: f32 = 2.0;
const BOX_SHADOW_WIDTH: f32 = 150.0;
const BUTTON_BACKGROUND: Color = Color::srgb(0.4, 0.6, 0.8);
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_OUTLINE_COLOR: Color = Color::srgb(1.0, 1.0, 0.0);
const BUTTON_OUTLINE_WIDTH: f32 = 2.0;
const BUTTON_WIDTH: f32 = 100.0;
const CALCULATED_CLIP_MAX: Vec2 = Vec2::new(100.0, 100.0);
const GRID_COLUMN_MAX_PX: f32 = 100.0;
const GRID_COLUMN_MIN_PX: f32 = 50.0;
const GRID_COLUMN_TRACK_COUNT: u16 = 1;
const GRID_ROW_TRACK_COUNT: u16 = 2;
const GRADIENT_START_PERCENT: f32 = 0.0;
const KEYBOARD_DISPLAY_FONT_SIZE: f32 = 20.0;
const KEYBOARD_TEXT_BACKGROUND: Color = Color::srgba(0.0, 0.0, 0.0, 0.3);
const LABEL_COLOR: Color = Color::srgb(1.0, 1.0, 0.0);
const LABEL_FONT_SIZE: f32 = 16.0;
const LABEL_TEXT: &str = "Test Label";
const TEST_CAMERA_ORDER: isize = 1;
const TEST_CAMERA_TRANSFORM: Vec3 = Vec3::new(0.0, 5.0, 10.0);
const TEXT_BOUNDS_HEIGHT: f32 = 200.0;
const TEXT_BOUNDS_WIDTH: f32 = 400.0;
const TEXT_CONTAINER_BACKGROUND: Color = Color::srgb(0.2, 0.3, 0.5);
const TEXT_CONTAINER_PADDING: f32 = 20.0;
const TEXT_INPUT_BACKGROUND: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const TEXT_INPUT_BOUNDS_HEIGHT: f32 = 100.0;
const TEXT_INPUT_BOUNDS_WIDTH: f32 = 400.0;
const TEXT_INPUT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const TEXT_INPUT_FONT_SIZE: f32 = 18.0;
const TEXT_INPUT_OUTLINE_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);
const TEXT_INPUT_OUTLINE_WIDTH: f32 = 1.0;
const TEXT_SHADOW_OFFSET: Vec2 = Vec2::new(2.0, 2.0);
const TEXT_SPAN_CONTENT: &str = "Test TextSpan Component";
const UI_CAMERA_ORDER: isize = 0;
const UI_FILL_PERCENT: f32 = 100.0;
const UI_IMAGE_COLOR: Color = Color::srgb(0.2, 0.3, 0.5);
const UI_MARGIN: f32 = 10.0;
const UI_NODE_SIZE: f32 = 200.0;

// window
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_WIDTH: u32 = 800;

/// Resource to track keyboard input history
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
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
    #[reflect(ignore)]
    press_time:           Option<Instant>,
    /// Duration between press and release in milliseconds
    last_duration_ms:     Option<u64>,
    /// Completion state for the last key press
    completion_state:     CompletionState,
}

#[derive(Default, Reflect)]
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

fn collect_modifier_labels(keys: &[String]) -> Vec<String> {
    let mut modifiers = Vec::new();

    for key in keys {
        if let Ok(modifier) = ModifierKey::try_from(key.as_str()) {
            let label = modifier.label();
            if !modifiers.iter().any(|existing| existing == label) {
                modifiers.push(label.to_string());
            }
        }
    }

    modifiers
}

/// Marker component for the keyboard input display text
#[derive(Component)]
struct KeyboardDisplayText;

/// Marker component for the text input display
#[derive(Component)]
struct TextInputDisplay;

/// Resource to store accumulated text input content, queryable via BRP
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct TextInputContent {
    text: String,
}

/// Test resource for BRP operations
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct TestConfigResource {
    level:        f32,
    label:        String,
    toggle_state: ToggleState,
}

/// Test resource for runtime statistics
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct RuntimeStatsResource {
    frame_count:  u32,
    total_time:   f32,
    runtime_mode: RuntimeMode,
}

#[derive(Default, Reflect)]
enum RuntimeMode {
    Debug,
    #[default]
    Standard,
}

#[derive(Default, Reflect)]
enum ToggleState {
    #[default]
    Disabled,
    Enabled,
}

/// Simple `HashSet` test component with just strings
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct SimpleSetComponent {
    string_set: HashSet<String>,
}

/// Test component with `HashMap` for testing map mutations
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMapComponent {
    /// `String` to `String` map
    strings:    HashMap<String, String>,
    /// `String` to `f32` map
    values:     HashMap<String, f32>,
    /// `String` to `Transform` map (complex nested type)
    transforms: HashMap<String, Transform>,
}

/// Test component with enum-keyed `HashMap` (`NotMutable` due to complex key)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestEnumKeyedMap {
    /// Enum to String map (should be `NotMutable` due to complex key)
    enum_keyed: HashMap<SimpleTestEnum, String>,
}

/// Simple test enum for `HashMap` key testing
#[derive(Default, Reflect, Hash, Eq, PartialEq, Clone)]
enum SimpleTestEnum {
    #[default]
    Variant1,
    Variant2,
    Variant3,
}

/// Test component struct for testing
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestStructNoSerDe {
    value:        f32,
    name:         String,
    toggle_state: ToggleState,
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

/// Simple nested enum for testing enum recursion - like `Option<Vec2>`
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum SimpleNestedEnum {
    #[default]
    None,
    /// This variant contains a `Vec2` - should generate nested paths
    WithVec2(Vec2),
    /// This variant contains a `Transform` - should generate deeply nested paths
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
    /// `Option<Vec2>` - should generate nested paths through `Some` variant
    MaybeVec2(Option<Vec2>),
    /// `Option<Transform>` - should generate deeply nested paths through `Some` variant
    MaybeTransform(Option<Transform>),
}

/// Test concrete enum that wraps other enums (simulating generics)
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum WrapperEnum {
    #[default]
    Empty,
    /// Wrapper with nested enum - should recurse into `SimpleNestedEnum`'s paths
    Simple(SimpleNestedEnum),
    /// Option wrapper - should recurse through `Option<SimpleNestedEnum>`
    Optional(Option<SimpleNestedEnum>),
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
    label:       String,
    /// Another regular field
    magnitude:   f32,
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
    /// Variant with array of `Vec2`
    Vec2([Vec2; 3]),
    /// Variant with array of `f32`
    Float([f32; 4]),
    /// Struct variant with array field
    Struct { points: [Vec3; 2], scale: f32 },
}

/// Test component with array field
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestArrayField {
    /// Fixed-size array field
    vertices: [Vec2; 3],
    /// Another array field
    values:   [f32; 4],
}

/// Test component with array of Transforms
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestArrayTransforms {
    /// Array of `Transform` components
    transforms: [Transform; 2],
}

/// Test component with tuple field
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestTupleField {
    /// Tuple field with two elements
    coords:    (f32, f32),
    /// Tuple field with three elements
    color_rgb: (u8, u8, u8),
}

/// Test tuple struct component
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestTupleStruct(f32, String, ToggleState);

/// Test component with complex tuple types for testing tuple recursion
#[derive(Component, Reflect)]
#[reflect(Component)]
struct TestComplexTuple {
    /// Tuple with complex types that should recurse
    complex: (Transform, Vec3),
    /// Nested tuple with both simple and complex types
    nested:  (Vec2, (f32, String)),
}

/// Core type with mixed mutability for `mutability_reason` testing
/// Simplified version with reduced nesting depth
#[derive(Default, Reflect)]
struct TestMixedMutabilityCore {
    /// Mutable string field
    mutable_string: String,

    /// Mutable float field
    mutable_float: f32,

    /// Not mutable field - Arc type
    not_mutable_arc: std::sync::Arc<String>,

    /// Partially mutable field - simple nested struct
    partially_mutable_nested: TestPartiallyMutableNested,
}

/// Vec parent containing mixed mutability items
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMixedMutabilityVec {
    /// Vec of mixed mutability items
    items: Vec<TestMixedMutabilityCore>,
}

/// Array parent containing mixed mutability items
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMixedMutabilityArray {
    /// Fixed-size array of mixed mutability items
    items: [TestMixedMutabilityCore; 2],
}

/// `TupleStruct` parent containing mixed mutability item
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestMixedMutabilityTuple(TestMixedMutabilityCore, f32, String);

/// Enum parent containing mixed mutability variants
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum TestMixedMutabilityEnum {
    #[default]
    None,
    /// Variant with mixed mutability struct
    Core(TestMixedMutabilityCore),
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
    mutable_value: f32,

    /// Not mutable - Arc type without serialization
    not_mutable_arc: std::sync::Arc<Vec<u8>>,
}

impl Default for TestComplexTuple {
    fn default() -> Self {
        Self {
            complex: (Transform::default(), Vec3::ZERO),
            nested:  (Vec2::ZERO, (0.0, String::new())),
        }
    }
}

/// Complex nested component with various field types
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct TestComplexComponent {
    /// Nested struct field (will have .transform.translation.x paths)
    transform:          Transform,
    /// Enum field
    simple_nested_enum: SimpleNestedEnum,
    /// Array field
    points:             [Vec3; 2],
    /// Tuple field
    range:              (f32, f32),
    /// Option field
    optional_value:     Option<f32>,
}

/// Test component with List and Set collection types containing complex elements
#[derive(Component, Reflect)]
#[reflect(Component)]
struct TestCollectionComponent {
    /// `Vec<Transform>` - should trigger `ListMutationBuilder` with complex recursion
    transform_list: Vec<Transform>,
    /// `HashSet<String>` - should trigger `SetMutationBuilder`
    struct_set:     HashSet<String>,
}

impl Default for TestCollectionComponent {
    fn default() -> Self {
        let mut struct_set = HashSet::new();
        struct_set.insert(STRUCT_SET_FIRST_ITEM.to_string());
        struct_set.insert(STRUCT_SET_SECOND_ITEM.to_string());
        struct_set.insert(STRUCT_SET_THIRD_ITEM.to_string());

        Self {
            transform_list: vec![
                Transform::from_translation(COMPLEX_ENTITY_TRANSLATION),
                Transform::from_rotation(Quat::from_rotation_y(
                    std::f32::consts::PI / COMPLEX_ROTATION_DIVISOR,
                )),
                Transform::from_scale(COMPLEX_ENTITY_SCALE),
            ],
            struct_set,
        }
    }
}

fn main() {
    let brp_extras_plugin = BrpExtrasPlugin::new().port_in_title(PortDisplay::Always);
    let (port, _) = brp_extras_plugin.get_effective_port();

    info!("Starting BRP Extras Test on port {port}");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: BRP_EXTRAS_TEST_TITLE.to_string(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                focused: false,
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_extras_plugin)
        .add_plugins(MeshPickingPlugin)
        .init_resource::<KeyboardInputHistory>()
        .init_resource::<TextInputContent>()
        .init_resource::<GlobalsUniform>()
        .insert_resource(CurrentPort(port))
        .insert_resource(WireframeConfig {
            global: true,
            default_color: Color::WHITE,
            ..default()
        })
        .insert_resource(Wireframe2dConfig {
            global:        true,
            default_color: Color::WHITE,
        })
        .insert_resource(TestConfigResource {
            level:        TEST_CONFIG_LEVEL,
            label:        TEST_CONFIG_LABEL.to_string(),
            toggle_state: ToggleState::Enabled,
        })
        .insert_resource(RuntimeStatsResource {
            frame_count:  0,
            total_time:   0.0,
            runtime_mode: RuntimeMode::Standard,
        })
        .insert_resource(MeshPickingSettings {
            require_markers:     false,
            ray_cast_visibility: RayCastVisibility::VisibleInView,
        })
        .insert_resource(SpritePickingSettings {
            require_markers: false,
            picking_mode:    SpritePickingMode::AlphaThreshold(SPRITE_ALPHA_THRESHOLD),
        })
        .insert_resource(InputFocus::default())
        .add_systems(
            Startup,
            (setup_test_entities, setup_ui, minimize_window_on_start),
        )
        .add_systems(PostStartup, (setup_skybox_test, setup_scene_test))
        .add_systems(
            Update,
            (
                track_keyboard_input,
                update_keyboard_display,
                handle_text_input,
            ),
        )
        .run();
}

/// Resource to store the current port
#[derive(Resource)]
struct CurrentPort(u16);

/// Minimize the window immediately on startup
/// On Linux/Wayland, minimizing causes a swap chain timeout panic because the
/// compositor stops providing frames. Skip minimization on Linux.
#[cfg(target_os = "linux")]
fn minimize_window_on_start(windows: Query<&mut Window, With<PrimaryWindow>>) {
    let _ = windows.iter().count();
}

#[cfg(not(target_os = "linux"))]
fn minimize_window_on_start(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}

/// Setup a skybox with a simple cube texture for testing mutations
fn setup_skybox_test(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Create a simple 1x6 pixel image (6 faces stacked vertically)
    // This will be reinterpreted as a cube texture
    let size = SKYBOX_FACE_SIZE;
    let mut data = Vec::new();
    for _ in 0..SKYBOX_FACE_COUNT {
        // Each face is 1x1 pixel, gray color
        data.extend_from_slice(&SKYBOX_FACE_DATA);
    }

    let mut image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width:                 size,
            height:                size * SKYBOX_FACE_COUNT, // Stack 6 faces vertically
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Reinterpret as cube texture (height/width = 6)
    #[allow(clippy::expect_used, reason = "infallible for valid cube textures")]
    image
        .reinterpret_stacked_2d_as_array(image.height() / image.width())
        .expect("Failed to reinterpret image as cube texture array");
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    let image_handle = images.add(image);

    // Spawn an entity with Skybox for testing mutations
    commands.spawn((
        Skybox {
            image:      Some(image_handle),
            brightness: SKYBOX_BRIGHTNESS,
            rotation:   Quat::IDENTITY,
        },
        Name::new(SKYBOX_TEST_ENTITY_NAME),
    ));

    info!("Skybox test entity created with cube texture");
}

/// Setup a simple scene with `WorldAssetRoot` for testing
fn setup_scene_test(mut commands: Commands, mut scenes: ResMut<Assets<WorldAsset>>) {
    // Create a simple scene with a few test entities
    let mut scene_world = World::new();

    // Add some simple entities to the scene
    scene_world.spawn((
        Transform::from_translation(SCENE_ENTITY_ONE_TRANSLATION),
        Name::new(SCENE_ENTITY1_NAME),
    ));

    scene_world.spawn((
        Transform::from_translation(SCENE_ENTITY_TWO_TRANSLATION),
        Name::new(SCENE_ENTITY2_NAME),
    ));

    let scene = WorldAsset::new(scene_world);
    let scene_handle = scenes.add(scene);

    // Spawn entity with WorldAssetRoot for testing
    commands.spawn((
        WorldAssetRoot(scene_handle),
        Name::new(WORLD_ASSET_ROOT_TEST_ENTITY_NAME),
    ));

    info!("WorldAssetRoot test entity created");
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
    commands.spawn((
        Transform::from_translation(TEST_ENTITY_TRANSLATION),
        Name::new(TEST_ENTITY1_NAME),
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
        Name::new(COMPLEX_TRANSFORM_ENTITY_NAME),
    ));

    // Entity with visibility component
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new(VISIBLE_ENTITY_NAME),
        Visibility::default(),
        VisibilityRange {
            start_margin: VISIBILITY_START_MARGIN,
            end_margin:   VISIBILITY_END_MARGIN,
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
            color: SPRITE_COLOR,
            custom_size: Some(SPRITE_SIZE),
            flip_x: false,
            flip_y: false,
            ..default()
        },
        Transform::from_translation(SPRITE_POSITION),
        Name::new(TEST_SPRITE_NAME),
        RenderLayers::layer(SPRITE_LAYER),
    ));

    // Entity with Wireframe2dColor for testing mutations
    commands.spawn((
        Wireframe2dColor {
            color: WIREFRAME_2D_COLOR,
        },
        Name::new(WIREFRAME2D_COLOR_TEST_ENTITY_NAME),
    ));

    // Entity with SMAA for testing mutations (separate from cameras to avoid conflicts)
    commands.spawn((Smaa::default(), Name::new(SMAA_TEST_ENTITY_NAME)));

    // Entity with BorderRadius for testing mutations
    commands.spawn((
        Node {
            border_radius: BorderRadius::all(Val::Px(BORDER_RADIUS)),
            ..default()
        },
        Name::new(BORDER_RADIUS_TEST_ENTITY_NAME),
    ));

    // Entity with CursorIcon for testing mutations
    commands.spawn((
        CursorIcon::System(SystemCursorIcon::Default),
        Name::new(CURSOR_ICON_TEST_ENTITY_NAME),
    ));

    // Entity with BorderGradient for testing mutations
    commands.spawn((
        BorderGradient(vec![Gradient::Linear(LinearGradient {
            color_space: InterpolationColorSpace::Srgba,
            angle:       std::f32::consts::FRAC_PI_4,
            stops:       vec![
                ColorStop::percent(BORDER_GRADIENT_START_COLOR, GRADIENT_START_PERCENT),
                ColorStop::percent(BORDER_GRADIENT_END_COLOR, UI_FILL_PERCENT),
            ],
        })]),
        Name::new(BORDER_GRADIENT_TEST_ENTITY_NAME),
    ));

    // Entity with BackgroundGradient for testing mutations
    commands.spawn((
        Node {
            width: Val::Px(UI_NODE_SIZE),
            height: Val::Px(UI_NODE_SIZE),
            grid_template_rows: vec![RepeatedGridTrack::minmax(
                GRID_ROW_TRACK_COUNT,
                MinTrackSizingFunction::Auto,
                MaxTrackSizingFunction::MaxContent,
            )],
            grid_template_columns: vec![RepeatedGridTrack::minmax(
                GRID_COLUMN_TRACK_COUNT,
                MinTrackSizingFunction::Px(GRID_COLUMN_MIN_PX),
                MaxTrackSizingFunction::Px(GRID_COLUMN_MAX_PX),
            )],
            ..default()
        },
        BackgroundGradient(vec![Gradient::Linear(LinearGradient {
            color_space: InterpolationColorSpace::Srgba,
            angle:       std::f32::consts::FRAC_PI_2,
            stops:       vec![
                ColorStop::percent(BACKGROUND_GRADIENT_START_COLOR, GRADIENT_START_PERCENT),
                ColorStop::percent(BACKGROUND_GRADIENT_END_COLOR, UI_FILL_PERCENT),
            ],
        })]),
        Name::new(BACKGROUND_GRADIENT_TEST_ENTITY_NAME),
    ));
}

fn spawn_light_entities(commands: &mut Commands, asset_server: &AssetServer) {
    // Entity with PointLight which will automatically get CubemapFrusta and shadow maps when
    // shadows enabled
    commands.spawn((
        PointLight {
            intensity: POINT_LIGHT_INTENSITY,
            color: Color::WHITE,
            shadow_maps_enabled: true, /* Enable shadows to trigger CubemapFrusta and
                                        * PointLightShadowMap */
            ..default()
        },
        Transform::from_translation(POINT_LIGHT_TRANSFORM),
        Name::new(POINT_LIGHT_TEST_ENTITY_NAME),
        ShadowFilteringMethod::default(),
        PointLightTexture {
            image:          asset_server.load(CAUSTIC_LIGHTMAP_PATH),
            cubemap_layout: bevy::camera::primitives::CubemapLayout::CrossVertical,
        },
        ShowLightGizmo {
            color: Some(bevy::light::gizmos::LightGizmoColor::Manual(
                POINT_LIGHT_GIZMO_COLOR,
            )),
        },
    ));

    // Entity with DirectionalLight for testing mutations
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: DIRECTIONAL_LIGHT_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        Name::new(DIRECTIONAL_LIGHT_TEST_ENTITY_NAME),
        CascadeShadowConfig::default(),
        Cascades::default(),
        VolumetricLight, // For testing mutations - enables light shafts/god rays
        DirectionalLightTexture {
            image: asset_server.load(CAUSTIC_LIGHTMAP_PATH),
            tiled: true,
        },
        ShowLightGizmo {
            color: Some(bevy::light::gizmos::LightGizmoColor::MatchLightColor),
        },
    ));

    // Entity with SpotLight for testing mutations
    commands.spawn((
        SpotLight {
            color: Color::WHITE,
            intensity: SPOT_LIGHT_INTENSITY,
            range: SPOT_LIGHT_RANGE,
            radius: SPOT_LIGHT_RADIUS,
            shadow_maps_enabled: true,
            inner_angle: SPOT_LIGHT_INNER_ANGLE,
            outer_angle: SPOT_LIGHT_OUTER_ANGLE,
            ..default()
        },
        Transform::from_translation(SPOT_LIGHT_TRANSFORM),
        Name::new(SPOT_LIGHT_TEST_ENTITY_NAME),
        SpotLightTexture {
            image: asset_server.load(CAUSTIC_LIGHTMAP_PATH),
        },
        ShowLightGizmo {
            color: Some(bevy::light::gizmos::LightGizmoColor::Varied),
        },
    ));

    // Entity with DistanceFog for testing mutations
    commands.spawn((
        bevy::pbr::DistanceFog {
            color:                      DISTANCE_FOG_COLOR,
            directional_light_color:    DISTANCE_FOG_LIGHT_COLOR,
            directional_light_exponent: DISTANCE_FOG_EXPONENT,
            falloff:                    bevy::pbr::FogFalloff::Linear {
                start: DISTANCE_FOG_START,
                end:   DISTANCE_FOG_END,
            },
        },
        Name::new(DISTANCE_FOG_TEST_ENTITY_NAME),
    ));
}

fn spawn_shadow_test_entities(commands: &mut Commands, asset_server: &AssetServer) {
    // Entity with NotShadowCaster for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()),                             // Dummy mesh handle
        MeshMaterial3d::<StandardMaterial>(Handle::default()), // Dummy material handle
        Transform::from_translation(AABB_TRANSFORM),
        NotShadowCaster, // For testing mutations
        Aabb::from_min_max(AABB_MIN, AABB_MAX),
        ShowAabbGizmo {
            color: Some(SHADOW_GIZMO_COLOR),
        },
        Name::new(NOT_SHADOW_CASTER_TEST_ENTITY_NAME),
    ));

    // Entity with Lightmap for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()),                             // Dummy mesh handle
        MeshMaterial3d::<StandardMaterial>(Handle::default()), // Dummy material handle
        Transform::from_xyz(0.0, 0.0, 0.0),
        Lightmap {
            image:            asset_server.load(CAUSTIC_LIGHTMAP_PATH),
            uv_rect:          LIGHTMAP_UV_RECT,
            bicubic_sampling: true,
        },
        Name::new(LIGHTMAP_TEST_ENTITY_NAME),
    ));

    // Entity with NotShadowReceiver for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()),                             // Dummy mesh handle
        MeshMaterial3d::<StandardMaterial>(Handle::default()), // Dummy material handle
        Transform::from_translation(NOT_SHADOW_RECEIVER_TRANSFORM),
        NotShadowReceiver, // For testing mutations
        Name::new(NOT_SHADOW_RECEIVER_TEST_ENTITY_NAME),
    ));

    // Entity with ExtendedMaterial<StandardMaterial, ForwardDecalMaterialExt> for testing mutations
    commands.spawn((
        Mesh3d(Handle::default()), // Dummy mesh handle
        MeshMaterial3d::<ExtendedMaterial<StandardMaterial, ForwardDecalMaterialExt>>(
            Handle::default(),
        ), // Dummy material handle
        Transform::from_translation(EXTENDED_DECAL_TRANSFORM),
        Name::new(EXTENDED_DECAL_MATERIAL_TEST_ENTITY_NAME),
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
            vertices: TEST_ARRAY_FIELD_VERTICES,
            values:   TEST_ARRAY_FIELD_VALUES,
        },
        Name::new(TEST_ARRAY_FIELD_ENTITY_NAME),
    ));

    commands.spawn((
        TestArrayTransforms {
            transforms: [
                Transform::from_translation(TEST_ENTITY_TRANSLATION),
                Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ],
        },
        Name::new(TEST_ARRAY_TRANSFORMS_ENTITY_NAME),
    ));

    commands.spawn((
        TestTupleField {
            coords:    TEST_TUPLE_COORDS,
            color_rgb: TEST_TUPLE_COLOR_RGB,
        },
        Name::new(TEST_TUPLE_FIELD_ENTITY_NAME),
    ));

    commands.spawn((
        TestTupleStruct(
            TEST_TUPLE_STRUCT_VALUE,
            TEST_TUPLE_STRUCT_LABEL.to_string(),
            ToggleState::Enabled,
        ),
        Name::new(TEST_TUPLE_STRUCT_ENTITY_NAME),
    ));

    commands.spawn((
        TestComplexTuple {
            complex: (
                Transform::from_translation(COMPLEX_TUPLE_TRANSFORM),
                COMPLEX_TUPLE_VEC3,
            ),
            nested:  (
                COMPLEX_TUPLE_NESTED_VEC2,
                (
                    COMPLEX_TUPLE_NESTED_SCALAR,
                    COMPLEX_TUPLE_NESTED_LABEL.to_string(),
                ),
            ),
        },
        Name::new(TEST_COMPLEX_TUPLE_ENTITY_NAME),
    ));
}

fn spawn_collection_test_entities(commands: &mut Commands) {
    let mut simple_set_component = SimpleSetComponent::default();
    simple_set_component
        .string_set
        .insert(SIMPLE_SET_HELLO.to_string());
    simple_set_component
        .string_set
        .insert(SIMPLE_SET_WORLD.to_string());
    simple_set_component
        .string_set
        .insert(SIMPLE_SET_TEST.to_string());
    commands.spawn((simple_set_component, Name::new(SIMPLE_SET_ENTITY_NAME)));

    let mut test_map_component = TestMapComponent::default();
    test_map_component
        .strings
        .insert(MAP_KEY_ONE.to_string(), MAP_VALUE_ONE.to_string());
    test_map_component
        .strings
        .insert(MAP_KEY_TWO.to_string(), MAP_VALUE_TWO.to_string());
    test_map_component
        .strings
        .insert(MAP_KEY_THREE.to_string(), MAP_VALUE_THREE.to_string());

    test_map_component
        .values
        .insert(MAP_TEMPERATURE_KEY.to_string(), MAP_TEMPERATURE_VALUE);
    test_map_component
        .values
        .insert(MAP_HUMIDITY_KEY.to_string(), MAP_HUMIDITY_VALUE);
    test_map_component
        .values
        .insert(MAP_PRESSURE_KEY.to_string(), MAP_PRESSURE_VALUE);

    test_map_component.transforms.insert(
        MAP_PLAYER_KEY.to_string(),
        Transform::from_translation(MAP_PLAYER_TRANSFORM),
    );
    test_map_component.transforms.insert(
        MAP_ENEMY_KEY.to_string(),
        Transform::from_translation(MAP_ENEMY_TRANSFORM),
    );
    test_map_component.transforms.insert(
        MAP_POWERUP_KEY.to_string(),
        Transform::from_translation(MAP_POWERUP_TRANSFORM).with_scale(MAP_POWERUP_SCALE),
    );

    commands.spawn((test_map_component, Name::new(TEST_MAP_ENTITY_NAME)));

    let mut enum_keyed_map = TestEnumKeyedMap::default();
    enum_keyed_map
        .enum_keyed
        .insert(SimpleTestEnum::Variant1, ENUM_KEYED_FIRST_VALUE.to_string());
    enum_keyed_map.enum_keyed.insert(
        SimpleTestEnum::Variant2,
        ENUM_KEYED_SECOND_VALUE.to_string(),
    );
    enum_keyed_map
        .enum_keyed
        .insert(SimpleTestEnum::Variant3, ENUM_KEYED_THIRD_VALUE.to_string());

    commands.spawn((enum_keyed_map, Name::new(TEST_ENUM_KEYED_MAP_ENTITY_NAME)));

    commands.spawn((
        TestCollectionComponent::default(),
        Name::new(TEST_COLLECTION_ENTITY_NAME),
    ));
}

fn spawn_enum_test_entities(commands: &mut Commands) {
    commands.spawn((
        NestedConfigEnum::Always,
        Name::new(NESTED_CONFIG_ENUM_ALWAYS_ENTITY_NAME),
    ));

    commands.spawn((
        NestedConfigEnum::Never,
        Name::new(NESTED_CONFIG_ENUM_NEVER_ENTITY_NAME),
    ));

    commands.spawn((
        NestedConfigEnum::Conditional(NESTED_CONFIG_CONDITIONAL_VALUE),
        Name::new(NESTED_CONFIG_ENUM_CONDITIONAL_ENTITY_NAME),
    ));

    commands.spawn((
        TestComplexComponent {
            transform:          Transform::from_translation(COMPLEX_COMPONENT_TRANSFORM),
            simple_nested_enum: SimpleNestedEnum::WithVec2(SIMPLE_NESTED_VEC2),
            points:             COMPLEX_COMPONENT_POINTS,
            range:              COMPLEX_COMPONENT_RANGE,
            optional_value:     Some(COMPLEX_COMPONENT_OPTIONAL_VALUE),
        },
        Name::new(TEST_COMPLEX_ENTITY_NAME),
    ));

    commands.spawn((
        TestVariantChainEnum::WithMiddleStruct {
            middle_struct: MiddleStruct {
                label:       VARIANT_CHAIN_LABEL.to_string(),
                magnitude:   VARIANT_CHAIN_MAGNITUDE,
                nested_enum: BottomEnum::VariantA(BOTTOM_ENUM_VARIANT_A_VALUE),
            },
        },
        Name::new(TEST_VARIANT_CHAIN_ENTITY_NAME),
    ));

    commands.spawn((
        SimpleNestedEnum::WithVec2(SIMPLE_NESTED_VEC2),
        Name::new(SIMPLE_NESTED_ENUM_ENTITY_NAME),
    ));

    commands.spawn((
        TestEnumWithArray::Vec2(TEST_ENUM_ARRAY_POINTS),
        Name::new(TEST_ENUM_WITH_ARRAY_ENTITY_NAME),
    ));

    commands.spawn((
        SimpleNestedEnum::WithVec2(SIMPLE_NESTED_VEC2),
        Name::new(SIMPLE_NESTED_ENUM_VEC2_ENTITY_NAME),
    ));

    commands.spawn((
        SimpleNestedEnum::WithTransform(Transform::from_translation(COMPLEX_COMPONENT_TRANSFORM)),
        Name::new(SIMPLE_NESTED_ENUM_TRANSFORM_ENTITY_NAME),
    ));

    commands.spawn((
        SimpleNestedEnum::WithStruct {
            position: SIMPLE_NESTED_STRUCT_POSITION,
            scale:    SIMPLE_NESTED_STRUCT_SCALE,
        },
        Name::new(SIMPLE_NESTED_ENUM_STRUCT_ENTITY_NAME),
    ));

    commands.spawn((
        OptionTestEnum::MaybeVec2(Some(OPTION_ENUM_VEC2)),
        Name::new(OPTION_TEST_ENUM_VEC2_ENTITY_NAME),
    ));

    commands.spawn((
        OptionTestEnum::MaybeTransform(Some(Transform::from_scale(OPTION_ENUM_TRANSFORM_SCALE))),
        Name::new(OPTION_TEST_ENUM_TRANSFORM_ENTITY_NAME),
    ));

    commands.spawn((
        WrapperEnum::Simple(SimpleNestedEnum::WithVec2(WRAPPER_SIMPLE_VEC2)),
        Name::new(WRAPPER_ENUM_SIMPLE_ENTITY_NAME),
    ));

    commands.spawn((
        WrapperEnum::Optional(Some(SimpleNestedEnum::WithTransform(
            Transform::from_rotation(Quat::from_rotation_y(WRAPPER_OPTIONAL_ROTATION_RADIANS)),
        ))),
        Name::new(WRAPPER_ENUM_OPTIONAL_ENTITY_NAME),
    ));
}

fn spawn_gltf_test_entities(commands: &mut Commands) {
    commands.spawn((
        TestStructNoSerDe {
            value:        TEST_STRUCT_NO_SER_DE_VALUE,
            name:         TEST_STRUCT_NO_SER_DE_NAME.to_string(),
            toggle_state: ToggleState::Enabled,
        },
        Name::new(TEST_STRUCT_NO_SER_DE_ENTITY_NAME),
    ));

    commands.spawn((Gamepad::default(), Name::new(GAMEPAD_TEST_ENTITY_NAME)));

    commands.spawn((
        GamepadSettings::default(),
        Name::new(GAMEPAD_SETTINGS_TEST_ENTITY_NAME),
    ));

    commands.spawn((
        bevy::gltf::GltfExtras {
            value: GLTF_EXTRAS_VALUE.to_string(),
        },
        Name::new(GLTF_EXTRAS_TEST_ENTITY_NAME),
    ));

    commands.spawn((
        bevy::gltf::GltfMaterialExtras {
            value: GLTF_MATERIAL_EXTRAS_VALUE.to_string(),
        },
        Name::new(GLTF_MATERIAL_EXTRAS_TEST_ENTITY_NAME),
    ));

    commands.spawn((
        bevy::gltf::GltfMaterialName(GLTF_MATERIAL_NAME_VALUE.to_string()),
        Name::new(GLTF_MATERIAL_NAME_TEST_ENTITY_NAME),
    ));

    commands.spawn((
        bevy::gltf::GltfMeshExtras {
            value: GLTF_MESH_EXTRAS_VALUE.to_string(),
        },
        Name::new(GLTF_MESH_EXTRAS_TEST_ENTITY_NAME),
    ));

    commands.spawn((
        bevy::gltf::GltfSceneExtras {
            value: GLTF_SCENE_EXTRAS_VALUE.to_string(),
        },
        Name::new(GLTF_SCENE_EXTRAS_TEST_ENTITY_NAME),
    ));

    commands.spawn((Gamepad::default(), Name::new(TEST_GAMEPAD_NAME)));
}

fn spawn_mixed_mutability_test_entities(commands: &mut Commands) {
    let create_mixed_core = |suffix: &str| TestMixedMutabilityCore {
        mutable_string:           format!("test_string_{suffix}"),
        mutable_float:            MIXED_MUTABILITY_FLOAT,
        not_mutable_arc:          Arc::new(format!("arc_string_{suffix}")),
        partially_mutable_nested: TestPartiallyMutableNested {
            mutable_value:   MIXED_MUTABILITY_NESTED_VALUE,
            not_mutable_arc: Arc::new(MIXED_MUTABILITY_ARC_ITEMS.to_vec()),
        },
    };

    commands.spawn((
        TestMixedMutabilityVec {
            items: vec![
                create_mixed_core(MIXED_MUTABILITY_VEC_ZERO_SUFFIX),
                create_mixed_core(MIXED_MUTABILITY_VEC_ONE_SUFFIX),
                create_mixed_core(MIXED_MUTABILITY_VEC_TWO_SUFFIX),
            ],
        },
        Name::new(TEST_MIXED_MUTABILITY_VEC_ENTITY_NAME),
    ));

    commands.spawn((
        TestMixedMutabilityArray {
            items: [
                create_mixed_core(MIXED_MUTABILITY_ARRAY_ZERO_SUFFIX),
                create_mixed_core(MIXED_MUTABILITY_ARRAY_ONE_SUFFIX),
            ],
        },
        Name::new(TEST_MIXED_MUTABILITY_ARRAY_ENTITY_NAME),
    ));

    commands.spawn((
        TestMixedMutabilityTuple(
            create_mixed_core(MIXED_MUTABILITY_TUPLE_SUFFIX),
            MIXED_MUTABILITY_TUPLE_VALUE,
            MIXED_MUTABILITY_TUPLE_LABEL.to_string(),
        ),
        Name::new(TEST_MIXED_MUTABILITY_TUPLE_ENTITY_NAME),
    ));

    commands.spawn((
        TestMixedMutabilityEnum::Multiple {
            name:  MIXED_MUTABILITY_ENUM_NAME.to_string(),
            mixed: create_mixed_core(MIXED_MUTABILITY_ENUM_SUFFIX),
            value: MIXED_MUTABILITY_ENUM_VALUE,
        },
        Name::new(TEST_MIXED_MUTABILITY_ENUM_ENTITY_NAME),
    ));
}

fn spawn_retained_gizmo_entities(
    commands: &mut Commands,
    gizmo_assets: &mut ResMut<Assets<GizmoAsset>>,
) {
    // Create a gizmo asset with a simple sphere
    let mut gizmo_asset = GizmoAsset::default();
    gizmo_asset.sphere(Vec3::ZERO, GIZMO_SPHERE_RADIUS, GIZMO_SPHERE_COLOR);

    let gizmo_handle = gizmo_assets.add(gizmo_asset);

    // Spawn entity with Gizmo component
    commands.spawn((
        Gizmo {
            handle:      gizmo_handle,
            line_config: GizmoLineConfig {
                width: GIZMO_LINE_WIDTH,
                perspective: true,
                ..default()
            },
            depth_bias:  0.0,
        },
        Name::new(RETAINED_GIZMO_TEST_ENTITY_NAME),
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
        Name::new(ANIMATION_GRAPH_HANDLE_AND_PLAYER_AND_TRANSITIONS_TEST_ENTITY_NAME),
    ));

    // Entity with AnimationTargetId and AnimatedBy for testing mutations
    commands.spawn((
        AnimationTargetId::from_name(&Name::new(ANIMATION_TARGET_NAME)),
        AnimatedBy(Entity::PLACEHOLDER),
        Name::new(ANIMATION_TARGET_TEST_ENTITY_NAME),
    ));

    // Entity with DenoiseCas for testing mutations
    // Note: DenoiseCas doesn't have a public constructor, but we can register it
    // It's automatically added when ContrastAdaptiveSharpening has denoise enabled

    // Entity with TemporalAntiAliasing for testing mutations
    commands.spawn((
        TemporalAntiAliasing::default(),
        Name::new(TEMPORAL_ANTI_ALIASING_TEST_ENTITY_NAME),
    ));

    // Entity with SpatialListener for testing mutations
    commands.spawn((
        SpatialListener::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new(SPATIAL_LISTENER_TEST_ENTITY_NAME),
    ));

    // Entity with TabGroup for testing mutations
    commands.spawn((TabGroup::new(0), Name::new(TAB_GROUP_TEST_ENTITY_NAME)));

    // Entity with TabIndex for testing mutations
    commands.spawn((TabIndex(0), Name::new(TAB_INDEX_TEST_ENTITY_NAME)));

    // Entity with FogVolume for testing mutations
    commands.spawn((
        FogVolume::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new(FOG_VOLUME_TEST_ENTITY_NAME),
    ));

    // Entity with MainPassResolutionOverride for testing mutations
    // Try accessing it directly from bevy::camera even though the module is private
    commands.spawn((
        bevy::camera::MainPassResolutionOverride(MAIN_PASS_RESOLUTION_OVERRIDE_SIZE),
        Name::new(MAIN_PASS_RESOLUTION_OVERRIDE_TEST_ENTITY_NAME),
    ));

    // Entity with GltfMeshName for testing mutations
    commands.spawn((
        bevy::gltf::GltfMeshName(GLTF_MESH_NAME_VALUE.to_string()),
        Name::new(GLTF_MESH_NAME_TEST_ENTITY_NAME),
    ));

    // Entity with PlaybackSettings - this is actually a component that can be spawned!
    commands.spawn((
        PlaybackSettings::default(),
        Name::new(PLAYBACK_SETTINGS_TEST_ENTITY_NAME),
    ));

    // Note: DenoiseCas is automatically added when ContrastAdaptiveSharpening has denoise enabled
    // AnimationGraphHandle, DirectionalLightTexture, PointLightTexture, SpotLightTexture,
    // GeneratedEnvironmentMapLight are internal/generated components
    // AudioSink, SpatialAudioSink, AudioSourceHandle, SpatialAudioSourceHandle, GlobalVolume,
    // Volume are not components
}

fn spawn_render_entities(commands: &mut Commands) {
    // Entity with MeshMorphWeights for testing mutations
    commands.spawn((
        MeshMorphWeights::Value {
            weights: MESH_MORPH_WEIGHTS.to_vec(),
        },
        Name::new(MESH_MORPH_WEIGHTS_TEST_ENTITY_NAME),
    ));

    // Entity with MorphWeights for testing mutations
    commands.spawn((
        MorphWeights::default(),
        Name::new(MORPH_WEIGHTS_TEST_ENTITY_NAME),
    ));

    // Entity with SkinnedMesh for testing mutations
    commands.spawn((
        SkinnedMesh::default(),
        Name::new(SKINNED_MESH_TEST_ENTITY_NAME),
    ));

    // Entity with MeshMaterial2d<ColorMaterial> and Mesh2d for testing mutations
    commands.spawn((
        Mesh2d(Handle::default()),
        bevy::prelude::MeshMaterial2d::<bevy::prelude::ColorMaterial>(Handle::default()),
        Name::new(MESH_MATERIAL2D_TEST_ENTITY_NAME),
    ));

    // Entity with CascadesFrusta for testing mutations
    commands.spawn((
        CascadesFrusta::default(),
        Name::new(CASCADES_FRUSTA_TEST_ENTITY_NAME),
    ));

    // Entity with Text2d for testing mutations
    commands.spawn((
        Text2d(TEXT2D_CONTENT.to_string()),
        Text2dShadow::default(), // For testing mutations
        Transform::from_translation(TEXT2D_TRANSFORM),
        Name::new(TEXT2D_TEST_ENTITY_NAME),
    ));

    // Entity with ClusteredDecal for testing mutations
    commands.spawn((
        ClusteredDecal::default(),
        Name::new(CLUSTERED_DECAL_TEST_ENTITY_NAME),
    ));

    // Entity with LightProbe for testing mutations
    commands.spawn((
        LightProbe::default(),
        Name::new(LIGHT_PROBE_TEST_ENTITY_NAME),
    ));

    // Entity with ClusterConfig for testing mutations
    commands.spawn((
        ClusterConfig::default(),
        Name::new(CLUSTER_CONFIG_TEST_ENTITY_NAME),
    ));

    // Entity with EnvironmentMapLight for testing mutations
    commands.spawn((
        EnvironmentMapLight::default(),
        Name::new(ENVIRONMENT_MAP_LIGHT_TEST_ENTITY_NAME),
    ));

    // Entity with GeneratedEnvironmentMapLight for testing mutations
    // Uses the same skybox image handle we created earlier
    commands.spawn((
        GeneratedEnvironmentMapLight {
            environment_map:                  Handle::default(), // Dummy handle for testing
            intensity:                        GENERATED_ENVIRONMENT_INTENSITY,
            rotation:                         Quat::IDENTITY,
            affects_lightmapped_mesh_diffuse: true,
        },
        Name::new(GENERATED_ENVIRONMENT_MAP_LIGHT_TEST_ENTITY_NAME),
    ));

    // Entity with IrradianceVolume for testing mutations
    commands.spawn((
        IrradianceVolume::default(),
        Name::new(IRRADIANCE_VOLUME_TEST_ENTITY_NAME),
    ));

    // Entity with AmbientLight (requires Camera) for testing mutations
    // Also has Msaa since this camera is disabled and won't cause rendering conflicts
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: AMBIENT_CAMERA_ORDER, // Unique order for this test camera
            is_active: false,            // Disable this test camera to avoid rendering
            ..default()
        },
        AmbientLight::default(),
        Msaa::default(), // Safe to test here since camera is disabled
        Transform::from_translation(AMBIENT_LIGHT_TRANSFORM),
        Name::new(AMBIENT_LIGHT_TEST_ENTITY_NAME),
    ));

    // Entity with Screenshot for testing mutations
    commands.spawn((
        Screenshot::primary_window(),
        Name::new(SCREENSHOT_TEST_ENTITY_NAME),
    ));

    // Entity with OcclusionCulling for testing mutations
    commands.spawn((
        OcclusionCulling,
        Name::new(OCCLUSION_CULLING_TEST_ENTITY_NAME),
    ));

    // Entity with NoFrustumCulling for testing mutations
    commands.spawn((
        NoFrustumCulling,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new(NO_FRUSTUM_CULLING_TEST_ENTITY_NAME),
    ));

    // Entity with ManualTextureViewHandle for testing mutations
    commands.spawn((
        ManualTextureViewHandle(MANUAL_TEXTURE_VIEW_HANDLE_ID),
        Name::new(MANUAL_TEXTURE_VIEW_HANDLE_TEST_ENTITY_NAME),
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
            order: UI_CAMERA_ORDER, // Main camera
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
                order: TEST_CAMERA_ORDER, // Different order to avoid ambiguity
                is_active: false,         /* Disable to avoid rendering conflicts with deferred
                                           * pipeline */
                ..default()
            },
            Transform::from_translation(TEST_CAMERA_TRANSFORM).looking_at(Vec3::ZERO, Vec3::Y),
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
        .insert(MotionBlur::default()); // For testing mutations (added separately due to bundle
    // size limit)
}

fn spawn_ui_elements(commands: &mut Commands, port: &Res<CurrentPort>) {
    // Background
    commands
        .spawn((
            Node {
                width: Val::Percent(UI_FILL_PERCENT),
                height: Val::Percent(UI_FILL_PERCENT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                grid_template_rows: vec![RepeatedGridTrack::minmax(
                    2,
                    MinTrackSizingFunction::Auto,
                    MaxTrackSizingFunction::MaxContent,
                )],
                grid_template_columns: vec![RepeatedGridTrack::minmax(
                    1,
                    MinTrackSizingFunction::Px(GRID_COLUMN_MIN_PX),
                    MaxTrackSizingFunction::Px(GRID_COLUMN_MAX_PX),
                )],
                ..default()
            },
            BackgroundColor(BACKGROUND_COLOR), // Back to dark background
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
                padding: UiRect::all(Val::Px(TEXT_CONTAINER_PADDING)),
                grid_template_rows: vec![RepeatedGridTrack::minmax(
                    2,
                    MinTrackSizingFunction::Auto,
                    MaxTrackSizingFunction::MaxContent,
                )],
                grid_template_columns: vec![RepeatedGridTrack::minmax(
                    1,
                    MinTrackSizingFunction::Px(GRID_COLUMN_MIN_PX),
                    MaxTrackSizingFunction::Px(GRID_COLUMN_MAX_PX),
                )],
                ..default()
            },
            BackgroundColor(TEXT_CONTAINER_BACKGROUND), /* Blue background for the entire text
                                                         * area */
            BoxShadowSamples(BOX_SHADOW_SAMPLES),
            CalculatedClip {
                clip: bevy::math::Rect::from_corners(Vec2::ZERO, CALCULATED_CLIP_MAX),
            },
            Name::new(CALCULATED_CLIP_TEST_ENTITY_NAME),
        ))
        .with_children(|parent| {
            spawn_keyboard_display_text(parent, port);
            spawn_button_test(parent);
            spawn_label_test(parent);
            spawn_text_input_section(parent);
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
            font_size: FontSize::Px(KEYBOARD_DISPLAY_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        bevy::text::TextBackgroundColor(KEYBOARD_TEXT_BACKGROUND),
        KeyboardDisplayText,
        bevy::text::TextBounds {
            width: Some(TEXT_BOUNDS_WIDTH),
            height: Some(TEXT_BOUNDS_HEIGHT),
        },
        bevy::text::TextSpan(TEXT_SPAN_CONTENT.to_string()),
        bevy::prelude::TextShadow {
            offset: TEXT_SHADOW_OFFSET,
            color: BOX_SHADOW_COLOR,
        },
        bevy::prelude::UiAntiAlias::On,
        bevy::prelude::UiTargetCamera(Entity::PLACEHOLDER),
        bevy::prelude::ImageNode {
            image: Handle::default(),
            color: UI_IMAGE_COLOR,  // Blue background instead of white
            flip_x: false,
            flip_y: false,
            image_mode: bevy::prelude::NodeImageMode::Auto,
            rect: None,
            texture_atlas: None,
            ..default()
        },
        Name::new(TEXT_BOUNDS_TEST_ENTITY_NAME),
    ));
}

fn spawn_button_test(parent: &mut RelatedSpawnerCommands<ChildOf>) {
    // Button component for testing mutations
    parent.spawn((
        Node {
            width: Val::Px(BUTTON_WIDTH),
            height: Val::Px(BUTTON_HEIGHT),
            margin: UiRect::all(Val::Px(UI_MARGIN)),
            grid_template_rows: vec![RepeatedGridTrack::minmax(
                GRID_ROW_TRACK_COUNT,
                MinTrackSizingFunction::Auto,
                MaxTrackSizingFunction::MaxContent,
            )],
            grid_template_columns: vec![RepeatedGridTrack::minmax(
                GRID_COLUMN_TRACK_COUNT,
                MinTrackSizingFunction::Px(GRID_COLUMN_MIN_PX),
                MaxTrackSizingFunction::Px(GRID_COLUMN_MAX_PX),
            )],
            ..default()
        },
        BackgroundColor(BUTTON_BACKGROUND),
        Button,
        Outline::new(
            Val::Px(BUTTON_OUTLINE_WIDTH),
            Val::Px(0.0),
            BUTTON_OUTLINE_COLOR,
        ), /* Yellow outline
            * for testing */
        FocusPolicy::Block,                          // For testing mutations
        Interaction::None,                           // For testing mutations
        ZIndex(0),                                   // For testing mutations
        bevy::ui::RelativeCursorPosition::default(), // For testing mutations
        Name::new(BUTTON_TEST_ENTITY_NAME),
    ));
}

fn spawn_label_test(parent: &mut RelatedSpawnerCommands<ChildOf>) {
    // Label component for testing mutations
    parent.spawn((
        Text::new(LABEL_TEXT),
        TextFont {
            font_size: FontSize::Px(LABEL_FONT_SIZE),
            ..default()
        },
        TextColor(LABEL_COLOR), // Yellow color
        Label,
        UiTargetCamera(Entity::PLACEHOLDER), // For testing mutations
        Name::new(LABEL_TEST_ENTITY_NAME),
    ));

    // BoxShadow component for testing mutations
    parent.spawn((
        Node {
            width: Val::Px(BOX_SHADOW_WIDTH),
            height: Val::Px(BOX_SHADOW_HEIGHT),
            margin: UiRect::all(Val::Px(UI_MARGIN)),
            ..default()
        },
        BackgroundColor(BOX_SHADOW_BACKGROUND), // Light blue background
        BoxShadow::new(
            BOX_SHADOW_COLOR,                  // Black shadow with 50% opacity
            Val::Px(BOX_SHADOW_OFFSET_X),      // x_offset
            Val::Px(BOX_SHADOW_OFFSET_Y),      // y_offset
            Val::Px(BOX_SHADOW_SPREAD_RADIUS), // spread_radius
            Val::Px(BOX_SHADOW_BLUR_RADIUS),   // blur_radius
        ),
        Name::new(BOX_SHADOW_TEST_ENTITY_NAME),
    ));
}

fn spawn_text_input_section(parent: &mut RelatedSpawnerCommands<ChildOf>) {
    parent.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(TEXT_INPUT_FONT_SIZE),
            ..default()
        },
        TextColor(TEXT_INPUT_COLOR),
        bevy::text::TextBackgroundColor(TEXT_INPUT_BACKGROUND),
        bevy::text::TextBounds {
            width:  Some(TEXT_INPUT_BOUNDS_WIDTH),
            height: Some(TEXT_INPUT_BOUNDS_HEIGHT),
        },
        Outline::new(
            Val::Px(TEXT_INPUT_OUTLINE_WIDTH),
            Val::Px(0.0),
            TEXT_INPUT_OUTLINE_COLOR,
        ),
        TextInputDisplay,
        Name::new(TEXT_INPUT_TEST_ENTITY_NAME),
    ));
}

/// Handle keyboard input for the text input field
fn handle_text_input(
    mut events: MessageReader<KeyboardInput>,
    mut content: ResMut<TextInputContent>,
    mut display: Query<&mut Text, With<TextInputDisplay>>,
) {
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match (&event.logical_key, &event.text) {
            (Key::Backspace, _) => {
                content.text.pop();
            },
            (_, Some(inserted_text)) if inserted_text.chars().all(is_printable_char) => {
                content.text.push_str(inserted_text);
            },
            _ => {},
        }
    }

    for mut text in &mut display {
        (**text).clone_from(&content.text);
    }
}

/// Filter out non-printable characters (from egui-winit)
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
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
            },
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
                    let combination = history.complete_combination.clone();
                    history.last_keys = combination;

                    // Extract modifiers from the complete combination
                    history.complete_modifiers =
                        collect_modifier_labels(&history.complete_combination);

                    history.completion_state = CompletionState::Completed;
                }
            },
        }

        // `KeyHistory::last_keys` is assigned after all keys are released.
    }

    // Update modifiers based on currently active keys
    history.modifiers = collect_modifier_labels(&history.active_keys);
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

        let status = if history.completion_state.is_completed() {
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
