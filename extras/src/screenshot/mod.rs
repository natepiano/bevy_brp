//! Screenshot request handling for BRP extras.

#[cfg(not(target_arch = "wasm32"))]
mod aabb;
#[cfg(not(target_arch = "wasm32"))]
mod capture;
#[cfg(not(target_arch = "wasm32"))]
mod request;
#[cfg(all(feature = "ui", not(target_arch = "wasm32")))]
mod ui;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::RenderAssetUsages;
#[cfg(not(target_arch = "wasm32"))]
use bevy::camera::NormalizedRenderTarget;
#[cfg(not(target_arch = "wasm32"))]
use bevy::camera::RenderTarget;
#[cfg(all(not(feature = "ui"), not(target_arch = "wasm32")))]
use bevy::camera::primitives::Aabb;
#[cfg(not(target_arch = "wasm32"))]
use bevy::camera::primitives::Frustum;
#[cfg(not(target_arch = "wasm32"))]
use bevy::camera::visibility::RenderLayers;
#[cfg(not(target_arch = "wasm32"))]
use bevy::camera::visibility::VisibleEntities;
use bevy::ecs::system::In;
#[cfg(not(target_arch = "wasm32"))]
use bevy::ecs::world::EntityRef;
use bevy::prelude::App;
use bevy::prelude::Plugin;
use bevy::prelude::World;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::texture::ManualTextureViews;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::screenshot::Screenshot;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::PrimaryWindow;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
#[cfg(not(target_arch = "wasm32"))]
use bevy_remote::error_codes::INVALID_PARAMS;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
use self::capture::CaptureInput;
#[cfg(not(target_arch = "wasm32"))]
use self::capture::CapturePlugin;
#[cfg(not(target_arch = "wasm32"))]
use self::capture::PendingScreenshotCaptures;
#[cfg(not(target_arch = "wasm32"))]
use self::request::ScreenshotRequest;
#[cfg(not(target_arch = "wasm32"))]
use self::request::ScreenshotScope;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::CAMERA_CANDIDATES_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::IMAGE_EXTENSION_PNG;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::PARAM_CAMERA;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::PARAM_ENTITY;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::PARAM_PATH;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_BOUNDS_KIND_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_CAPTURE_KIND_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_HEIGHT_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_NAME_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_NOTE_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_REASON_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_RECT_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_STATUS_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_SUCCESS_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_WIDTH_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_WORKING_DIRECTORY_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_X_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_Y_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_BOUNDS_KIND_AABB;
#[cfg(all(feature = "ui", not(target_arch = "wasm32")))]
use crate::constants::SCREENSHOT_BOUNDS_KIND_UI;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_CAMERA_REASON_AMBIGUOUS;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_CAPTURE_KIND_ENTITY;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_CAPTURE_NOTE;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_STATUS_COMPLETED;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::UNKNOWN_WORKING_DIRECTORY;

pub(super) struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(CapturePlugin);
        #[cfg(target_arch = "wasm32")]
        let _ = app;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CaptureResponseMetadata {
    Full,
    Entity(EntityResponseMetadata),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EntityResponseMetadata {
    bounds_kind: BoundsKind,
    camera:      Entity,
    entity:      Entity,
    name:        Option<String>,
    rect:        URect,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundsKind {
    Aabb,
    #[cfg(feature = "ui")]
    Ui,
}

#[cfg(not(target_arch = "wasm32"))]
struct ValidatedCameraTarget {
    camera:            Camera,
    #[cfg(feature = "ui")]
    entity:            Entity,
    normalized_target: NormalizedRenderTarget,
    render_target:     RenderTarget,
    #[cfg(feature = "ui")]
    target_size:       UVec2,
}

#[cfg(not(target_arch = "wasm32"))]
struct SelectedCamera {
    camera:            Camera,
    entity:            Entity,
    frustum:           Frustum,
    global_transform:  GlobalTransform,
    normalized_target: NormalizedRenderTarget,
    render_layers:     Option<RenderLayers>,
    render_target:     RenderTarget,
    visible_entities:  Option<VisibleEntities>,
}

/// Handles the terminal `brp_extras/screenshot` watching request.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>> {
    ensure_png_support()?;

    let request = ScreenshotRequest::from_params(params)?;
    if let Some(response) = capture::read_existing(
        &mut world.resource_mut::<PendingScreenshotCaptures>(),
        &request,
    ) {
        return response;
    }

    if let Some(response) = capture::join_existing(
        &mut world.resource_mut::<PendingScreenshotCaptures>(),
        &request,
    ) {
        return response;
    }

    let capture_input = capture_input(world, &request)?;

    let (response, spawn_target) = capture::handle(
        &mut world.resource_mut::<PendingScreenshotCaptures>(),
        request,
        capture_input,
    )?;

    if let Some(target) = spawn_target {
        capture::spawn_target_batch(world, target);
    }

    Ok(response)
}

/// Returns an actionable error on targets without filesystem publication.
#[cfg(target_arch = "wasm32")]
pub(crate) fn handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>> {
    drop(params);
    let _ = world;
    Err(BrpError {
        code:    INTERNAL_ERROR,
        message: "Screenshot PNG publication is unsupported on WASM; use a native target with filesystem access"
            .to_string(),
        data:    None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn completed_response(path: &Path, metadata: &CaptureResponseMetadata) -> Value {
    let mut response = json!({
        RESPONSE_SUCCESS_FIELD: true,
        PARAM_PATH: path.to_string_lossy(),
        RESPONSE_WORKING_DIRECTORY_FIELD: std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from(UNKNOWN_WORKING_DIRECTORY))
            .to_string_lossy(),
        RESPONSE_NOTE_FIELD: SCREENSHOT_CAPTURE_NOTE,
        RESPONSE_STATUS_FIELD: SCREENSHOT_STATUS_COMPLETED,
    });

    if let CaptureResponseMetadata::Entity(metadata) = metadata {
        response[RESPONSE_CAPTURE_KIND_FIELD] = json!(SCREENSHOT_CAPTURE_KIND_ENTITY);
        response[PARAM_ENTITY] = json!(metadata.entity.to_bits());
        if let Some(name) = &metadata.name {
            response[RESPONSE_NAME_FIELD] = json!(name);
        }
        response[PARAM_CAMERA] = json!(metadata.camera.to_bits());
        response[RESPONSE_BOUNDS_KIND_FIELD] = json!(match metadata.bounds_kind {
            BoundsKind::Aabb => SCREENSHOT_BOUNDS_KIND_AABB,
            #[cfg(feature = "ui")]
            BoundsKind::Ui => SCREENSHOT_BOUNDS_KIND_UI,
        });
        response[RESPONSE_RECT_FIELD] = json!({
            RESPONSE_X_FIELD: metadata.rect.min.x,
            RESPONSE_Y_FIELD: metadata.rect.min.y,
            RESPONSE_WIDTH_FIELD: metadata.rect.width(),
            RESPONSE_HEIGHT_FIELD: metadata.rect.height(),
        });
    }

    response
}

#[cfg(not(target_arch = "wasm32"))]
fn capture_input(world: &mut World, request: &ScreenshotRequest) -> BrpResult<CaptureInput> {
    match request.scope() {
        ScreenshotScope::Full => full_capture_input(world),
        ScreenshotScope::Entity {
            entity,
            camera,
            padding,
        } => entity_capture_input(world, *entity, *camera, *padding),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn full_capture_input(world: &World) -> BrpResult<CaptureInput> {
    let primary_window = primary_window(world).ok_or_else(no_primary_window_error)?;
    let render_target = Screenshot::primary_window().0;
    let normalized_target = render_target
        .normalize(Some(primary_window))
        .ok_or_else(no_primary_window_error)?;
    if live_target_size(world, &normalized_target).is_none() {
        return Err(no_primary_window_error());
    }

    Ok(CaptureInput {
        crop: None,
        normalized_target,
        render_target,
        response_metadata: CaptureResponseMetadata::Full,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn entity_capture_input(
    world: &mut World,
    entity: Entity,
    requested_camera: Option<Entity>,
    padding: u32,
) -> BrpResult<CaptureInput> {
    if world.get_entity(entity).is_err() {
        return Err(invalid_entity_error(entity));
    }

    #[cfg(feature = "ui")]
    if let Some(resolved) = ui::resolve(world, entity, requested_camera, padding)? {
        let camera = resolved.camera;
        return Ok(entity_capture_from_parts(
            world,
            entity,
            camera.entity,
            camera.normalized_target,
            camera.render_target,
            resolved.rect,
            BoundsKind::Ui,
        ));
    }

    #[cfg(not(feature = "ui"))]
    if world.get::<Aabb>(entity).is_none() {
        return Err(unsupported_bounds_error(entity));
    }

    let selected_camera = select_camera(world, requested_camera)?;
    let rect = aabb::resolve(world, entity, &selected_camera, padding)?;
    Ok(entity_capture_from_parts(
        world,
        entity,
        selected_camera.entity,
        selected_camera.normalized_target,
        selected_camera.render_target,
        rect,
        BoundsKind::Aabb,
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn entity_capture_from_parts(
    world: &World,
    entity: Entity,
    camera: Entity,
    normalized_target: NormalizedRenderTarget,
    render_target: RenderTarget,
    rect: URect,
    bounds_kind: BoundsKind,
) -> CaptureInput {
    let name = world
        .get::<Name>(entity)
        .map(|name| name.as_str().to_owned());

    CaptureInput {
        crop: Some(rect),
        normalized_target,
        render_target,
        response_metadata: CaptureResponseMetadata::Entity(EntityResponseMetadata {
            bounds_kind,
            camera,
            entity,
            name,
            rect,
        }),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn select_camera(world: &mut World, requested: Option<Entity>) -> BrpResult<SelectedCamera> {
    let primary_window = primary_window(world);
    if let Some(camera) = requested {
        return eligible_camera(world, camera, primary_window)
            .ok_or_else(|| invalid_camera_error(camera));
    }

    let mut camera_query = world.query_filtered::<Entity, With<Camera>>();
    let mut candidates = camera_query
        .iter(world)
        .filter_map(|entity| eligible_camera(world, entity, primary_window))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.entity.to_bits());

    match candidates.len() {
        0 => Err(no_camera_error()),
        1 => candidates.pop().ok_or_else(no_camera_error),
        _ => Err(ambiguous_camera_error(&candidates)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn eligible_camera(
    world: &World,
    entity: Entity,
    primary_window: Option<Entity>,
) -> Option<SelectedCamera> {
    let validated = validated_camera_target(world, entity, primary_window)?;

    Some(SelectedCamera {
        camera: validated.camera,
        entity,
        frustum: *world.get::<Frustum>(entity)?,
        global_transform: *world.get::<GlobalTransform>(entity)?,
        normalized_target: validated.normalized_target,
        render_layers: world.get::<RenderLayers>(entity).cloned(),
        render_target: validated.render_target,
        visible_entities: world.get::<VisibleEntities>(entity).cloned(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn validated_camera_target(
    world: &World,
    entity: Entity,
    primary_window: Option<Entity>,
) -> Option<ValidatedCameraTarget> {
    let camera = world.get::<Camera>(entity)?;
    let render_target = world.get::<RenderTarget>(entity)?;
    if !camera.is_active
        || !camera.physical_target_size()?.cmpgt(UVec2::ZERO).all()
        || camera.physical_viewport_rect()?.is_empty()
        || matches!(render_target, RenderTarget::None { .. })
    {
        return None;
    }
    let normalized_target = render_target.normalize(primary_window)?;
    #[cfg(feature = "ui")]
    let target_size = live_target_size(world, &normalized_target)?;
    #[cfg(not(feature = "ui"))]
    live_target_size(world, &normalized_target)?;

    Some(ValidatedCameraTarget {
        camera: camera.clone(),
        #[cfg(feature = "ui")]
        entity,
        normalized_target,
        render_target: render_target.clone(),
        #[cfg(feature = "ui")]
        target_size,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn live_target_size(world: &World, target: &NormalizedRenderTarget) -> Option<UVec2> {
    let size = match target {
        NormalizedRenderTarget::Window(window) => {
            world.get::<Window>(window.entity())?.physical_size()
        },
        NormalizedRenderTarget::Image(image_target) => {
            let image = world
                .get_resource::<Assets<Image>>()?
                .get(&image_target.handle)?;
            if !image.asset_usage.contains(RenderAssetUsages::RENDER_WORLD) {
                return None;
            }
            image.size()
        },
        NormalizedRenderTarget::TextureView(handle) => {
            world
                .get_resource::<ManualTextureViews>()?
                .get(handle)?
                .size
        },
        NormalizedRenderTarget::None { .. } => return None,
    };

    size.cmpgt(UVec2::ZERO).all().then_some(size)
}

#[cfg(not(target_arch = "wasm32"))]
fn primary_window(world: &World) -> Option<Entity> {
    world
        .iter_entities()
        .find(EntityRef::contains::<PrimaryWindow>)
        .map(|entity| entity.id())
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_png_support() -> BrpResult<()> {
    if bevy::image::ImageFormat::from_extension(IMAGE_EXTENSION_PNG).is_some() {
        return Ok(());
    }

    Err(BrpError {
        code:    INTERNAL_ERROR,
        message: "PNG support not available. Enable the 'png' feature in your Bevy dependency"
            .to_string(),
        data:    None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn no_primary_window_error() -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: "Screenshot capture requires a primary window".to_string(),
        data:    None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn invalid_entity_error(entity: Entity) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!("Invalid screenshot entity: {}", entity.to_bits()),
        data:    None,
    }
}

#[cfg(all(not(feature = "ui"), not(target_arch = "wasm32")))]
fn unsupported_bounds_error(entity: Entity) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!(
            "Screenshot entity {} has no supported bounds; UI bounds support is disabled",
            entity.to_bits()
        ),
        data:    None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn invalid_camera_error(camera: Entity) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!(
            "Screenshot camera {} is missing, inactive, uninitialized, or has an unsupported target",
            camera.to_bits()
        ),
        data:    None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn no_camera_error() -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: "Entity screenshot capture requires one eligible active camera".to_string(),
        data:    None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ambiguous_camera_error(candidates: &[SelectedCamera]) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: "Entity screenshot capture has multiple eligible active cameras".to_string(),
        data:    Some(json!({
            RESPONSE_REASON_FIELD: SCREENSHOT_CAMERA_REASON_AMBIGUOUS,
            CAMERA_CANDIDATES_FIELD: candidates
                .iter()
                .map(|candidate| candidate.entity.to_bits())
                .collect::<Vec<_>>(),
        })),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    use std::error::Error;
    use std::io;
    use std::io::Error as IoError;
    use std::time::Duration;
    use std::time::Instant;

    use async_channel::Receiver as AsyncReceiver;
    use async_channel::TryRecvError;
    use bevy::MinimalPlugins;
    use bevy::camera::ComputedCameraValues;
    use bevy::camera::ManualTextureViewHandle;
    use bevy::camera::RenderTargetInfo;
    use bevy::camera::primitives::Aabb;
    use bevy::ecs::system::SystemId;
    use bevy::math::primitives::ViewFrustum;
    use bevy::render::render_resource::Extent3d;
    use bevy::render::render_resource::TextureDimension;
    use bevy::render::render_resource::TextureFormat;
    use bevy::render::texture::ManualTextureView;
    use bevy::render::view::screenshot::ScreenshotCaptured;
    use bevy::window::WindowRef;
    use bevy_remote::BrpMessage;
    use bevy_remote::BrpSender;
    use bevy_remote::RemoteMethodSystemId;
    use bevy_remote::RemoteMethods;
    use bevy_remote::RemotePlugin;
    use image::GenericImageView;
    use serde_json::Value;
    use serde_json::json;
    use tempfile::TempDir;
    use wgpu::Device;
    use wgpu::Queue;
    use wgpu::Texture;
    use wgpu::TextureDescriptor;

    use super::*;
    use crate::constants::METHOD_SCREENSHOT;

    const CAPTURE_TEST_TIMEOUT: Duration = Duration::from_secs(5);

    #[derive(Resource, Default)]
    struct TestManualTextureOwners {
        resources: Vec<(Device, Queue, Texture)>,
    }

    fn brp<T>(result: BrpResult<T>) -> Result<T, IoError> {
        result.map_err(|error| io::Error::other(error.message))
    }

    fn spawn_camera(
        world: &mut World,
        render_target: RenderTarget,
        target_size: Option<UVec2>,
    ) -> Entity {
        let clip_from_view = Mat4::IDENTITY;
        world
            .spawn((
                Camera {
                    computed: ComputedCameraValues {
                        clip_from_view,
                        target_info: target_size.map(|physical_size| RenderTargetInfo {
                            physical_size,
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
                Frustum(ViewFrustum::from_clip_from_world(&clip_from_view)),
                GlobalTransform::IDENTITY,
                render_target,
                VisibleEntities::default(),
            ))
            .id()
    }

    fn spawn_aabb_entity(world: &mut World, name: &str) -> Entity {
        world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.5)),
                Name::new(name.to_owned()),
            ))
            .id()
    }

    fn render_target_image(size: UVec2, asset_usage: RenderAssetUsages) -> Image {
        Image::new_fill(
            Extent3d {
                width:                 size.x,
                height:                size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            asset_usage,
        )
    }

    fn add_render_target_image(world: &mut World, size: UVec2) -> Handle<Image> {
        world.init_resource::<Assets<Image>>();
        world
            .resource_mut::<Assets<Image>>()
            .add(render_target_image(size, RenderAssetUsages::default()))
    }

    fn add_manual_texture_view(world: &mut World, handle: ManualTextureViewHandle, size: UVec2) {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let texture = device.create_texture(&TextureDescriptor {
            label:           None,
            size:            wgpu::Extent3d {
                width:                 size.x,
                height:                size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          wgpu::TextureFormat::Rgba8UnormSrgb,
            usage:           wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats:    &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        world.init_resource::<TestManualTextureOwners>();
        world
            .resource_mut::<TestManualTextureOwners>()
            .resources
            .push((device, queue, texture));
        world.init_resource::<ManualTextureViews>();
        world.resource_mut::<ManualTextureViews>().insert(
            handle,
            ManualTextureView::with_default_format(texture_view.into(), size),
        );
    }

    fn run_handler(
        world: &mut World,
        system_id: SystemId<In<Option<Value>>, BrpResult<Option<Value>>>,
        params: Value,
    ) -> BrpResult<Option<Value>> {
        world
            .run_system_with(system_id, Some(params))
            .map_err(|error| BrpError {
                code:    INTERNAL_ERROR,
                message: error.to_string(),
                data:    None,
            })?
    }

    fn send_remote_request(
        app: &App,
        params: Value,
    ) -> Result<AsyncReceiver<BrpResult<Value>>, IoError> {
        let (response_sender, response_receiver) = async_channel::bounded(1);
        app.world()
            .resource::<BrpSender>()
            .force_send(BrpMessage {
                method: METHOD_SCREENSHOT.to_string(),
                params: Some(params),
                sender: response_sender,
            })
            .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(response_receiver)
    }

    fn receive_terminal(
        app: &mut App,
        receiver: &AsyncReceiver<BrpResult<Value>>,
    ) -> Result<Value, IoError> {
        let deadline = Instant::now() + CAPTURE_TEST_TIMEOUT;
        loop {
            app.update();
            match receiver.try_recv() {
                Ok(response) => return brp(response),
                Err(TryRecvError::Empty) if Instant::now() < deadline => {
                    std::thread::yield_now();
                },
                Err(error) => {
                    return Err(io::Error::other(format!(
                        "screenshot response did not complete: {error}"
                    )));
                },
            }
        }
    }

    fn screenshot_entity_for_target(
        world: &mut World,
        target: &RenderTarget,
    ) -> Result<Entity, IoError> {
        world
            .query::<(Entity, &Screenshot)>()
            .iter(world)
            .find_map(|(entity, screenshot)| match (&screenshot.0, target) {
                (RenderTarget::Image(actual), RenderTarget::Image(expected))
                    if actual.handle == expected.handle =>
                {
                    Some(entity)
                },
                (RenderTarget::TextureView(actual), RenderTarget::TextureView(expected))
                    if actual == expected =>
                {
                    Some(entity)
                },
                _ => None,
            })
            .ok_or_else(|| io::Error::other("missing screenshot entity for render target"))
    }

    fn assert_terminal_png(response: &Value, path: &Path) -> Result<(), Box<dyn Error>> {
        assert_eq!(
            response.get(RESPONSE_SUCCESS_FIELD),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            response.get(RESPONSE_STATUS_FIELD).and_then(Value::as_str),
            Some(SCREENSHOT_STATUS_COMPLETED)
        );
        assert_eq!(
            response.get(PARAM_PATH).and_then(Value::as_str),
            Some(path.to_string_lossy().as_ref())
        );
        let png = image::open(path)?;
        assert_eq!(png.color(), image::ColorType::Rgb8);
        let rect = response
            .get(RESPONSE_RECT_FIELD)
            .ok_or_else(|| io::Error::other("missing terminal capture rectangle"))?;
        assert_eq!(
            png.dimensions(),
            (
                rect.get(RESPONSE_WIDTH_FIELD)
                    .and_then(Value::as_u64)
                    .ok_or_else(|| io::Error::other("missing capture width"))?
                    .try_into()?,
                rect.get(RESPONSE_HEIGHT_FIELD)
                    .and_then(Value::as_u64)
                    .ok_or_else(|| io::Error::other("missing capture height"))?
                    .try_into()?,
            )
        );
        Ok(())
    }

    #[test]
    fn completed_response_preserves_existing_fields_and_adds_terminal_status() {
        let response = completed_response(
            Path::new("/tmp/screenshot.png"),
            &CaptureResponseMetadata::Full,
        );

        assert_eq!(
            response.get(RESPONSE_SUCCESS_FIELD),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            response.get(PARAM_PATH).and_then(Value::as_str),
            Some("/tmp/screenshot.png")
        );
        assert!(response.get(RESPONSE_WORKING_DIRECTORY_FIELD).is_some());
        assert_eq!(
            response.get(RESPONSE_NOTE_FIELD).and_then(Value::as_str),
            Some(SCREENSHOT_CAPTURE_NOTE)
        );
        assert_eq!(
            response.get(RESPONSE_STATUS_FIELD).and_then(Value::as_str),
            Some(SCREENSHOT_STATUS_COMPLETED)
        );
    }

    #[test]
    fn entity_response_adds_snapshotted_metadata() -> Result<(), Box<dyn Error>> {
        let mut world = World::new();
        let target_window = world.spawn(Window::default()).id();
        let camera = spawn_camera(
            &mut world,
            RenderTarget::Window(WindowRef::Entity(target_window)),
            Some(UVec2::splat(100)),
        );
        let entity = spawn_aabb_entity(&mut world, "Before");
        let capture_input = entity_capture_input(&mut world, entity, Some(camera), 0)
            .map_err(|error| io::Error::other(error.message))?;
        world.entity_mut(entity).insert(Name::new("After"));
        let response = completed_response(
            Path::new("/tmp/entity.png"),
            &capture_input.response_metadata,
        );

        assert_eq!(
            response
                .get(RESPONSE_CAPTURE_KIND_FIELD)
                .and_then(Value::as_str),
            Some(SCREENSHOT_CAPTURE_KIND_ENTITY)
        );
        assert_eq!(
            response.get(PARAM_ENTITY).and_then(Value::as_u64),
            Some(entity.to_bits())
        );
        assert_eq!(
            response.get(RESPONSE_NAME_FIELD).and_then(Value::as_str),
            Some("Before")
        );
        assert_eq!(
            response.get(PARAM_CAMERA).and_then(Value::as_u64),
            Some(camera.to_bits())
        );
        assert_eq!(
            response
                .get(RESPONSE_BOUNDS_KIND_FIELD)
                .and_then(Value::as_str),
            Some(SCREENSHOT_BOUNDS_KIND_AABB)
        );
        assert!(response.get(RESPONSE_RECT_FIELD).is_some());
        Ok(())
    }

    #[cfg(not(feature = "ui"))]
    #[test]
    fn non_aabb_capture_names_disabled_ui_support() -> Result<(), Box<dyn Error>> {
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        let error = entity_capture_input(&mut world, entity, None, 0)
            .err()
            .ok_or_else(|| io::Error::other("unsupported bounds did not fail"))?;

        assert!(error.message.contains("UI bounds support is disabled"));
        Ok(())
    }

    #[test]
    fn camera_selection_handles_explicit_inferred_and_ambiguous_views() -> Result<(), Box<dyn Error>>
    {
        let mut world = World::new();
        let target_window = world.spawn(Window::default()).id();
        assert!(select_camera(&mut world, None).is_err());

        let first = spawn_camera(
            &mut world,
            RenderTarget::Window(WindowRef::Entity(target_window)),
            Some(UVec2::splat(100)),
        );
        assert_eq!(brp(select_camera(&mut world, Some(first)))?.entity, first);
        assert_eq!(brp(select_camera(&mut world, None))?.entity, first);

        let second = spawn_camera(
            &mut world,
            RenderTarget::Window(WindowRef::Entity(target_window)),
            Some(UVec2::splat(100)),
        );
        let ambiguity = select_camera(&mut world, None)
            .err()
            .ok_or_else(|| io::Error::other("multiple cameras were not ambiguous"))?;
        let candidates = ambiguity
            .data
            .as_ref()
            .and_then(|data| data.get(CAMERA_CANDIDATES_FIELD))
            .and_then(Value::as_array)
            .ok_or_else(|| io::Error::other("missing camera candidate data"))?;
        let mut expected = vec![first.to_bits(), second.to_bits()];
        expected.sort_unstable();
        assert_eq!(
            candidates,
            &expected.into_iter().map(Value::from).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn camera_selection_rejects_inactive_uninitialized_and_none_targets()
    -> Result<(), Box<dyn Error>> {
        let mut world = World::new();
        let target_window = world.spawn(Window::default()).id();
        let inactive = spawn_camera(
            &mut world,
            RenderTarget::Window(WindowRef::Entity(target_window)),
            Some(UVec2::splat(100)),
        );
        world
            .get_mut::<Camera>(inactive)
            .ok_or_else(|| IoError::other("missing inactive test camera"))?
            .is_active = false;
        let uninitialized = spawn_camera(
            &mut world,
            RenderTarget::Window(WindowRef::Entity(target_window)),
            None,
        );
        let no_target = spawn_camera(
            &mut world,
            RenderTarget::None {
                size: UVec2::splat(100),
            },
            Some(UVec2::splat(100)),
        );
        let removed_window = world.spawn(Window::default()).id();
        let removed_window_camera = spawn_camera(
            &mut world,
            RenderTarget::Window(WindowRef::Entity(removed_window)),
            Some(UVec2::splat(100)),
        );
        world.despawn(removed_window);

        assert!(select_camera(&mut world, Some(inactive)).is_err());
        assert!(select_camera(&mut world, Some(uninitialized)).is_err());
        assert!(select_camera(&mut world, Some(no_target)).is_err());
        assert!(select_camera(&mut world, Some(removed_window_camera)).is_err());
        Ok(())
    }

    #[test]
    fn camera_selection_requires_live_image_and_manual_texture_targets()
    -> Result<(), Box<dyn Error>> {
        let mut world = World::new();
        let target_size = UVec2::splat(100);
        let image_handle = add_render_target_image(&mut world, target_size);
        let manual_handle = ManualTextureViewHandle::default();
        add_manual_texture_view(&mut world, manual_handle, target_size);
        let image = spawn_camera(
            &mut world,
            RenderTarget::Image(image_handle.clone().into()),
            Some(target_size),
        );
        let manual = spawn_camera(
            &mut world,
            RenderTarget::TextureView(manual_handle),
            Some(target_size),
        );

        assert_eq!(brp(select_camera(&mut world, Some(image)))?.entity, image);
        assert_eq!(brp(select_camera(&mut world, Some(manual)))?.entity, manual);
        world.resource_mut::<Assets<Image>>().remove(&image_handle);
        world
            .resource_mut::<ManualTextureViews>()
            .remove(&manual_handle);
        assert!(select_camera(&mut world, Some(image)).is_err());
        assert!(select_camera(&mut world, Some(manual)).is_err());
        let main_world_only = render_target_image(target_size, RenderAssetUsages::MAIN_WORLD);
        world
            .resource_mut::<Assets<Image>>()
            .insert(image_handle.id(), main_world_only)?;
        assert!(select_camera(&mut world, Some(image)).is_err());
        Ok(())
    }

    #[test]
    fn camera_selection_accepts_render_world_only_image_targets() -> Result<(), Box<dyn Error>> {
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        let target_size = UVec2::splat(100);
        let image_handle = world
            .resource_mut::<Assets<Image>>()
            .add(render_target_image(
                target_size,
                RenderAssetUsages::RENDER_WORLD,
            ));
        let camera = spawn_camera(
            &mut world,
            RenderTarget::Image(image_handle.into()),
            Some(target_size),
        );

        assert_eq!(brp(select_camera(&mut world, Some(camera)))?.entity, camera);
        Ok(())
    }

    #[test]
    fn handler_spawns_screenshots_with_concrete_image_and_manual_targets()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let image_path = temp_dir.path().join("image.png");
        let manual_path = temp_dir.path().join("manual.png");
        let target_size = UVec2::splat(100);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RemotePlugin::default(), ScreenshotPlugin));
        let system_id = app.world_mut().register_system(handler);
        app.world_mut()
            .resource_mut::<RemoteMethods>()
            .insert(METHOD_SCREENSHOT, RemoteMethodSystemId::Watching(system_id));
        let image_handle = add_render_target_image(app.world_mut(), target_size);
        let manual_handle = ManualTextureViewHandle::default();
        add_manual_texture_view(app.world_mut(), manual_handle, target_size);
        let image_camera = spawn_camera(
            app.world_mut(),
            RenderTarget::Image(image_handle.clone().into()),
            Some(target_size),
        );
        let manual_camera = spawn_camera(
            app.world_mut(),
            RenderTarget::TextureView(manual_handle),
            Some(target_size),
        );
        let entity = spawn_aabb_entity(app.world_mut(), "Concrete target");
        app.update();
        let image_receiver = send_remote_request(
            &app,
            json!({
                "camera": image_camera.to_bits(),
                "capture_id": "image",
                "entity": entity.to_bits(),
                "path": image_path,
            }),
        )?;
        let manual_receiver = send_remote_request(
            &app,
            json!({
                "camera": manual_camera.to_bits(),
                "capture_id": "manual",
                "entity": entity.to_bits(),
                "path": manual_path,
            }),
        )?;
        app.update();

        let image_target = RenderTarget::Image(image_handle.into());
        let manual_target = RenderTarget::TextureView(manual_handle);
        let image_screenshot = screenshot_entity_for_target(app.world_mut(), &image_target)?;
        let manual_screenshot = screenshot_entity_for_target(app.world_mut(), &manual_target)?;
        assert_ne!(image_screenshot, manual_screenshot);
        for screenshot_entity in [image_screenshot, manual_screenshot] {
            app.world_mut()
                .entity_mut(screenshot_entity)
                .trigger(|entity| ScreenshotCaptured {
                    entity,
                    image: render_target_image(target_size, RenderAssetUsages::MAIN_WORLD),
                });
        }

        let image_response = receive_terminal(&mut app, &image_receiver)?;
        let manual_response = receive_terminal(&mut app, &manual_receiver)?;
        assert_terminal_png(&image_response, &image_path)?;
        assert_terminal_png(&manual_response, &manual_path)?;
        assert_eq!(
            app.world()
                .resource::<TestManualTextureOwners>()
                .resources
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn watcher_identity_and_generation_join_precede_entity_resolution() -> Result<(), Box<dyn Error>>
    {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("entity.png");
        let mut app = App::new();
        app.add_plugins(ScreenshotPlugin);
        let primary_window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let camera = spawn_camera(
            app.world_mut(),
            RenderTarget::Window(WindowRef::Primary),
            Some(UVec2::splat(100)),
        );
        let entity = spawn_aabb_entity(app.world_mut(), "Snapshot");
        let system_id = app.world_mut().register_system(handler);
        let params = json!({
            "camera": camera.to_bits(),
            "capture_id": "first",
            "entity": entity.to_bits(),
            "path": path,
        });

        assert!(brp(run_handler(app.world_mut(), system_id, params.clone()))?.is_none());
        let mut same_token_different_scope = params.clone();
        same_token_different_scope["padding"] = json!(1);
        assert!(run_handler(app.world_mut(), system_id, same_token_different_scope).is_err());
        let mut different_token_different_scope = params.clone();
        different_token_different_scope["capture_id"] = json!("conflict");
        different_token_different_scope["padding"] = json!(1);
        assert!(run_handler(app.world_mut(), system_id, different_token_different_scope).is_err());
        app.world_mut().despawn(entity);
        app.world_mut().despawn(camera);
        app.world_mut().despawn(primary_window);
        assert!(brp(run_handler(app.world_mut(), system_id, params.clone()))?.is_none());

        let mut joined = params;
        joined["capture_id"] = json!("second");
        assert!(brp(run_handler(app.world_mut(), system_id, joined))?.is_none());
        Ok(())
    }

    #[test]
    fn full_and_entity_requests_share_one_normalized_target_capture() -> Result<(), Box<dyn Error>>
    {
        let temp_dir = TempDir::new()?;
        let mut app = App::new();
        app.add_plugins(ScreenshotPlugin);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let camera = spawn_camera(
            app.world_mut(),
            RenderTarget::Window(WindowRef::Primary),
            Some(UVec2::splat(100)),
        );
        let entity = spawn_aabb_entity(app.world_mut(), "Shared Target");
        let system_id = app.world_mut().register_system(handler);

        brp(run_handler(
            app.world_mut(),
            system_id,
            json!({
                "capture_id": "full",
                "path": temp_dir.path().join("full.png"),
            }),
        ))?;
        brp(run_handler(
            app.world_mut(),
            system_id,
            json!({
                "camera": camera.to_bits(),
                "capture_id": "entity",
                "entity": entity.to_bits(),
                "path": temp_dir.path().join("entity.png"),
            }),
        ))?;

        let mut query = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
        assert_eq!(query.iter(app.world()).count(), 1);
        Ok(())
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use std::error::Error;
    use std::io;

    use bevy::prelude::*;
    use serde_json::json;

    use super::*;

    #[test]
    fn unsupported_publication_returns_before_resource_or_job_creation()
    -> Result<(), Box<dyn Error>> {
        let mut app = App::new();
        app.add_plugins(ScreenshotPlugin);
        let initial_entities = app.world().entities().len();
        let system_id = app.world_mut().register_system(handler);

        let result = app
            .world_mut()
            .run_system_with(system_id, Some(json!({ "path": 42 })))
            .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(matches!(
            result,
            Err(error) if error.message.contains("unsupported on WASM")
        ));
        assert_eq!(app.world().entities().len(), initial_entities);
        Ok(())
    }
}
