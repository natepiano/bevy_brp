//! Shutdown handler for BRP extras

use bevy::prelude::*;
use bevy_remote::BrpResult;
use serde_json::Value;
use serde_json::json;

/// Number of frames to defer shutdown to allow the response to be sent
const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;

/// Resource to track pending shutdown
#[derive(Resource)]
pub(crate) struct PendingShutdown {
    frames_remaining: u32,
}

/// Handler for shutdown requests
///
/// Schedules a graceful shutdown after a few frames to allow the response to be sent
#[allow(
    clippy::unnecessary_wraps,
    reason = "BRP handler signature requires BrpResult return type"
)]
pub(crate) fn handler(In(_): In<Option<Value>>, world: &mut World) -> BrpResult {
    info!("BRP EXTRAS SHUTDOWN METHOD CALLED - scheduling deferred shutdown");
    info!("Call stack: {:?}", std::backtrace::Backtrace::capture());

    // Schedule shutdown for a few frames from now to allow the response to be sent
    world.insert_resource(PendingShutdown {
        frames_remaining: DEFERRED_SHUTDOWN_FRAMES,
    });

    info!("Shutdown scheduled - will exit in {DEFERRED_SHUTDOWN_FRAMES} frames");

    Ok(json!({
        "success": true,
        "message": "Shutdown initiated - will exit in 10 frames",
        "pid": std::process::id()
    }))
}

/// System to handle deferred shutdown
pub(super) fn deferred_shutdown_system(
    pending: Option<ResMut<PendingShutdown>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    if let Some(mut shutdown) = pending {
        shutdown.frames_remaining = shutdown.frames_remaining.saturating_sub(1);

        if shutdown.frames_remaining == 0 {
            info!("Deferred shutdown triggered - sending AppExit::Success event");
            exit.write(bevy::app::AppExit::Success);
        }
    }
}
