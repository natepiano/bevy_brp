//! FPS diagnostics handler for BRP extras

use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::remote::error_codes::INTERNAL_ERROR;
use bevy::remote::BrpError;
use bevy::remote::BrpResult;
use serde_json::json;
use serde_json::Value;

/// Handler for `get_diagnostics` requests
///
/// Returns FPS and frame time diagnostics from Bevy's `DiagnosticsStore`.
/// Requires `FrameTimeDiagnosticsPlugin` to be installed (done automatically
/// by `BrpExtrasPlugin` when the `diagnostics` feature is enabled).
#[allow(clippy::unnecessary_wraps)]
pub fn handler(In(_params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let Some(store) = world.get_resource::<DiagnosticsStore>() else {
        return Err(BrpError {
            code:    INTERNAL_ERROR,
            message: "DiagnosticsStore not found - FrameTimeDiagnosticsPlugin may not be installed"
                .to_string(),
            data:    None,
        });
    };

    let fps = store.get(&FrameTimeDiagnosticsPlugin::FPS);
    let frame_time = store.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME);
    let frame_count = store.get(&FrameTimeDiagnosticsPlugin::FRAME_COUNT);

    let fps_value = fps.and_then(bevy::diagnostic::Diagnostic::value);
    let fps_avg = fps.and_then(bevy::diagnostic::Diagnostic::average);
    let fps_smoothed = fps.and_then(bevy::diagnostic::Diagnostic::smoothed);
    let fps_history_len = fps.map_or(0, bevy::diagnostic::Diagnostic::history_len);
    let fps_max_history = fps.map_or(0, bevy::diagnostic::Diagnostic::get_max_history_length);
    let fps_duration_secs = fps
        .and_then(bevy::diagnostic::Diagnostic::duration)
        .map(|d| d.as_secs_f64());

    let frame_time_value = frame_time.and_then(bevy::diagnostic::Diagnostic::value);
    let frame_time_avg = frame_time.and_then(bevy::diagnostic::Diagnostic::average);
    let frame_time_smoothed = frame_time.and_then(bevy::diagnostic::Diagnostic::smoothed);

    let total_frames = frame_count.and_then(bevy::diagnostic::Diagnostic::value);

    Ok(json!({
        "fps": {
            "current": fps_value,
            "average": fps_avg,
            "smoothed": fps_smoothed,
            "history_len": fps_history_len,
            "max_history_len": fps_max_history,
            "history_duration_secs": fps_duration_secs,
        },
        "frame_time_ms": {
            "current": frame_time_value,
            "average": frame_time_avg,
            "smoothed": frame_time_smoothed,
        },
        "frame_count": total_frames,
    }))
}
