//! Shutdown handler for BRP extras

use bevy::prelude::*;
use bevy::remote::BrpResult;
use serde_json::{Value, json};

/// Resource to track pending shutdown
#[derive(Resource)]
pub struct PendingShutdown {
    frames_remaining: u32,
}

/// Handler for shutdown requests
///
/// Schedules a graceful shutdown after a few frames to allow the response to be sent
#[allow(clippy::unnecessary_wraps)]
pub fn handler(In(_): In<Option<Value>>, world: &mut World) -> BrpResult {
    info!("BRP EXTRAS SHUTDOWN METHOD CALLED - scheduling deferred shutdown");
    info!("Call stack: {:?}", std::backtrace::Backtrace::capture());

    // Schedule shutdown for 10 frames from now (about 167ms at 60fps)
    world.insert_resource(PendingShutdown {
        frames_remaining: 10,
    });

    info!("Shutdown scheduled - will exit in 10 frames");

    Ok(json!({
        "success": true,
        "message": "Shutdown initiated - will exit in 10 frames",
        "pid": std::process::id()
    }))
}

/// System to handle deferred shutdown
pub fn deferred_shutdown_system(
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
