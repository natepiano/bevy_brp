use std::error::Error;
use std::fs;
use std::io;
use std::io::Error as IoError;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use async_channel::Receiver as AsyncReceiver;
use async_channel::TryRecvError;
use async_channel::bounded;
use bevy::MinimalPlugins;
use bevy::asset::RenderAssetUsages;
use bevy::camera::ComputedCameraValues;
use bevy::camera::RenderTarget;
use bevy::camera::RenderTargetInfo;
use bevy::camera::primitives::Aabb;
use bevy::camera::primitives::Frustum;
use bevy::camera::visibility::VisibleEntities;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::SystemId;
use bevy::math::primitives::ViewFrustum;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::window::PrimaryWindow;
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

use super::*;
use crate::constants::METHOD_SCREENSHOT;
use crate::screenshot;
use crate::screenshot::ScreenshotPlugin;
use crate::screenshot::capture::screenshot_job;
use crate::screenshot::capture::screenshot_job::CaptureMetadata;
use crate::screenshot::capture::target_rgb_image::TargetRgbImage;

type TestResult = Result<(), Box<dyn Error>>;

const FIRST_PIXEL: [u8; 4] = [10, 20, 30, 240];
const SECOND_PIXEL: [u8; 4] = [40, 50, 60, 230];
const THIRD_PIXEL: [u8; 4] = [70, 80, 90, 220];
const FOURTH_PIXEL: [u8; 4] = [100, 110, 120, 210];
const TEST_ENTITY_INDEX: u32 = 1;
const WORKER_TEST_TIMEOUT: Duration = Duration::from_secs(5);
static CONVERSIONS: AtomicUsize = AtomicUsize::new(0);

fn brp<T>(result: BrpResult<T>) -> Result<T, IoError> {
    result.map_err(|error| io::Error::other(error.message))
}

fn request(path: &Path, capture_id: Option<&str>) -> Result<ScreenshotRequest, IoError> {
    let mut params = json!({ "path": path });
    if let Some(capture_id) = capture_id {
        params["capture_id"] = json!(capture_id);
    }
    brp(ScreenshotRequest::from_params(Some(params)))
}

fn target() -> Result<NormalizedRenderTarget, IoError> {
    let entity = Entity::from_raw_u32(TEST_ENTITY_INDEX)
        .ok_or_else(|| io::Error::other("test entity index was invalid"))?;
    RenderTarget::Window(WindowRef::Primary)
        .normalize(Some(entity))
        .ok_or_else(|| io::Error::other("primary target did not normalize"))
}

fn capture_input() -> Result<CaptureInput, IoError> {
    Ok(CaptureInput {
        crop:              None,
        normalized_target: target()?,
        render_target:     RenderTarget::Window(WindowRef::Primary),
        response_metadata: CaptureResponseMetadata::Full,
    })
}

fn begin_frame(pending: &mut PendingScreenshotCaptures) { pending.begin_frame(Vec::new()); }

fn finish_frame(pending: &mut PendingScreenshotCaptures) {
    assert!(pending.advance(Instant::now()).is_empty());
    pending.cleanup();
}

fn finish_frame_at(pending: &mut PendingScreenshotCaptures, now: Instant) {
    assert!(pending.advance(now).is_empty());
    pending.cleanup();
}

fn handle(
    pending: &mut PendingScreenshotCaptures,
    path: &Path,
    capture_id: Option<&str>,
) -> Result<CaptureDispatch, IoError> {
    brp(pending.handle(request(path, capture_id)?, capture_input()?))
}

fn handle_at(
    pending: &mut PendingScreenshotCaptures,
    path: &Path,
    capture_id: Option<&str>,
    now: Instant,
) -> Result<CaptureDispatch, IoError> {
    brp(pending.handle_at(request(path, capture_id)?, capture_input()?, now))
}

fn identity(path: &Path, capture_id: Option<&str>) -> Result<CaptureIdentity, IoError> {
    Ok(request(path, capture_id)?.identity().clone())
}

fn ready_completion(
    pending: &PendingScreenshotCaptures,
    path: &Path,
    bytes: &[u8],
) -> Result<WorkerCompletion, IoError> {
    let reservation = pending
        .reservations
        .get(path)
        .ok_or_else(|| io::Error::other("missing destination reservation"))?;
    let temp_path = brp(screenshot_job::create_temporary_file(path, bytes))?;
    Ok(WorkerCompletion {
        deadline:        reservation.worker_deadline,
        identity:        reservation.owner.clone(),
        path:            path.to_path_buf(),
        path_generation: reservation.generation,
        result:          Ok(OwnedTempCapture {
            identity: reservation.owner.clone(),
            metadata: CaptureMetadata {
                dimensions:        UVec2::ONE,
                response_metadata: reservation.response_metadata.clone(),
            },
            path_generation: reservation.generation,
            temp_path,
        }),
    })
}

fn progress_to_publication(
    pending: &mut PendingScreenshotCaptures,
    path: &Path,
    capture_id: Option<&str>,
    bytes: &[u8],
) -> Result<(), IoError> {
    let completion = ready_completion(pending, path, bytes)?;
    pending.begin_frame(vec![completion]);
    handle(pending, path, capture_id)?;
    finish_frame(pending);
    assert!(!path.exists());

    begin_frame(pending);
    handle(pending, path, capture_id)?;
    finish_frame(pending);
    Ok(())
}

fn test_image() -> Image {
    Image::new(
        Extent3d {
            width:                 2,
            height:                2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        [FIRST_PIXEL, SECOND_PIXEL, THIRD_PIXEL, FOURTH_PIXEL].concat(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    )
}

fn counting_conversion(image: Image) -> BrpResult<TargetRgbImage> {
    CONVERSIONS.fetch_add(1, Ordering::SeqCst);
    TargetRgbImage::try_from(image)
}

fn spawn_camera(world: &mut World, render_target: RenderTarget, target_size: UVec2) -> Entity {
    let clip_from_view = Mat4::IDENTITY;
    world
        .spawn((
            Camera {
                computed: ComputedCameraValues {
                    clip_from_view,
                    target_info: Some(RenderTargetInfo {
                        physical_size: target_size,
                        scale_factor:  1.0,
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

fn run_handler(
    world: &mut World,
    system_id: SystemId<In<Option<Value>>, BrpResult<Option<Value>>>,
    params: &Value,
) -> BrpResult<Option<Value>> {
    world
        .run_system_with(system_id, Some(params.clone()))
        .map_err(|error| BrpError {
            code:    INTERNAL_ERROR,
            message: error.to_string(),
            data:    None,
        })?
}

fn begin_app_frame(app: &mut App, completions: Vec<WorkerCompletion>) {
    app.world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .begin_frame(completions);
}

fn finish_app_frame(app: &mut App) {
    let expired = app
        .world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .advance(Instant::now());
    assert!(expired.is_empty());
    app.world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .cleanup();
}

fn receive_worker_completions(app: &App, count: usize) -> Result<Vec<WorkerCompletion>, IoError> {
    let channel = app.world().resource::<CaptureCompletionChannel>();
    let receiver = channel
        .receiver
        .lock()
        .map_err(|_| io::Error::other("completion channel mutex poisoned"))?;
    (0..count)
        .map(|_| {
            receiver
                .recv_timeout(WORKER_TEST_TIMEOUT)
                .map_err(IoError::other)
        })
        .collect()
}

fn image_with_size(size: UVec2) -> Image {
    Image::new_fill(
        Extent3d {
            width:                 size.x,
            height:                size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[10, 20, 30, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    )
}

fn remote_app() -> App {
    let mut app = App::new();
    app.add_plugins((RemotePlugin::default(), ScreenshotPlugin));
    let system_id = app.world_mut().register_system(screenshot::handler);
    app.world_mut()
        .resource_mut::<RemoteMethods>()
        .insert(METHOD_SCREENSHOT, RemoteMethodSystemId::Watching(system_id));
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    app
}

fn send_remote_request(
    app: &App,
    path: &Path,
    capture_id: &str,
) -> Result<AsyncReceiver<BrpResult<Value>>, IoError> {
    let (response_sender, response_receiver) = bounded(1);
    app.world()
        .resource::<BrpSender>()
        .force_send(BrpMessage {
            method: METHOD_SCREENSHOT.to_string(),
            params: Some(json!({
                "capture_id": capture_id,
                "path": path,
            })),
            sender: response_sender,
        })
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(response_receiver)
}

fn receive_terminal(response_receiver: &AsyncReceiver<BrpResult<Value>>) -> Result<Value, IoError> {
    let response = response_receiver
        .try_recv()
        .map_err(|error| io::Error::other(error.to_string()))?;
    brp(response)
}

#[test]
fn repeated_tokens_are_idempotent_and_distinct_tokens_are_isolated() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("shared.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);

    let first = handle(&mut pending, &path, Some("first"))?;
    let repeated = handle(&mut pending, &path, Some("first"))?;
    let second = handle(&mut pending, &path, Some("second"))?;

    assert!(first.spawn_target.is_some());
    assert!(repeated.spawn_target.is_none());
    assert!(second.spawn_target.is_none());
    assert_eq!(pending.captures.len(), 2);
    assert_eq!(pending.reservations.len(), 1);
    let jobs = pending
        .target_batches
        .get(&target()?)
        .ok_or_else(|| io::Error::other("missing target batch"))?;
    assert_eq!(jobs.jobs.len(), 1);
    Ok(())
}

#[test]
fn shared_reservation_keeps_distinct_token_delivery_independent() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("shared-result.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);
    handle(&mut pending, &path, Some("first"))?;
    handle(&mut pending, &path, Some("second"))?;
    finish_frame(&mut pending);

    let completion = ready_completion(&pending, &path, b"complete")?;
    pending.begin_frame(vec![completion]);
    handle(&mut pending, &path, Some("first"))?;
    handle(&mut pending, &path, Some("second"))?;
    finish_frame(&mut pending);
    begin_frame(&mut pending);
    handle(&mut pending, &path, Some("first"))?;
    handle(&mut pending, &path, Some("second"))?;
    finish_frame(&mut pending);

    begin_frame(&mut pending);
    let first = handle(&mut pending, &path, Some("first"))?;
    let second_identity = identity(&path, Some("second"))?;
    assert_eq!(
        pending
            .captures
            .get(&second_identity)
            .ok_or_else(|| io::Error::other("missing second capture"))?
            .delivery_status,
        DeliveryStatus::Awaiting
    );
    let second = handle(&mut pending, &path, Some("second"))?;
    assert_eq!(first.response, second.response);
    Ok(())
}

#[test]
fn delivery_frame_shares_terminal_result_then_tombstones_until_unseen() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("terminal.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);
    handle(&mut pending, &path, Some("terminal"))?;
    finish_frame(&mut pending);
    progress_to_publication(&mut pending, &path, Some("terminal"), b"complete")?;

    begin_frame(&mut pending);
    let first = handle(&mut pending, &path, Some("terminal"))?;
    let concurrent = handle(&mut pending, &path, Some("terminal"))?;
    assert!(first.response.is_some());
    assert_eq!(first.response, concurrent.response);
    finish_frame(&mut pending);
    assert_eq!(pending.captures.len(), 1);

    begin_frame(&mut pending);
    let tombstone = handle(&mut pending, &path, Some("terminal"))?;
    assert!(tombstone.response.is_none());
    assert!(tombstone.spawn_target.is_none());
    finish_frame(&mut pending);
    assert_eq!(pending.captures.len(), 1);

    begin_frame(&mut pending);
    finish_frame(&mut pending);
    assert!(pending.captures.is_empty());
    assert!(pending.reservations.is_empty());
    Ok(())
}

#[test]
fn legacy_tombstone_cleanup_allows_a_later_generation() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("legacy.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);
    handle(&mut pending, &path, None)?;
    handle(&mut pending, &path, None)?;
    let first_generation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing first reservation"))?
        .generation;
    finish_frame(&mut pending);
    progress_to_publication(&mut pending, &path, None, b"first")?;

    begin_frame(&mut pending);
    assert!(handle(&mut pending, &path, None)?.response.is_some());
    finish_frame(&mut pending);
    begin_frame(&mut pending);
    assert!(handle(&mut pending, &path, None)?.response.is_none());
    finish_frame(&mut pending);
    begin_frame(&mut pending);
    finish_frame(&mut pending);

    begin_frame(&mut pending);
    let fresh = handle(&mut pending, &path, None)?;
    let second_generation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing second reservation"))?
        .generation;
    assert!(fresh.spawn_target.is_some());
    assert!(second_generation.0 > first_generation.0);
    Ok(())
}

#[test]
fn token_reuse_with_a_different_fingerprint_is_invalid() -> TestResult {
    let temp_dir = TempDir::new()?;
    let first = temp_dir.path().join("first.png");
    let second = temp_dir.path().join("second.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);

    handle(&mut pending, &first, Some("token"))?;
    let error = pending.handle(request(&second, Some("token"))?, capture_input()?);

    assert!(matches!(error, Err(error) if error.code == INVALID_PARAMS));
    Ok(())
}

#[test]
fn active_destination_with_a_different_fingerprint_is_rejected() -> TestResult {
    let temp_dir = TempDir::new()?;
    let destination = temp_dir.path().join("destination.png");
    let other = temp_dir.path().join("other.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);
    handle(&mut pending, &destination, Some("first"))?;

    let (_, fingerprint, _, _) = request(&other, Some("other"))?.into_parts();
    let conflicting_request = request(&destination, Some("second"))?.with_fingerprint(fingerprint);
    let error = pending.handle(conflicting_request, capture_input()?);

    assert!(matches!(error, Err(error) if error.code == INVALID_PARAMS));
    assert_eq!(pending.captures.len(), 1);
    assert_eq!(pending.reservations.len(), 1);
    Ok(())
}

#[test]
fn unseen_ready_capture_is_abandoned_before_publication() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("disconnected.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);
    handle(&mut pending, &path, Some("disconnect"))?;
    finish_frame(&mut pending);

    let completion = ready_completion(&pending, &path, b"never published")?;
    pending.begin_frame(vec![completion]);
    finish_frame(&mut pending);

    assert!(!path.exists());
    assert!(pending.captures.is_empty());
    assert!(pending.reservations.is_empty());
    Ok(())
}

#[test]
fn existing_requests_remain_readable_without_a_primary_window() -> TestResult {
    let temp_dir = TempDir::new()?;
    let pending_path = temp_dir.path().join("pending.png");
    let completed_path = temp_dir.path().join("completed.png");
    let mut app = App::new();
    app.add_plugins((RemotePlugin::default(), ScreenshotPlugin));
    let primary_window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.update();
    let handler_id = app.world_mut().register_system(screenshot::handler);

    let initial = app
        .world_mut()
        .run_system_with(
            handler_id,
            Some(json!({ "capture_id": "pending", "path": pending_path })),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    assert!(brp(initial)?.is_none());
    app.world_mut().despawn(primary_window);

    let pending_read = app
        .world_mut()
        .run_system_with(
            handler_id,
            Some(json!({ "capture_id": "pending", "path": pending_path })),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    assert!(brp(pending_read)?.is_none());

    {
        let mut captures = app.world_mut().resource_mut::<PendingScreenshotCaptures>();
        begin_frame(&mut captures);
        handle(&mut captures, &completed_path, Some("completed"))?;
        finish_frame(&mut captures);
        progress_to_publication(
            &mut captures,
            &completed_path,
            Some("completed"),
            b"complete",
        )?;
    }
    let completed_read = app
        .world_mut()
        .run_system_with(
            handler_id,
            Some(json!({
                "capture_id": "completed",
                "path": completed_path,
            })),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    assert!(brp(completed_read)?.is_some());

    let new_request = app
        .world_mut()
        .run_system_with(
            handler_id,
            Some(json!({ "capture_id": "new", "path": temp_dir.path().join("new.png") })),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    assert!(matches!(new_request, Err(error) if error.message.contains("primary window")));
    Ok(())
}

#[test]
fn handler_spawns_one_screenshot_entity_and_observer_takes_batched_jobs() -> TestResult {
    let temp_dir = TempDir::new()?;
    let first_path = temp_dir.path().join("first.png");
    let second_path = temp_dir.path().join("second.png");
    CONVERSIONS.store(0, Ordering::SeqCst);

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, RemotePlugin::default(), ScreenshotPlugin));
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    app.world_mut()
        .resource_mut::<CaptureCompletionChannel>()
        .converter = counting_conversion;
    let handler_id = app.world_mut().register_system(screenshot::handler);

    let first = app
        .world_mut()
        .run_system_with(
            handler_id,
            Some(json!({ "capture_id": "first", "path": first_path })),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    brp(first)?;
    let second = app
        .world_mut()
        .run_system_with(
            handler_id,
            Some(json!({ "capture_id": "second", "path": second_path })),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    brp(second)?;

    let mut screenshots = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
    let mut screenshot_entities = screenshots.iter(app.world());
    let screenshot_entity = screenshot_entities
        .next()
        .ok_or_else(|| io::Error::other("missing screenshot entity"))?;
    assert!(screenshot_entities.next().is_none());
    app.world_mut()
        .entity_mut(screenshot_entity)
        .trigger(|entity| ScreenshotCaptured {
            entity,
            image: test_image(),
        });

    let channel = app.world().resource::<CaptureCompletionChannel>();
    let receiver = channel
        .receiver
        .lock()
        .map_err(|_| io::Error::other("completion channel mutex poisoned"))?;
    let first_completion = receiver.recv_timeout(WORKER_TEST_TIMEOUT)?;
    let second_completion = receiver.recv_timeout(WORKER_TEST_TIMEOUT)?;
    drop(receiver);
    assert!(first_completion.result.is_ok());
    assert!(second_completion.result.is_ok());
    assert_eq!(CONVERSIONS.load(Ordering::SeqCst), 1);
    assert!(
        app.world()
            .resource::<PendingScreenshotCaptures>()
            .target_batches
            .is_empty()
    );
    Ok(())
}

fn expire_removed_target_capture(
    app: &mut App,
    image_handle: &Handle<Image>,
    screenshot_entity: Entity,
    handler_id: SystemId<In<Option<Value>>, BrpResult<Option<Value>>>,
    params: &Value,
    deadline: Instant,
) -> TestResult {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .remove(image_handle);

    app.world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .begin_frame_at(Vec::new(), deadline);
    assert!(brp(run_handler(app.world_mut(), handler_id, params))?.is_none());
    let expired_screenshot_entities = app
        .world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .advance(deadline);
    assert_eq!(expired_screenshot_entities, vec![screenshot_entity]);
    for expired_screenshot_entity in expired_screenshot_entities {
        app.world_mut().try_despawn(expired_screenshot_entity)?;
    }
    app.world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .cleanup();
    Ok(())
}

fn acknowledge_expired_capture(
    app: &mut App,
    path: &Path,
    screenshot_entity: Entity,
    handler_id: SystemId<In<Option<Value>>, BrpResult<Option<Value>>>,
    params: &Value,
    deadline: Instant,
) -> TestResult {
    assert!(app.world().get_entity(screenshot_entity).is_err());
    {
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        assert!(pending.target_batches.is_empty());
        assert!(matches!(
            pending
                .reservations
                .get(path)
                .ok_or_else(|| io::Error::other("missing acknowledged reservation"))?
                .work,
            ReservationWork::Suppressed | ReservationWork::Failed(_)
        ));
    }
    assert!(matches!(
        run_handler(app.world_mut(), handler_id, params),
        Err(error) if error.message.contains("deadline")
    ));

    app.world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .begin_frame_at(Vec::new(), deadline);
    assert!(
        app.world_mut()
            .resource_mut::<PendingScreenshotCaptures>()
            .advance(deadline)
            .is_empty()
    );
    app.world_mut()
        .resource_mut::<PendingScreenshotCaptures>()
        .cleanup();
    assert!(
        app.world()
            .resource::<PendingScreenshotCaptures>()
            .reservations
            .is_empty()
    );
    Ok(())
}

fn retry_removed_target_capture(
    app: &mut App,
    path: &Path,
    image_handle: &Handle<Image>,
    camera: Entity,
    entity: Entity,
    handler_id: SystemId<In<Option<Value>>, BrpResult<Option<Value>>>,
    first_generation: PathGeneration,
) -> TestResult {
    let target_size = app
        .world()
        .get::<Camera>(camera)
        .and_then(Camera::physical_target_size)
        .ok_or_else(|| io::Error::other("missing camera target size"))?;
    let mut replacement_image = image_with_size(target_size);
    replacement_image.asset_usage = RenderAssetUsages::default();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(image_handle.id(), replacement_image)?;
    let retry_params = json!({
        "camera": camera.to_bits(),
        "capture_id": "retry",
        "entity": entity.to_bits(),
        "path": path,
    });
    assert!(brp(run_handler(app.world_mut(), handler_id, &retry_params))?.is_none());
    let retry_generation = app
        .world()
        .resource::<PendingScreenshotCaptures>()
        .reservations
        .get(path)
        .ok_or_else(|| io::Error::other("missing retry reservation"))?
        .generation;
    assert!(retry_generation.0 > first_generation.0);
    let screenshot_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(screenshot_entity)
        .trigger(|entity| ScreenshotCaptured {
            entity,
            image: image_with_size(target_size),
        });
    let completions = receive_worker_completions(app, 1)?;
    begin_app_frame(app, completions);
    assert!(brp(run_handler(app.world_mut(), handler_id, &retry_params))?.is_none());
    finish_app_frame(app);
    begin_app_frame(app, Vec::new());
    assert!(brp(run_handler(app.world_mut(), handler_id, &retry_params))?.is_none());
    finish_app_frame(app);
    begin_app_frame(app, Vec::new());
    let response = brp(run_handler(app.world_mut(), handler_id, &retry_params))?
        .ok_or_else(|| io::Error::other("missing retry terminal response"))?;

    assert_eq!(
        response.get(crate::constants::RESPONSE_SUCCESS_FIELD),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        response
            .get(crate::constants::RESPONSE_STATUS_FIELD)
            .and_then(Value::as_str),
        Some(crate::constants::SCREENSHOT_STATUS_COMPLETED)
    );
    assert!(path.exists());
    assert!(image::open(path)?.dimensions().0 > 0);
    Ok(())
}

#[test]
fn removed_target_timeout_despawns_capture_and_releases_path_for_retry() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("removed-target.png");
    let target_size = UVec2::splat(100);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ScreenshotPlugin));
    app.init_resource::<Assets<Image>>();
    let mut target_image = image_with_size(target_size);
    target_image.asset_usage = RenderAssetUsages::default();
    let image_handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(target_image);
    let camera = spawn_camera(
        app.world_mut(),
        RenderTarget::Image(image_handle.clone().into()),
        target_size,
    );
    let entity = spawn_aabb_entity(app.world_mut(), "Removed target");
    let handler_id = app.world_mut().register_system(screenshot::handler);
    let params = json!({
        "camera": camera.to_bits(),
        "capture_id": "expired",
        "entity": entity.to_bits(),
        "path": path,
    });

    assert!(brp(run_handler(app.world_mut(), handler_id, &params))?.is_none());
    let screenshot_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    let (first_generation, deadline) = {
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        let reservation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing destination reservation"))?;
        (reservation.generation, reservation.worker_deadline)
    };

    expire_removed_target_capture(
        &mut app,
        &image_handle,
        screenshot_entity,
        handler_id,
        &params,
        deadline,
    )?;
    acknowledge_expired_capture(
        &mut app,
        &path,
        screenshot_entity,
        handler_id,
        &params,
        deadline,
    )?;
    retry_removed_target_capture(
        &mut app,
        &path,
        &image_handle,
        camera,
        entity,
        handler_id,
        first_generation,
    )
}

fn expected_entity_rect(
    app: &App,
    path: &Path,
    entity: Entity,
    camera: Entity,
) -> Result<URect, IoError> {
    let pending = app.world().resource::<PendingScreenshotCaptures>();
    let metadata = &pending
        .reservations
        .get(path)
        .ok_or_else(|| io::Error::other("missing entity reservation"))?
        .response_metadata;
    let CaptureResponseMetadata::Entity(metadata) = metadata else {
        return Err(io::Error::other("missing entity metadata"));
    };
    assert_eq!(metadata.entity, entity);
    assert_eq!(metadata.name.as_deref(), Some("Before"));
    assert_eq!(metadata.camera, camera);
    assert_eq!(metadata.bounds_kind, screenshot::BoundsKind::Aabb);
    Ok(metadata.rect)
}

fn complete_capture_after_source_entities_are_removed(
    app: &mut App,
    target_size: UVec2,
    entity: Entity,
    camera: Entity,
    handler_id: SystemId<In<Option<Value>>, BrpResult<Option<Value>>>,
    first_params: &Value,
    second_params: &Value,
) -> Result<(Value, Value), Box<dyn Error>> {
    app.world_mut().entity_mut(entity).insert((
        GlobalTransform::from(Transform::from_xyz(0.5, 0.5, 0.5)),
        Name::new("After"),
    ));
    assert!(brp(run_handler(app.world_mut(), handler_id, first_params))?.is_none());
    assert!(brp(run_handler(app.world_mut(), handler_id, second_params))?.is_none());
    app.world_mut().despawn(entity);
    app.world_mut().despawn(camera);

    let screenshot_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(screenshot_entity)
        .trigger(|entity| ScreenshotCaptured {
            entity,
            image: image_with_size(target_size),
        });
    let completions = receive_worker_completions(app, 1)?;
    begin_app_frame(app, completions);
    assert!(brp(run_handler(app.world_mut(), handler_id, first_params))?.is_none());
    assert!(brp(run_handler(app.world_mut(), handler_id, second_params))?.is_none());
    finish_app_frame(app);
    begin_app_frame(app, Vec::new());
    assert!(brp(run_handler(app.world_mut(), handler_id, first_params))?.is_none());
    assert!(brp(run_handler(app.world_mut(), handler_id, second_params))?.is_none());
    finish_app_frame(app);
    begin_app_frame(app, Vec::new());
    let first = brp(run_handler(app.world_mut(), handler_id, first_params))?
        .ok_or_else(|| io::Error::other("missing first terminal response"))?;
    let second = brp(run_handler(app.world_mut(), handler_id, second_params))?
        .ok_or_else(|| io::Error::other("missing second terminal response"))?;
    Ok((first, second))
}

fn assert_entity_snapshot_response(
    first: &Value,
    second: &Value,
    path: &Path,
    entity: Entity,
    camera: Entity,
    expected_rect: URect,
) {
    assert_eq!(first, second);
    assert_eq!(
        first[crate::constants::PARAM_ENTITY],
        json!(entity.to_bits())
    );
    assert_eq!(
        first[crate::constants::RESPONSE_NAME_FIELD],
        json!("Before")
    );
    assert_eq!(
        first[crate::constants::PARAM_CAMERA],
        json!(camera.to_bits())
    );
    assert_eq!(
        first[crate::constants::RESPONSE_BOUNDS_KIND_FIELD],
        json!(crate::constants::SCREENSHOT_BOUNDS_KIND_AABB)
    );
    assert_eq!(
        first[crate::constants::RESPONSE_RECT_FIELD],
        json!({
            crate::constants::RESPONSE_X_FIELD: expected_rect.min.x,
            crate::constants::RESPONSE_Y_FIELD: expected_rect.min.y,
            crate::constants::RESPONSE_WIDTH_FIELD: expected_rect.width(),
            crate::constants::RESPONSE_HEIGHT_FIELD: expected_rect.height(),
        })
    );
    assert!(path.exists());
}

#[test]
fn entity_terminal_lifecycle_keeps_the_original_metadata_snapshot() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("entity-snapshot.png");
    let target_size = UVec2::splat(100);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ScreenshotPlugin));
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let camera = spawn_camera(
        app.world_mut(),
        RenderTarget::Window(WindowRef::Primary),
        target_size,
    );
    let entity = spawn_aabb_entity(app.world_mut(), "Before");
    let handler_id = app.world_mut().register_system(screenshot::handler);
    let first_params = json!({
        "camera": camera.to_bits(),
        "capture_id": "first",
        "entity": entity.to_bits(),
        "path": path,
    });
    let second_params = json!({
        "camera": camera.to_bits(),
        "capture_id": "second",
        "entity": entity.to_bits(),
        "path": path,
    });

    assert!(brp(run_handler(app.world_mut(), handler_id, &first_params))?.is_none());
    let expected_rect = expected_entity_rect(&app, &path, entity, camera)?;
    let (first, second) = complete_capture_after_source_entities_are_removed(
        &mut app,
        target_size,
        entity,
        camera,
        handler_id,
        &first_params,
        &second_params,
    )?;
    assert_entity_snapshot_response(&first, &second, &path, entity, camera, expected_rect);
    Ok(())
}

#[test]
fn shared_full_and_entity_capture_converts_once_and_completes_both_outputs() -> TestResult {
    let temp_dir = TempDir::new()?;
    let full_path = temp_dir.path().join("full.png");
    let entity_path = temp_dir.path().join("entity.png");
    let target_size = UVec2::splat(100);
    CONVERSIONS.store(0, Ordering::SeqCst);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ScreenshotPlugin));
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let camera = spawn_camera(
        app.world_mut(),
        RenderTarget::Window(WindowRef::Primary),
        target_size,
    );
    let entity = spawn_aabb_entity(app.world_mut(), "Shared target");
    app.world_mut()
        .resource_mut::<CaptureCompletionChannel>()
        .converter = counting_conversion;
    let handler_id = app.world_mut().register_system(screenshot::handler);
    let full_params = json!({
        "capture_id": "full",
        "path": full_path,
    });
    let entity_params = json!({
        "camera": camera.to_bits(),
        "capture_id": "entity",
        "entity": entity.to_bits(),
        "path": entity_path,
    });

    assert!(brp(run_handler(app.world_mut(), handler_id, &full_params))?.is_none());
    assert!(brp(run_handler(app.world_mut(), handler_id, &entity_params))?.is_none());
    let entity_rect = {
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        let CaptureResponseMetadata::Entity(metadata) = &pending
            .reservations
            .get(&entity_path)
            .ok_or_else(|| io::Error::other("missing entity reservation"))?
            .response_metadata
        else {
            return Err(io::Error::other("missing entity metadata").into());
        };
        metadata.rect
    };
    let screenshot_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(screenshot_entity)
        .trigger(|entity| ScreenshotCaptured {
            entity,
            image: image_with_size(target_size),
        });
    let completions = receive_worker_completions(&app, 2)?;
    assert_eq!(CONVERSIONS.load(Ordering::SeqCst), 1);

    begin_app_frame(&mut app, completions);
    assert!(brp(run_handler(app.world_mut(), handler_id, &full_params))?.is_none());
    assert!(brp(run_handler(app.world_mut(), handler_id, &entity_params))?.is_none());
    finish_app_frame(&mut app);
    begin_app_frame(&mut app, Vec::new());
    assert!(brp(run_handler(app.world_mut(), handler_id, &full_params))?.is_none());
    assert!(brp(run_handler(app.world_mut(), handler_id, &entity_params))?.is_none());
    finish_app_frame(&mut app);
    begin_app_frame(&mut app, Vec::new());
    assert!(brp(run_handler(app.world_mut(), handler_id, &full_params))?.is_some());
    assert!(brp(run_handler(app.world_mut(), handler_id, &entity_params))?.is_some());

    assert_eq!(
        image::open(&full_path)?.dimensions(),
        (target_size.x, target_size.y)
    );
    assert_eq!(
        image::open(&entity_path)?.dimensions(),
        (entity_rect.width(), entity_rect.height())
    );
    Ok(())
}

#[test]
fn atomic_publication_replaces_existing_and_creates_absent_destinations() -> TestResult {
    let temp_dir = TempDir::new()?;
    let existing = temp_dir.path().join("replace.png");
    fs::write(&existing, b"sentinel")?;
    let replacement = brp(screenshot_job::create_temporary_file(
        &existing,
        b"complete png",
    ))?;
    replacement.persist(&existing)?;
    assert_eq!(fs::read(&existing)?, b"complete png");

    let absent = temp_dir.path().join("new.png");
    let created = brp(screenshot_job::create_temporary_file(
        &absent,
        b"complete png",
    ))?;
    created.persist(&absent)?;
    assert_eq!(fs::read(&absent)?, b"complete png");
    Ok(())
}

#[test]
fn persist_failure_preserves_existing_destination() -> TestResult {
    let temp_dir = TempDir::new()?;
    let destination = temp_dir.path().join("preserved.png");
    fs::write(&destination, b"sentinel")?;
    let temp_path = brp(screenshot_job::create_temporary_file(
        &destination,
        b"replacement",
    ))?;
    let temporary_file_path: &Path = temp_path.as_ref();
    fs::remove_file(temporary_file_path)?;

    assert!(temp_path.persist(&destination).is_err());
    assert_eq!(fs::read(&destination)?, b"sentinel");
    Ok(())
}

#[test]
fn new_token_cannot_join_an_expired_worker_generation() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("expired-generation.png");
    let mut pending = PendingScreenshotCaptures::default();
    let started = Instant::now();
    pending.begin_frame_at(Vec::new(), started);
    handle_at(&mut pending, &path, Some("owner"), started)?;
    let reservation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing destination reservation"))?;
    let generation = reservation.generation;
    let worker_deadline = reservation.worker_deadline;

    let rejected = pending.handle_at(
        request(&path, Some("late-watcher"))?,
        capture_input()?,
        worker_deadline,
    );

    assert!(matches!(
        rejected,
        Err(error) if error.message.contains("awaiting worker acknowledgement or cleanup")
    ));
    assert_eq!(pending.captures.len(), 1);
    assert!(
        !pending
            .captures
            .contains_key(&identity(&path, Some("late-watcher"))?)
    );
    let reservation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing destination reservation"))?;
    assert_eq!(reservation.generation, generation);
    assert_eq!(reservation.worker_deadline, worker_deadline);
    assert_eq!(reservation.watcher_ids.len(), 1);
    assert!(matches!(reservation.work, ReservationWork::Capturing));
    assert_eq!(pending.path_generations.get(&path), Some(&generation));
    assert_eq!(
        pending
            .target_batches
            .get(&target()?)
            .ok_or_else(|| io::Error::other("missing target batch"))?
            .jobs
            .len(),
        1
    );
    Ok(())
}

struct LateEncodingCompletion {
    completion:       WorkerCompletion,
    first_generation: PathGeneration,
    temporary_path:   PathBuf,
    worker_deadline:  Instant,
}

fn start_late_encoding_completion(
    pending: &mut PendingScreenshotCaptures,
    path: &Path,
    started: Instant,
) -> Result<LateEncodingCompletion, Box<dyn Error>> {
    handle_at(pending, path, Some("owner"), started)?;
    let reservation = pending
        .reservations
        .get(path)
        .ok_or_else(|| io::Error::other("missing destination reservation"))?;
    let first_generation = reservation.generation;
    let owner_identity = reservation.owner.clone();
    let worker_deadline = reservation.worker_deadline;
    handle_at(
        pending,
        path,
        Some("joined-before-expiration"),
        worker_deadline
            .checked_sub(Duration::from_millis(1))
            .ok_or_else(|| io::Error::other("worker deadline could not move earlier"))?,
    )?;
    assert!(
        pending
            .captures
            .values()
            .all(|record| record.deadline == worker_deadline)
    );

    let jobs = pending
        .take_target_batch(&target()?)
        .ok_or_else(|| io::Error::other("missing target batch"))?;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].path_generation, first_generation);
    assert_eq!(jobs[0].identity, owner_identity);
    assert!(matches!(
        pending
            .reservations
            .get(path)
            .ok_or_else(|| io::Error::other("missing encoding reservation"))?
            .work,
        ReservationWork::Encoding
    ));
    assert!(
        pending
            .captures
            .values()
            .all(|record| record.state == CaptureState::Encoding)
    );

    let completion = ready_completion(pending, path, b"late")?;
    let temporary_path = completion
        .result
        .as_ref()
        .map_err(|error| io::Error::other(error.message.clone()))?
        .temp_path
        .to_path_buf();
    Ok(LateEncodingCompletion {
        completion,
        first_generation,
        temporary_path,
        worker_deadline,
    })
}

fn expire_encoding_watchers(
    pending: &mut PendingScreenshotCaptures,
    path: &Path,
    first_generation: PathGeneration,
    worker_deadline: Instant,
) -> TestResult {
    pending.begin_frame_at(Vec::new(), worker_deadline);
    for capture_id in ["owner", "joined-before-expiration"] {
        assert!(
            brp(pending.handle_at(
                request(path, Some(capture_id))?,
                capture_input()?,
                worker_deadline,
            ))?
            .response
            .is_none()
        );
    }
    finish_frame_at(pending, worker_deadline);
    for capture_id in ["owner", "joined-before-expiration"] {
        assert!(matches!(
            pending.handle_at(
                request(path, Some(capture_id))?,
                capture_input()?,
                worker_deadline,
            ),
            Err(error) if error.message.contains("deadline")
        ));
    }

    assert!(matches!(
        pending
            .reservations
            .get(path)
            .ok_or_else(|| io::Error::other("missing expired encoding reservation"))?
            .work,
        ReservationWork::Encoding
    ));
    assert_eq!(pending.path_generations.get(path), Some(&first_generation));
    assert!(
        pending
            .captures
            .values()
            .all(|record| record.state == CaptureState::TimedOut)
    );
    assert!(matches!(
        pending.handle_at(
            request(path, Some("blocked-reuse"))?,
            capture_input()?,
            worker_deadline,
        ),
        Err(error) if error.message.contains("awaiting worker acknowledgement or cleanup")
    ));
    Ok(())
}

#[test]
fn late_encoding_completion_keeps_generation_owned_until_acknowledged() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("late-completion.png");
    let mut pending = PendingScreenshotCaptures::default();
    let started = Instant::now();
    pending.begin_frame_at(Vec::new(), started);
    let late = start_late_encoding_completion(&mut pending, &path, started)?;
    expire_encoding_watchers(
        &mut pending,
        &path,
        late.first_generation,
        late.worker_deadline,
    )?;
    assert!(late.temporary_path.exists());

    pending.begin_frame_at(vec![late.completion], late.worker_deadline);
    assert!(!late.temporary_path.exists());
    assert!(matches!(
        pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing acknowledged reservation"))?
            .work,
        ReservationWork::Failed(_)
    ));
    finish_frame_at(&mut pending, late.worker_deadline);

    assert!(!path.exists());
    assert!(pending.captures.is_empty());
    assert!(pending.reservations.is_empty());

    pending.begin_frame_at(Vec::new(), late.worker_deadline);
    let fresh = handle_at(&mut pending, &path, Some("fresh"), late.worker_deadline)?;
    let second_generation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing fresh reservation"))?
        .generation;
    assert!(fresh.spawn_target.is_some());
    assert!(second_generation.0 > late.first_generation.0);
    Ok(())
}

#[test]
fn acknowledged_expiration_cleanup_allows_a_greater_generation() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("expired-then-fresh.png");
    let mut pending = PendingScreenshotCaptures::default();
    let started = Instant::now();
    pending.begin_frame_at(Vec::new(), started);
    handle_at(&mut pending, &path, Some("expired"), started)?;
    let reservation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing destination reservation"))?;
    let first_generation = reservation.generation;
    let worker_deadline = reservation.worker_deadline;
    let completion = ready_completion(&pending, &path, b"late")?;

    pending.begin_frame_at(vec![completion], worker_deadline);
    assert!(
        pending
            .handle_at(
                request(&path, Some("expired"))?,
                capture_input()?,
                worker_deadline,
            )
            .is_err()
    );
    finish_frame_at(&mut pending, worker_deadline);
    pending.begin_frame_at(Vec::new(), worker_deadline);
    finish_frame_at(&mut pending, worker_deadline);
    assert!(pending.captures.is_empty());
    assert!(pending.reservations.is_empty());

    pending.begin_frame_at(Vec::new(), worker_deadline);
    let fresh = handle_at(&mut pending, &path, Some("fresh"), worker_deadline)?;
    let second_generation = pending
        .reservations
        .get(&path)
        .ok_or_else(|| io::Error::other("missing fresh reservation"))?
        .generation;
    assert!(fresh.spawn_target.is_some());
    assert!(second_generation.0 > first_generation.0);
    Ok(())
}

#[test]
fn stale_generation_cannot_replace_a_new_destination() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("generation.png");
    let mut pending = PendingScreenshotCaptures::default();
    begin_frame(&mut pending);
    handle(&mut pending, &path, Some("old"))?;
    let stale = ready_completion(&pending, &path, b"stale")?;
    let old_identity = identity(&path, Some("old"))?;
    pending
        .captures
        .get_mut(&old_identity)
        .ok_or_else(|| io::Error::other("missing old capture"))?
        .deadline = Instant::now();
    handle(&mut pending, &path, Some("old"))?;
    finish_frame(&mut pending);
    assert!(
        pending
            .handle(request(&path, Some("old"))?, capture_input()?)
            .is_err()
    );
    let acknowledgement = ready_completion(&pending, &path, b"discarded")?;
    pending.begin_frame(vec![acknowledgement]);
    finish_frame(&mut pending);
    begin_frame(&mut pending);
    finish_frame(&mut pending);

    begin_frame(&mut pending);
    handle(&mut pending, &path, Some("new"))?;
    fs::write(&path, b"current")?;
    pending.begin_frame(vec![stale]);
    handle(&mut pending, &path, Some("new"))?;
    finish_frame(&mut pending);

    assert_eq!(fs::read(path)?, b"current");
    Ok(())
}

#[test]
fn real_watcher_publishes_once_and_tombstone_waits_for_receiver_close() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("live-watcher.png");
    let mut app = remote_app();
    let response_receiver = send_remote_request(&app, &path, "live")?;
    app.update();

    let completion = {
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        ready_completion(pending, &path, b"complete png")?
    };
    app.world()
        .resource::<CaptureCompletionChannel>()
        .sender
        .send(completion)?;

    app.update();
    assert!(!path.exists());
    app.update();
    assert!(path.exists());
    app.update();
    let terminal = receive_terminal(&response_receiver)?;
    assert_eq!(
        terminal.get(crate::constants::RESPONSE_SUCCESS_FIELD),
        Some(&Value::Bool(true))
    );

    app.update();
    assert!(matches!(
        response_receiver.try_recv(),
        Err(TryRecvError::Empty)
    ));
    app.update();
    assert!(matches!(
        response_receiver.try_recv(),
        Err(TryRecvError::Empty)
    ));
    {
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        assert_eq!(pending.captures.len(), 1);
        assert_eq!(pending.reservations.len(), 1);
        assert!(pending.target_batches.is_empty());
    }
    let mut screenshots = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
    assert_eq!(screenshots.iter(app.world()).count(), 1);

    drop(response_receiver);
    app.update();
    assert_eq!(
        app.world()
            .resource::<PendingScreenshotCaptures>()
            .captures
            .len(),
        1
    );
    app.update();
    let pending = app.world().resource::<PendingScreenshotCaptures>();
    assert!(pending.captures.is_empty());
    assert!(pending.reservations.is_empty());
    Ok(())
}

#[test]
fn real_disconnected_watcher_suppresses_publication() -> TestResult {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("closed-watcher.png");
    let mut app = remote_app();
    let response_receiver = send_remote_request(&app, &path, "closed")?;
    app.update();

    let completion = {
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        ready_completion(pending, &path, b"must not publish")?
    };
    app.world()
        .resource::<CaptureCompletionChannel>()
        .sender
        .send(completion)?;
    drop(response_receiver);

    app.update();
    assert!(!path.exists());
    app.update();
    assert!(!path.exists());
    assert!(
        app.world()
            .resource::<PendingScreenshotCaptures>()
            .captures
            .is_empty()
    );
    Ok(())
}
