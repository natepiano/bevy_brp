//! Screenshot capture lifecycle and worker plugin.

mod identity;
mod pending_screenshot_captures;
mod screenshot_job;
mod target_rgb_image;

use bevy::camera::NormalizedRenderTarget;
use bevy::prelude::*;
use bevy_remote::BrpResult;
use bevy_remote::RemoteLast;
use bevy_remote::RemoteSystems;
pub(super) use identity::CaptureIdentity;
pub(super) use identity::CaptureToken;
pub(super) use identity::RequestFingerprint;
pub(super) use pending_screenshot_captures::PendingScreenshotCaptures;
use serde_json::Value;

use self::pending_screenshot_captures::advance_capture_lifecycle;
use self::pending_screenshot_captures::cleanup_capture_lifecycle;
use self::pending_screenshot_captures::ingest_capture_completions;
use self::screenshot_job::CaptureCompletionChannel;
use super::request::ScreenshotRequest;

pub(super) struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingScreenshotCaptures>()
            .init_resource::<CaptureCompletionChannel>()
            .add_systems(
                RemoteLast,
                ingest_capture_completions.before(RemoteSystems::ProcessRequests),
            )
            .add_systems(
                RemoteLast,
                (advance_capture_lifecycle, cleanup_capture_lifecycle)
                    .chain()
                    .after(RemoteSystems::ProcessRequests)
                    .before(RemoteSystems::Cleanup),
            );
    }
}

pub(super) fn read_existing(
    pending: &mut PendingScreenshotCaptures,
    request: &ScreenshotRequest,
) -> Option<BrpResult<Option<Value>>> {
    pending
        .read_existing(request)
        .map(|result| result.map(|dispatch| dispatch.response))
}

pub(super) fn handle(
    pending: &mut PendingScreenshotCaptures,
    request: ScreenshotRequest,
    target: NormalizedRenderTarget,
) -> BrpResult<(Option<Value>, Option<NormalizedRenderTarget>)> {
    pending
        .handle(request, target)
        .map(|dispatch| (dispatch.response, dispatch.spawn_target))
}

pub(super) fn spawn_primary_window_batch(world: &mut World, target: NormalizedRenderTarget) {
    pending_screenshot_captures::spawn_primary_window_batch(world, target);
}
