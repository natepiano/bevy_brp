//! Single-request screenshot lifecycle and terminal PNG publication.

use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Instant;

use bevy::prelude::*;
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
    delivered_frame:   Option<FrameStamp>,
    request:           ScreenshotRequest,
    screenshot_entity: Entity,
    seen_frame:        FrameStamp,
    status:            CaptureStatus,
}

impl ActiveCapture {
    fn read(&mut self, current_frame: FrameStamp) -> BrpResult<Option<Value>> {
        self.seen_frame = current_frame;
        if self.delivered_frame.is_some() {
            return Ok(None);
        }

        match &self.status {
            CaptureStatus::Completed(response) => {
                self.delivered_frame = Some(current_frame);
                Ok(Some(response.clone()))
            },
            CaptureStatus::Failed(error) => {
                self.delivered_frame = Some(current_frame);
                Err(error.clone())
            },
            CaptureStatus::Capturing(_) | CaptureStatus::Encoding => Ok(None),
        }
    }
}

#[derive(Resource, Default)]
pub struct PendingScreenshotCapture {
    active:             Option<ActiveCapture>,
    completion_channel: Option<CaptureCompletionChannel>,
    current_frame:      FrameStamp,
}

impl PendingScreenshotCapture {
    pub(super) fn read(&mut self, request: &ScreenshotRequest) -> Option<BrpResult<Option<Value>>> {
        let active = self.active.as_mut()?;
        if &active.request != request {
            return Some(Err(capture_in_progress_error()));
        }
        Some(active.read(self.current_frame))
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
            delivered_frame: None,
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
        let active = self.active.as_mut()?;
        if active.seen_frame != self.current_frame {
            let screenshot_entity = active.screenshot_entity;
            self.active = None;
            self.completion_channel = None;
            return Some(screenshot_entity);
        }
        if !active.status.is_terminal() && now >= active.deadline {
            active.status = CaptureStatus::Failed(timeout_error());
            return Some(active.screenshot_entity);
        }
        None
    }

    const fn is_active(&self) -> bool { self.active.is_some() }
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

pub(super) fn advance_capture_lifecycle(
    mut commands: Commands,
    mut pending: ResMut<PendingScreenshotCapture>,
) {
    if let Some(screenshot_entity) = pending.advance(Instant::now()) {
        commands.entity(screenshot_entity).try_despawn();
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
    use bevy::MinimalPlugins;
    use bevy_remote::RemotePlugin;

    use super::*;
    use crate::screenshot::ScreenshotPlugin;

    const IDLE_UPDATE_COUNT: usize = 3;

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
}
