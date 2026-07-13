//! Deterministic render-target fixtures for terminal screenshot integration tests.
//!
//! The retained target is 256x192 physical pixels. Both fixture cameras use a
//! 224x168 viewport whose target-space origin is (16, 12). Zero-padding capture
//! rectangles are expected to be:
//!
//! - `Screenshot2dUiReference`: (16, 12, 224, 168)
//! - `NatesList`: (40, 32, 64, 48)
//! - `ScreenshotRotatedClippedUi`: (132, 40, 32, 56)
//! - `Screenshot2dAabb`: (106, 98, 12, 60)
//! - `Screenshot3dReference`: (16, 12, 224, 168)
//! - `Screenshot3dAabb`: (162, 90, 12, 48)
//!
//! During the 2D/UI epoch, `NatesList` contains a yellow marker at (52, 44)
//! and a magenta marker at (100, 56), while `Screenshot2dAabb` contains a
//! yellow marker at (112, 128). During the 3D epoch, `Screenshot3dAabb`
//! contains a yellow marker at (168, 114). Coordinates are target-space pixels.

use bevy::asset::RenderAssetUsages;
use bevy::camera::ClearColorConfig;
use bevy::camera::RenderTarget;
use bevy::camera::ScalingMode;
use bevy::camera::Viewport;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoCpuCulling;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::math::Rot2;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::render::render_resource::TextureUsages;
use bevy::ui::ComputedNode;
use bevy::window::PrimaryWindow;

// camera epochs
const PRIMARY_WINDOW_CAMERA_NAME: &str = "ScreenshotPrimaryWindowCamera";
const PRIMARY_WINDOW_TARGET_NAME: &str = "ScreenshotPrimaryWindowTarget";
const THREE_D_CAMERA_NAME: &str = "Screenshot3dCamera";
const THREE_D_CAMERA_ORDER: isize = 1;
const TWO_D_UI_CAMERA_NAME: &str = "Screenshot2dUiCamera";
const TWO_D_UI_CAMERA_ORDER: isize = 0;

// colors
const EDGE_MARKER_COLOR: Color = Color::srgb_u8(255, 0, 255);
const FIXED_UI_COLOR: Color = Color::srgb_u8(0, 0, 255);
const INTERIOR_MARKER_COLOR: Color = Color::srgb_u8(255, 255, 0);
const ROTATED_UI_COLOR: Color = Color::srgb_u8(0, 255, 255);
const TARGET_CLEAR_COLOR: Color = Color::srgb_u8(0, 0, 0);
const THREE_D_COLOR: Color = Color::srgb_u8(0, 255, 0);
const TWO_D_COLOR: Color = Color::srgb_u8(255, 0, 0);

// entity names
const DISJOINT_LAYER_NAME: &str = "ScreenshotDisjointLayer";
const DUPLICATE_NAME: &str = "ScreenshotDuplicateName";
const HIDDEN_AABB_NAME: &str = "ScreenshotHiddenAabb";
const HIDDEN_UI_NAME: &str = "ScreenshotHiddenUi";
const NATES_LIST_NAME: &str = "NatesList";
const PARTIAL_UI_NAME: &str = "ScreenshotPartialUi";
const ROTATED_CLIPPED_UI_NAME: &str = "ScreenshotRotatedClippedUi";
const THREE_D_AABB_NAME: &str = "Screenshot3dAabb";
const THREE_D_MARKER_NAME: &str = "Screenshot3dMarker";
const THREE_D_REFERENCE_NAME: &str = "Screenshot3dReference";
const TWO_D_AABB_NAME: &str = "Screenshot2dAabb";
const TWO_D_MARKER_NAME: &str = "Screenshot2dMarker";
const TWO_D_UI_REFERENCE_NAME: &str = "Screenshot2dUiReference";
const UNIQUE_NAME: &str = "ScreenshotUniqueName";
const UNSUPPORTED_NAME: &str = "ScreenshotUnsupported";

// render layers
const DISJOINT_RENDER_LAYER: usize = 12;
const THREE_D_RENDER_LAYER: usize = 11;
const TWO_D_RENDER_LAYER: usize = 10;

// target geometry
const EDGE_MARKER_POSITION: Vec2 = Vec2::new(56.0, 0.0);
const EDGE_MARKER_SIZE: Vec2 = Vec2::new(8.0, 48.0);
const FIXED_UI_POSITION: Vec2 = Vec2::new(24.0, 20.0);
const FIXED_UI_SIZE: Vec2 = Vec2::new(64.0, 48.0);
const HIDDEN_UI_POSITION: Vec2 = Vec2::new(184.0, 16.0);
const HIDDEN_UI_SIZE: Vec2 = Vec2::splat(24.0);
const INTERIOR_MARKER_POSITION: Vec2 = Vec2::splat(8.0);
const INTERIOR_MARKER_SIZE: Vec2 = Vec2::splat(8.0);
const ROTATED_UI_CHILD_POSITION: Vec2 = Vec2::new(-12.0, 8.0);
const ROTATED_UI_CHILD_SIZE: Vec2 = Vec2::new(64.0, 32.0);
const ROTATED_UI_PARENT_POSITION: Vec2 = Vec2::new(112.0, 28.0);
const ROTATED_UI_PARENT_SIZE: Vec2 = Vec2::new(64.0, 56.0);
const TARGET_FILL: [u8; 4] = [0, 0, 0, 255];
const TARGET_PERCENT: f32 = 100.0;
const TARGET_SIZE: UVec2 = UVec2::new(256, 192);
const VIEWPORT_POSITION: UVec2 = UVec2::new(16, 12);
const VIEWPORT_SIZE: UVec2 = UVec2::new(224, 168);

// world geometry
const AABB_DEPTH: f32 = 1.0;
const DISJOINT_AABB_POSITION: Vec3 = Vec3::new(72.0, -48.0, 0.0);
const ERROR_AABB_HALF_EXTENTS: Vec3 = Vec3::new(8.0, 8.0, 0.5);
const HIDDEN_AABB_POSITION: Vec3 = Vec3::new(72.0, 48.0, 0.0);
const MARKER_2D_SIZE: Vec2 = Vec2::splat(4.0);
const MARKER_3D_SIZE: Vec3 = Vec3::new(4.0, 4.0, 2.0);
const REFERENCE_DEPTH: f32 = 2.0;
const THREE_D_CAMERA_POSITION: Vec3 = Vec3::new(0.0, 0.0, 100.0);
const THREE_D_ENTITY_POSITION: Vec3 = Vec3::new(40.0, -18.0, 0.0);
const THREE_D_ENTITY_SCALE: Vec3 = Vec3::new(1.5, 0.5, 1.0);
const THREE_D_ENTITY_SIZE: Vec3 = Vec3::new(32.0, 24.0, 10.0);
const THREE_D_MARKER_POSITION: Vec3 = Vec3::new(40.0, -18.0, 8.0);
const TWO_D_ENTITY_POSITION: Vec3 = Vec3::new(-16.0, -32.0, 0.0);
const TWO_D_ENTITY_SCALE: Vec3 = Vec3::new(1.5, 0.5, 1.0);
const TWO_D_ENTITY_SIZE: Vec2 = Vec2::new(40.0, 24.0);
const TWO_D_MARKER_POSITION: Vec3 = Vec3::new(-16.0, -32.0, 1.0);
const UNNAMED_AABB_HALF_EXTENTS: Vec3 = Vec3::splat(4.0);
const UNNAMED_AABB_POSITION: Vec3 = Vec3::new(-88.0, -60.0, 0.0);

pub(super) struct ScreenshotFixturesPlugin;

impl Plugin for ScreenshotFixturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_screenshot_fixtures)
            .add_systems(
                PostStartup,
                (establish_initial_camera_epoch, report_fixture_state).chain(),
            );
    }
}

#[derive(Resource)]
struct ScreenshotFixtureState {
    image:                 Handle<Image>,
    primary_window_camera: Option<Entity>,
    three_d_camera:        Entity,
    two_d_camera:          Entity,
}

fn setup_screenshot_fixtures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
) {
    commands
        .entity(*primary_window)
        .insert(Name::new(PRIMARY_WINDOW_TARGET_NAME));

    let image = images.add(render_target_image());
    let render_target = RenderTarget::Image(image.clone().into());
    let two_d_camera = spawn_two_d_camera(&mut commands, render_target.clone());
    let three_d_camera = spawn_three_d_camera(&mut commands, render_target);

    spawn_ui_fixtures(&mut commands, two_d_camera);
    spawn_two_d_fixtures(&mut commands);
    spawn_three_d_fixtures(&mut commands, &mut materials, &mut meshes);
    spawn_error_and_name_fixtures(&mut commands);

    commands.insert_resource(ScreenshotFixtureState {
        image,
        primary_window_camera: None,
        three_d_camera,
        two_d_camera,
    });
}

fn spawn_two_d_camera(commands: &mut Commands, render_target: RenderTarget) -> Entity {
    commands
        .spawn((
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(TARGET_CLEAR_COLOR),
                order: TWO_D_UI_CAMERA_ORDER,
                viewport: Some(fixture_viewport()),
                ..default()
            },
            render_target,
            Msaa::Off,
            RenderLayers::layer(TWO_D_RENDER_LAYER),
            Name::new(TWO_D_UI_CAMERA_NAME),
        ))
        .id()
}

fn spawn_three_d_camera(commands: &mut Commands, render_target: RenderTarget) -> Entity {
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(TARGET_CLEAR_COLOR),
                is_active: false,
                order: THREE_D_CAMERA_ORDER,
                viewport: Some(fixture_viewport()),
                ..default()
            },
            render_target,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width:  VIEWPORT_SIZE.as_vec2().x,
                    height: VIEWPORT_SIZE.as_vec2().y,
                },
                ..OrthographicProjection::default_3d()
            }),
            Transform::from_translation(THREE_D_CAMERA_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
            DebandDither::Disabled,
            Msaa::Off,
            RenderLayers::layer(THREE_D_RENDER_LAYER),
            Tonemapping::None,
            Name::new(THREE_D_CAMERA_NAME),
        ))
        .id()
}

fn spawn_ui_fixtures(commands: &mut Commands, camera: Entity) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(TARGET_PERCENT),
            height: Val::Percent(TARGET_PERCENT),
            ..default()
        },
        UiTargetCamera(camera),
        Name::new(TWO_D_UI_REFERENCE_NAME),
    ));

    commands
        .spawn((
            absolute_node(FIXED_UI_POSITION, FIXED_UI_SIZE),
            BackgroundColor(FIXED_UI_COLOR),
            UiTargetCamera(camera),
            Name::new(NATES_LIST_NAME),
        ))
        .with_children(|parent| {
            parent.spawn((
                absolute_node(INTERIOR_MARKER_POSITION, INTERIOR_MARKER_SIZE),
                BackgroundColor(INTERIOR_MARKER_COLOR),
            ));
            parent.spawn((
                absolute_node(EDGE_MARKER_POSITION, EDGE_MARKER_SIZE),
                BackgroundColor(EDGE_MARKER_COLOR),
            ));
        });

    commands
        .spawn((
            Node {
                overflow: Overflow::clip(),
                ..absolute_node(ROTATED_UI_PARENT_POSITION, ROTATED_UI_PARENT_SIZE)
            },
            UiTargetCamera(camera),
        ))
        .with_child((
            absolute_node(ROTATED_UI_CHILD_POSITION, ROTATED_UI_CHILD_SIZE),
            BackgroundColor(ROTATED_UI_COLOR),
            UiTransform::from_rotation(Rot2::FRAC_PI_2),
            Name::new(ROTATED_CLIPPED_UI_NAME),
        ));

    commands.spawn((
        absolute_node(HIDDEN_UI_POSITION, HIDDEN_UI_SIZE),
        BackgroundColor(INTERIOR_MARKER_COLOR),
        UiTargetCamera(camera),
        Visibility::Hidden,
        Name::new(HIDDEN_UI_NAME),
    ));
}

fn spawn_two_d_fixtures(commands: &mut Commands) {
    let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    commands.spawn((
        Sprite::from_color(TWO_D_COLOR, TWO_D_ENTITY_SIZE),
        Aabb::from_min_max(
            Vec3::new(
                -TWO_D_ENTITY_SIZE.x / 2.0,
                -TWO_D_ENTITY_SIZE.y / 2.0,
                -AABB_DEPTH / 2.0,
            ),
            Vec3::new(
                TWO_D_ENTITY_SIZE.x / 2.0,
                TWO_D_ENTITY_SIZE.y / 2.0,
                AABB_DEPTH / 2.0,
            ),
        ),
        Transform::from_translation(TWO_D_ENTITY_POSITION)
            .with_rotation(rotation)
            .with_scale(TWO_D_ENTITY_SCALE),
        NoCpuCulling,
        RenderLayers::layer(TWO_D_RENDER_LAYER),
        Name::new(TWO_D_AABB_NAME),
    ));
    commands.spawn((
        Sprite::from_color(INTERIOR_MARKER_COLOR, MARKER_2D_SIZE),
        Transform::from_translation(TWO_D_MARKER_POSITION),
        RenderLayers::layer(TWO_D_RENDER_LAYER),
        Name::new(TWO_D_MARKER_NAME),
    ));

    commands.spawn((
        Aabb::from_min_max(-ERROR_AABB_HALF_EXTENTS, ERROR_AABB_HALF_EXTENTS),
        Transform::from_translation(HIDDEN_AABB_POSITION),
        NoCpuCulling,
        Visibility::Hidden,
        RenderLayers::layer(TWO_D_RENDER_LAYER),
        Name::new(HIDDEN_AABB_NAME),
    ));
    commands.spawn((
        Aabb::from_min_max(-ERROR_AABB_HALF_EXTENTS, ERROR_AABB_HALF_EXTENTS),
        Transform::from_translation(DISJOINT_AABB_POSITION),
        NoCpuCulling,
        RenderLayers::layer(DISJOINT_RENDER_LAYER),
        Name::new(DISJOINT_LAYER_NAME),
    ));
    commands.spawn((
        Aabb::from_min_max(-UNNAMED_AABB_HALF_EXTENTS, UNNAMED_AABB_HALF_EXTENTS),
        Transform::from_translation(UNNAMED_AABB_POSITION),
        NoCpuCulling,
        RenderLayers::layer(TWO_D_RENDER_LAYER),
    ));
}

fn spawn_three_d_fixtures(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    let entity_mesh = meshes.add(Cuboid::new(
        THREE_D_ENTITY_SIZE.x,
        THREE_D_ENTITY_SIZE.y,
        THREE_D_ENTITY_SIZE.z,
    ));
    let entity_material = materials.add(StandardMaterial {
        base_color: THREE_D_COLOR,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(entity_mesh),
        MeshMaterial3d(entity_material),
        Aabb::from_min_max(-THREE_D_ENTITY_SIZE / 2.0, THREE_D_ENTITY_SIZE / 2.0),
        Transform::from_translation(THREE_D_ENTITY_POSITION)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
            .with_scale(THREE_D_ENTITY_SCALE),
        NoCpuCulling,
        RenderLayers::layer(THREE_D_RENDER_LAYER),
        Name::new(THREE_D_AABB_NAME),
    ));

    let marker_mesh = meshes.add(Cuboid::new(
        MARKER_3D_SIZE.x,
        MARKER_3D_SIZE.y,
        MARKER_3D_SIZE.z,
    ));
    let marker_material = materials.add(StandardMaterial {
        base_color: INTERIOR_MARKER_COLOR,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(marker_mesh),
        MeshMaterial3d(marker_material),
        Transform::from_translation(THREE_D_MARKER_POSITION),
        RenderLayers::layer(THREE_D_RENDER_LAYER),
        Name::new(THREE_D_MARKER_NAME),
    ));

    commands.spawn((
        Aabb::from_min_max(
            Vec3::new(
                -VIEWPORT_SIZE.as_vec2().x / 2.0,
                -VIEWPORT_SIZE.as_vec2().y / 2.0,
                -REFERENCE_DEPTH / 2.0,
            ),
            Vec3::new(
                VIEWPORT_SIZE.as_vec2().x / 2.0,
                VIEWPORT_SIZE.as_vec2().y / 2.0,
                REFERENCE_DEPTH / 2.0,
            ),
        ),
        Transform::default(),
        NoCpuCulling,
        RenderLayers::layer(THREE_D_RENDER_LAYER),
        Name::new(THREE_D_REFERENCE_NAME),
    ));
}

fn spawn_error_and_name_fixtures(commands: &mut Commands) {
    commands.spawn((ComputedNode::default(), Name::new(PARTIAL_UI_NAME)));
    commands.spawn(Name::new(UNSUPPORTED_NAME));
    commands.spawn(Name::new(UNIQUE_NAME));
    commands.spawn(Name::new(DUPLICATE_NAME));
    commands.spawn(Name::new(DUPLICATE_NAME));
}

fn establish_initial_camera_epoch(
    mut cameras: Query<(Entity, &RenderTarget, Option<&Camera2d>, &mut Camera)>,
    mut commands: Commands,
    mut state: ResMut<ScreenshotFixtureState>,
) {
    let mut primary_window_camera: Option<Entity> = None;
    for (entity, target, camera_2d, mut camera) in &mut cameras {
        camera.is_active = entity == state.two_d_camera;
        if camera_2d.is_some() && matches!(target, RenderTarget::Window(_)) {
            primary_window_camera = match primary_window_camera {
                Some(current) if current.to_bits() < entity.to_bits() => Some(current),
                _ => Some(entity),
            };
        }
    }

    if let Some(entity) = primary_window_camera {
        commands
            .entity(entity)
            .insert(Name::new(PRIMARY_WINDOW_CAMERA_NAME));
    }
    state.primary_window_camera = primary_window_camera;
}

fn report_fixture_state(state: Res<ScreenshotFixtureState>, images: Res<Assets<Image>>) {
    if images.get(&state.image).is_none() {
        error!("Screenshot fixture target image is not retained in Assets<Image>");
        return;
    }

    info!(
        "Screenshot fixtures ready: image={:?}, primary_window_camera={:?}, two_d_camera={}, three_d_camera={}",
        state.image.id(),
        state.primary_window_camera.map(Entity::to_bits),
        state.two_d_camera.to_bits(),
        state.three_d_camera.to_bits(),
    );
}

fn render_target_image() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: TARGET_SIZE.x,
            height: TARGET_SIZE.y,
            ..default()
        },
        TextureDimension::D2,
        &TARGET_FILL,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC
        | TextureUsages::RENDER_ATTACHMENT;
    image
}

fn fixture_viewport() -> Viewport {
    Viewport {
        physical_position: VIEWPORT_POSITION,
        physical_size: VIEWPORT_SIZE,
        ..default()
    }
}

fn absolute_node(position: Vec2, size: Vec2) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(position.x),
        top: Val::Px(position.y),
        width: Val::Px(size.x),
        height: Val::Px(size.y),
        ..default()
    }
}
