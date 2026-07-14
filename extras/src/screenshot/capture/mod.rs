//! Screenshot capture lifecycle and worker plugin.

mod pending_screenshot_capture;
mod screenshot_job;
mod target_rgb_image;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy_remote::BrpResult;
use bevy_remote::RemoteLast;
use bevy_remote::RemoteSystems;
pub(super) use pending_screenshot_capture::PendingScreenshotCapture;
use serde_json::Value;

use self::pending_screenshot_capture::advance_capture_lifecycle;
use self::pending_screenshot_capture::ingest_capture_completion;
use self::pending_screenshot_capture::screenshot_capture_active;
use super::CaptureResponseMetadata;
use super::request::ScreenshotRequest;

pub(super) struct CaptureInput {
    pub(super) crop:              Option<URect>,
    pub(super) render_target:     RenderTarget,
    pub(super) response_metadata: CaptureResponseMetadata,
}

pub(super) struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingScreenshotCapture>()
            .add_systems(
                RemoteLast,
                ingest_capture_completion
                    .run_if(screenshot_capture_active)
                    .before(RemoteSystems::ProcessRequests),
            )
            .add_systems(
                RemoteLast,
                advance_capture_lifecycle
                    .run_if(screenshot_capture_active)
                    .after(RemoteSystems::ProcessRequests)
                    .before(RemoteSystems::Cleanup),
            );
    }
}

pub(super) fn read(
    pending: &mut PendingScreenshotCapture,
    request: &ScreenshotRequest,
) -> Option<BrpResult<Option<Value>>> {
    pending.read(request)
}

pub(super) fn start(
    world: &mut World,
    request: ScreenshotRequest,
    capture_input: CaptureInput,
) -> BrpResult<()> {
    pending_screenshot_capture::start(world, request, capture_input)
}
