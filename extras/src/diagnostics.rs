//! FPS diagnostics handler for BRP extras

use std::time::Duration;

use bevy::diagnostic::Diagnostic;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use serde_json::Value;
use serde_json::json;

use crate::constants::DIAGNOSTICS_AVERAGE_FIELD;
use crate::constants::DIAGNOSTICS_CURRENT_FIELD;
use crate::constants::DIAGNOSTICS_FPS_FIELD;
use crate::constants::DIAGNOSTICS_FRAME_COUNT_FIELD;
use crate::constants::DIAGNOSTICS_FRAME_TIME_MS_FIELD;
use crate::constants::DIAGNOSTICS_HISTORY_DURATION_SECS_FIELD;
use crate::constants::DIAGNOSTICS_HISTORY_LEN_FIELD;
use crate::constants::DIAGNOSTICS_MAX_HISTORY_LEN_FIELD;
use crate::constants::DIAGNOSTICS_SMOOTHED_FIELD;

/// Handler for `get_diagnostics` requests
///
/// Returns FPS and frame time diagnostics from Bevy's `DiagnosticsStore`.
/// Requires `FrameTimeDiagnosticsPlugin` to be installed (done automatically
/// by `BrpExtrasPlugin` when the `diagnostics` feature is enabled).
pub(crate) fn handler(In(_): In<Option<Value>>, world: &mut World) -> BrpResult {
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

    let fps_value = fps.and_then(Diagnostic::value);
    let fps_avg = fps.and_then(Diagnostic::average);
    let fps_smoothed = fps.and_then(Diagnostic::smoothed);
    let fps_history_len = fps.map_or(0, Diagnostic::history_len);
    let fps_max_history = fps.map_or(0, Diagnostic::get_max_history_length);
    let fps_duration_secs = fps
        .and_then(Diagnostic::duration)
        .as_ref()
        .map(Duration::as_secs_f64);

    let frame_time_value = frame_time.and_then(Diagnostic::value);
    let frame_time_avg = frame_time.and_then(Diagnostic::average);
    let frame_time_smoothed = frame_time.and_then(Diagnostic::smoothed);

    let total_frames = frame_count.and_then(Diagnostic::value);

    Ok(json!({
        DIAGNOSTICS_FPS_FIELD: {
            DIAGNOSTICS_CURRENT_FIELD: fps_value,
            DIAGNOSTICS_AVERAGE_FIELD: fps_avg,
            DIAGNOSTICS_SMOOTHED_FIELD: fps_smoothed,
            DIAGNOSTICS_HISTORY_LEN_FIELD: fps_history_len,
            DIAGNOSTICS_MAX_HISTORY_LEN_FIELD: fps_max_history,
            DIAGNOSTICS_HISTORY_DURATION_SECS_FIELD: fps_duration_secs,
        },
        DIAGNOSTICS_FRAME_TIME_MS_FIELD: {
            DIAGNOSTICS_CURRENT_FIELD: frame_time_value,
            DIAGNOSTICS_AVERAGE_FIELD: frame_time_avg,
            DIAGNOSTICS_SMOOTHED_FIELD: frame_time_smoothed,
        },
        DIAGNOSTICS_FRAME_COUNT_FIELD: total_frames,
    }))
}
