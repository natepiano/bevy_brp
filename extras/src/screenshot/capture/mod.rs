//! Screenshot capture lifecycle and worker plugin.

mod identity;
mod pending_screenshot_captures;
mod screenshot_job;
mod target_rgb_image;

use std::path::PathBuf;

use bevy::camera::NormalizedRenderTarget;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy_remote::BrpResult;
use bevy_remote::RemoteLast;
use bevy_remote::RemoteSystems;
pub(super) use identity::CaptureIdentity;
pub(super) use identity::CaptureToken;
pub(super) use identity::RequestFingerprint;
use identity::RequestScopeFingerprint;
pub(super) use pending_screenshot_captures::PendingScreenshotCaptures;
use serde_json::Value;

use self::pending_screenshot_captures::advance_capture_lifecycle;
use self::pending_screenshot_captures::cleanup_capture_lifecycle;
use self::pending_screenshot_captures::ingest_capture_completions;
use self::screenshot_job::CaptureCompletionChannel;
use super::CaptureResponseMetadata;
use super::request::ScreenshotRequest;

pub(super) const fn request_fingerprint(
    path: PathBuf,
    camera: Option<u64>,
    entity: Option<u64>,
    padding: Option<u32>,
) -> RequestFingerprint {
    RequestFingerprint::new(
        path,
        RequestScopeFingerprint {
            camera,
            entity,
            padding,
        },
    )
}

pub(super) struct CaptureInput {
    pub(super) crop:              Option<URect>,
    pub(super) normalized_target: NormalizedRenderTarget,
    pub(super) render_target:     RenderTarget,
    pub(super) response_metadata: CaptureResponseMetadata,
}

pub(super) struct CaptureTarget {
    pub(super) normalized_target: NormalizedRenderTarget,
    pub(super) render_target:     RenderTarget,
}

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

pub(super) fn join_existing(
    pending: &mut PendingScreenshotCaptures,
    request: &ScreenshotRequest,
) -> Option<BrpResult<Option<Value>>> {
    pending
        .join_existing(request)
        .map(|result| result.map(|dispatch| dispatch.response))
}

pub(super) fn handle(
    pending: &mut PendingScreenshotCaptures,
    request: ScreenshotRequest,
    capture_input: CaptureInput,
) -> BrpResult<(Option<Value>, Option<CaptureTarget>)> {
    pending
        .handle(request, capture_input)
        .map(|dispatch| (dispatch.response, dispatch.spawn_target))
}

pub(super) fn spawn_target_batch(world: &mut World, target: CaptureTarget) {
    pending_screenshot_captures::spawn_target_batch(world, target);
}
