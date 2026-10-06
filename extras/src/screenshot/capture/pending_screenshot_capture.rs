//! Single-request screenshot lifecycle and terminal PNG publication.

use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Instant;

use bevy::prelude::*;
use bevy::render::view::screenshot::Capturing;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use serde_json::Value;

use super::CaptureInput;
use super::screenshot_job;
use super::screenshot_job::CaptureCompletionChannel;
use super::screenshot_job::ImageConverter;
use super::screenshot_job::OwnedTempCapture;
use super::screenshot_job::ScreenshotJob;
use super::screenshot_job::WorkerCompletion;
use crate::constants::SCREENSHOT_CAPTURE_DEADLINE;
use crate::constants::SCREENSHOT_ENTITY_NAME;
use crate::screenshot;
use crate::screenshot::request::ScreenshotRequest;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FrameStamp(u64);

impl FrameStamp {
    const fn next(self) -> Self { Self(self.0.wrapping_add(1)) }
}

enum CaptureStatus {
    Capturing(ScreenshotJob),
    Encoding,
    Completed(Value),
    Failed(BrpError),
}

impl CaptureStatus {
    const fn is_terminal(&self) -> bool { matches!(self, Self::Completed(_) | Self::Failed(_)) }
}

struct ActiveCapture {
    deadline:          Instant,
    request:           ScreenshotRequest,
    screenshot_entity: Entity,
    seen_frame:        FrameStamp,
    status:            CaptureStatus,
}

/// The last delivered request. Bevy Remote gives a call no identity, so this record swallows
/// identical calls in the delivery frame and at most one in the next frame, which covers the
/// delivered call's own repeat before Bevy Remote drops it. After that an identical call is a new
/// request.
struct DeliveredCapture {
    frame:            FrameStamp,
    request:          ScreenshotRequest,
    repeat_swallowed: bool,
}

impl DeliveredCapture {
    fn swallows(&mut self, request: &ScreenshotRequest, current_frame: FrameStamp) -> bool {
        if &self.request != request {
            return false;
        }
        if current_frame == self.frame {
            return true;
        }
        if current_frame == self.frame.next() && !self.repeat_swallowed {
            self.repeat_swallowed = true;
            return true;
        }
        false
    }
}

pub(super) struct CaptureRead {
    pub(super) response:        BrpResult<Option<Value>>,
    pub(super) released_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub(in crate::screenshot) struct PendingScreenshotCapture {
    active:             Option<ActiveCapture>,
    completion_channel: Option<CaptureCompletionChannel>,
    current_frame:      FrameStamp,
    delivered:          Option<DeliveredCapture>,
}

impl PendingScreenshotCapture {
    fn read(&mut self, request: &ScreenshotRequest) -> Option<CaptureRead> {
        let current_frame = self.current_frame;
        if self
            .delivered
            .as_mut()
            .is_some_and(|delivered| delivered.swallows(request, current_frame))
        {
            return Some(CaptureRead {
                response:        Ok(None),
                released_entity: None,
            });
        }

        let active = self.active.as_mut()?;
        if &active.request != request {
            return Some(CaptureRead {
                response:        Err(capture_in_progress_error()),
                released_entity: None,
            });
        }
        active.seen_frame = current_frame;
        if !active.status.is_terminal() {
            return Some(CaptureRead {
                response:        Ok(None),
                released_entity: None,
            });
        }

        let active = self.active.take()?;
        self.completion_channel = None;
        self.delivered = Some(DeliveredCapture {
            frame:            current_frame,
            request:          active.request,
            repeat_swallowed: false,
        });
        let response = match active.status {
            CaptureStatus::Completed(response) => Ok(Some(response)),
            CaptureStatus::Failed(error) => Err(error),
            CaptureStatus::Capturing(_) | CaptureStatus::Encoding => Ok(None),
        };
        Some(CaptureRead {
            response,
            released_entity: Some(active.screenshot_entity),
        })
    }

    fn start(
        &mut self,
        request: ScreenshotRequest,
        capture_input: CaptureInput,
        screenshot_entity: Entity,
        now: Instant,
    ) -> BrpResult<()> {
        if self.active.is_some() {
            return Err(capture_in_progress_error());
        }

        let screenshot_job = ScreenshotJob {
            crop:              capture_input.crop,
            path:              request.path().to_path_buf(),
            response_metadata: capture_input.response_metadata,
        };
        self.active = Some(ActiveCapture {
            deadline: now + SCREENSHOT_CAPTURE_DEADLINE,
            request,
            screenshot_entity,
            seen_frame: self.current_frame,
            status: CaptureStatus::Capturing(screenshot_job),
        });
        Ok(())
    }

    fn begin_frame(&mut self) -> BrpResult<Option<WorkerCompletion>> {
        self.current_frame = self.current_frame.next();
        let Some(channel) = self.completion_channel.as_ref() else {
            return Ok(None);
        };
        let receiver = channel
            .receiver
            .lock()
            .map_err(|_| capture_error("Screenshot completion channel mutex is poisoned"))?;
        Ok(receiver.try_recv().ok())
    }

    fn begin_encoding(
        &mut self,
        screenshot_entity: Entity,
    ) -> Option<(ScreenshotJob, Sender<WorkerCompletion>, ImageConverter)> {
        let active = self.active.as_mut()?;
        if active.screenshot_entity != screenshot_entity {
            return None;
        }
        let CaptureStatus::Capturing(_) = active.status else {
            return None;
        };
        let screenshot_job = match std::mem::replace(&mut active.status, CaptureStatus::Encoding) {
            CaptureStatus::Capturing(screenshot_job) => screenshot_job,
            status => {
                active.status = status;
                return None;
            },
        };
        let channel = self.completion_channel.get_or_insert_default();
        Some((screenshot_job, channel.sender.clone(), channel.converter))
    }

    fn complete(&mut self, completion: WorkerCompletion, now: Instant) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        if !matches!(active.status, CaptureStatus::Encoding) {
            return;
        }
        if now >= active.deadline {
            drop(completion);
            active.status = CaptureStatus::Failed(timeout_error());
            return;
        }

        active.status = match completion.result {
            Ok(capture) => publish_capture(active.request.path(), capture),
            Err(error) => CaptureStatus::Failed(error),
        };
    }

    fn fail_completion_channel(&mut self, error: BrpError) {
        if let Some(active) = self.active.as_mut()
            && matches!(active.status, CaptureStatus::Encoding)
        {
            active.status = CaptureStatus::Failed(error);
        }
    }

    fn advance(&mut self, now: Instant) -> Option<Entity> {
        if self
            .delivered
            .as_ref()
            .is_some_and(|delivered| delivered.frame != self.current_frame)
        {
            self.delivered = None;
        }

        let active = self.active.as_mut()?;
        if active.seen_frame != self.current_frame {
            let screenshot_entity = active.screenshot_entity;
            self.active = None;
            self.completion_channel = None;
            return Some(screenshot_entity);
        }
        if !active.status.is_terminal() && now >= active.deadline {
            active.status = CaptureStatus::Failed(timeout_error());
        }
        None
    }

    const fn is_active(&self) -> bool { self.active.is_some() || self.delivered.is_some() }
}

pub(super) fn read(
    world: &mut World,
    request: &ScreenshotRequest,
) -> Option<BrpResult<Option<Value>>> {
    let capture_read = world
        .resource_mut::<PendingScreenshotCapture>()
        .read(request)?;
    if let Some(screenshot_entity) = capture_read.released_entity {
        release_screenshot_entity(world, screenshot_entity);
    }
    Some(capture_read.response)
}

pub(super) fn start(
    world: &mut World,
    request: ScreenshotRequest,
    capture_input: CaptureInput,
) -> BrpResult<()> {
    let render_target = capture_input.render_target.clone();
    let screenshot_entity = world
        .spawn((Screenshot(render_target), Name::new(SCREENSHOT_ENTITY_NAME)))
        .observe(on_screenshot_captured)
        .id();
    let result = world.resource_mut::<PendingScreenshotCapture>().start(
        request,
        capture_input,
        screenshot_entity,
        Instant::now(),
    );
    if result.is_err() {
        world.entity_mut(screenshot_entity).despawn();
    }
    result
}

fn on_screenshot_captured(
    screenshot_captured: On<ScreenshotCaptured>,
    mut pending: ResMut<PendingScreenshotCapture>,
) {
    let Some((screenshot_job, sender, converter)) =
        pending.begin_encoding(screenshot_captured.event().entity)
    else {
        return;
    };
    screenshot_job::start_capture_worker(
        screenshot_captured.event().image.clone(),
        screenshot_job,
        sender,
        converter,
    );
}

pub(super) fn ingest_capture_completion(mut pending: ResMut<PendingScreenshotCapture>) {
    match pending.begin_frame() {
        Ok(Some(completion)) => pending.complete(completion, Instant::now()),
        Ok(None) => {},
        Err(error) => pending.fail_completion_channel(error),
    }
}

pub(super) fn screenshot_capture_active(pending: Res<PendingScreenshotCapture>) -> bool {
    pending.is_active()
}

pub(super) fn advance_capture_lifecycle(world: &mut World) {
    let abandoned_entity = world
        .resource_mut::<PendingScreenshotCapture>()
        .advance(Instant::now());
    if let Some(screenshot_entity) = abandoned_entity {
        release_screenshot_entity(world, screenshot_entity);
    }
}

/// Despawns a finished or abandoned screenshot entity unless Bevy has already extracted it. Bevy
/// owns a `Capturing` entity: it inserts `Captured` on it and despawns it itself, so despawning it
/// here would make that insert panic.
fn release_screenshot_entity(world: &mut World, screenshot_entity: Entity) {
    if let Ok(entity) = world.get_entity_mut(screenshot_entity)
        && !entity.contains::<Capturing>()
    {
        entity.despawn();
    }
}

fn publish_capture(path: &Path, capture: OwnedTempCapture) -> CaptureStatus {
    if !capture.metadata.dimensions.cmpgt(UVec2::ZERO).all() {
        return CaptureStatus::Failed(capture_error("Screenshot worker produced an empty image"));
    }
    let response_metadata = capture.metadata.response_metadata;
    match capture.temp_path.persist(path) {
        Ok(()) => {
            CaptureStatus::Completed(screenshot::completed_response(path, &response_metadata))
        },
        Err(error) => {
            let message = format!(
                "Failed to publish screenshot to {}: {}",
                path.display(),
                error.error
            );
            drop(error.path);
            CaptureStatus::Failed(capture_error(message))
        },
    }
}

fn capture_error(message: impl Into<String>) -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: message.into(),
        data:    None,
    }
}

fn capture_in_progress_error() -> BrpError {
    capture_error("A screenshot capture is already in progress")
}

fn timeout_error() -> BrpError {
    capture_error(format!(
        "Screenshot capture exceeded the {}-second server deadline",
        SCREENSHOT_CAPTURE_DEADLINE.as_secs()
    ))
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::io;

    use bevy::MinimalPlugins;
    use bevy::render::view::screenshot::Captured;
    use bevy_remote::RemotePlugin;
    use screenshot::CaptureResponseMetadata;
    use screenshot_job::CaptureMetadata;
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::screenshot::ScreenshotPlugin;

    const ABSENT_DESTINATION_NAME: &str = "new.png";
    const EXISTING_DESTINATION_NAME: &str = "replace.png";
    const IDLE_UPDATE_COUNT: usize = 3;
    const INITIAL_DESTINATION_CONTENT: &[u8] = b"sentinel";
    const SCREENSHOT_CONTENT: &[u8] = b"complete png";

    fn completed_capture(destination: &Path) -> Result<OwnedTempCapture, io::Error> {
        let temp_path = screenshot_job::create_temporary_file(destination, SCREENSHOT_CONTENT)
            .map_err(|error| io::Error::other(error.message))?;
        Ok(OwnedTempCapture {
            metadata: CaptureMetadata {
                dimensions:        UVec2::ONE,
                response_metadata: CaptureResponseMetadata::Full,
            },
            temp_path,
        })
    }

    fn request(path: &str) -> Result<ScreenshotRequest, io::Error> {
        ScreenshotRequest::from_params(Some(json!({ "path": path })))
            .map_err(|error| io::Error::other(error.message))
    }

    fn full_capture_input() -> CaptureInput {
        CaptureInput {
            crop:              None,
            render_target:     Screenshot::primary_window().0,
            response_metadata: CaptureResponseMetadata::Full,
        }
    }

    fn start_capture(world: &mut World, request: ScreenshotRequest) -> Result<Entity, io::Error> {
        start(world, request, full_capture_input())
            .map_err(|error| io::Error::other(error.message))?;
        world
            .resource::<PendingScreenshotCapture>()
            .active
            .as_ref()
            .map(|active| active.screenshot_entity)
            .ok_or_else(|| io::Error::other("capture did not start"))
    }

    fn capture_world() -> World {
        let mut world = World::new();
        world.init_resource::<PendingScreenshotCapture>();
        world
    }

    fn terminal_capture(
        request: &ScreenshotRequest,
        status: CaptureStatus,
    ) -> Result<PendingScreenshotCapture, io::Error> {
        let mut pending = PendingScreenshotCapture::default();
        pending
            .start(
                request.clone(),
                full_capture_input(),
                Entity::PLACEHOLDER,
                Instant::now(),
            )
            .map_err(|error| io::Error::other(error.message))?;
        pending
            .active
            .as_mut()
            .ok_or_else(|| io::Error::other("no active capture"))?
            .status = status;
        Ok(pending)
    }

    fn begin_frame(world: &mut World) -> Result<(), io::Error> {
        world
            .resource_mut::<PendingScreenshotCapture>()
            .begin_frame()
            .map(drop)
            .map_err(|error| io::Error::other(error.message))
    }

    /// Asserts the release rule, then plays Bevy's part on a kept entity: inserting `Captured`
    /// panics if extras despawned an entity Bevy still owns.
    fn assert_released_unless_extracted(
        world: &mut World,
        screenshot_entity: Entity,
        extracted: bool,
    ) {
        assert_eq!(world.get_entity(screenshot_entity).is_ok(), extracted);
        if extracted {
            world.entity_mut(screenshot_entity).insert(Captured);
        }
    }

    #[test]
    fn delivered_record_swallows_the_delivery_frame_and_one_repeat() -> Result<(), Box<dyn Error>> {
        let shot = request("shot.png")?;
        let mut pending = terminal_capture(&shot, CaptureStatus::Completed(json!({})))?;

        let delivery = pending
            .read(&shot)
            .ok_or_else(|| io::Error::other("terminal capture was not read"))?;
        assert!(matches!(delivery.response, Ok(Some(_))));
        assert_eq!(delivery.released_entity, Some(Entity::PLACEHOLDER));
        assert!(pending.active.is_none());

        for _ in 0..IDLE_UPDATE_COUNT {
            let repeat = pending.read(&shot);
            assert!(matches!(
                repeat,
                Some(CaptureRead {
                    response:        Ok(None),
                    released_entity: None,
                })
            ));
        }
        assert!(pending.read(&request("other.png")?).is_none());
        assert!(pending.advance(Instant::now()).is_none());
        assert!(pending.is_active());

        pending
            .begin_frame()
            .map_err(|error| io::Error::other(error.message))?;
        assert!(matches!(
            pending.read(&shot),
            Some(CaptureRead {
                response:        Ok(None),
                released_entity: None,
            })
        ));
        assert!(pending.read(&shot).is_none());
        assert!(pending.advance(Instant::now()).is_none());
        assert!(!pending.is_active());
        Ok(())
    }

    #[test]
    fn identical_call_after_the_repeat_frame_is_a_new_request() -> Result<(), Box<dyn Error>> {
        let shot = request("shot.png")?;
        let mut pending = terminal_capture(&shot, CaptureStatus::Failed(timeout_error()))?;
        assert!(matches!(
            pending.read(&shot),
            Some(CaptureRead {
                response: Err(_),
                ..
            })
        ));

        for _ in 0..2 {
            pending
                .begin_frame()
                .map_err(|error| io::Error::other(error.message))?;
            assert!(pending.advance(Instant::now()).is_none());
        }
        assert!(pending.read(&shot).is_none());
        Ok(())
    }

    #[test]
    fn abandoned_capture_releases_only_unextracted_entities() -> Result<(), Box<dyn Error>> {
        for extracted in [false, true] {
            let mut world = capture_world();
            let screenshot_entity = start_capture(&mut world, request("abandoned.png")?)?;
            if extracted {
                world.entity_mut(screenshot_entity).insert(Capturing);
            }

            begin_frame(&mut world)?;
            advance_capture_lifecycle(&mut world);

            assert!(!world.resource::<PendingScreenshotCapture>().is_active());
            assert_released_unless_extracted(&mut world, screenshot_entity, extracted);
        }
        Ok(())
    }

    #[test]
    fn timed_out_capture_releases_only_unextracted_entities_on_delivery()
    -> Result<(), Box<dyn Error>> {
        for extracted in [false, true] {
            let mut world = capture_world();
            let shot = request("timeout.png")?;
            let screenshot_entity = start_capture(&mut world, shot.clone())?;
            if extracted {
                world.entity_mut(screenshot_entity).insert(Capturing);
            }
            world
                .resource_mut::<PendingScreenshotCapture>()
                .active
                .as_mut()
                .ok_or_else(|| io::Error::other("no active capture"))?
                .deadline = Instant::now();

            advance_capture_lifecycle(&mut world);
            assert!(world.get_entity(screenshot_entity).is_ok());

            let response = read(&mut world, &shot)
                .ok_or_else(|| io::Error::other("timed-out capture was not read"))?;
            assert!(matches!(
                response,
                Err(error) if error.message.contains("server deadline")
            ));
            assert_released_unless_extracted(&mut world, screenshot_entity, extracted);
        }
        Ok(())
    }

    #[test]
    fn idle_capture_plugin_keeps_lifecycle_dormant() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RemotePlugin::default(), ScreenshotPlugin));

        for _ in 0..IDLE_UPDATE_COUNT {
            app.update();
        }

        let pending = app.world().resource::<PendingScreenshotCapture>();
        assert_eq!(pending.current_frame, FrameStamp::default());
        assert!(!pending.is_active());
        assert!(pending.completion_channel.is_none());
    }

    #[test]
    fn atomic_publication_replaces_existing_and_creates_absent_destinations()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let existing = temp_dir.path().join(EXISTING_DESTINATION_NAME);
        fs::write(&existing, INITIAL_DESTINATION_CONTENT)?;

        let replacement = completed_capture(&existing)?;
        assert!(matches!(
            publish_capture(&existing, replacement),
            CaptureStatus::Completed(_)
        ));
        assert_eq!(fs::read(&existing)?, SCREENSHOT_CONTENT);

        let absent = temp_dir.path().join(ABSENT_DESTINATION_NAME);
        let created = completed_capture(&absent)?;
        assert!(matches!(
            publish_capture(&absent, created),
            CaptureStatus::Completed(_)
        ));
        assert_eq!(fs::read(&absent)?, SCREENSHOT_CONTENT);
        Ok(())
    }
}
