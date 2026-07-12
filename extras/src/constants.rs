//! Crate-level constants for `bevy_brp_extras`

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

// command constants
/// Command prefix for `brp_extras` methods
pub(crate) const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";
pub(crate) const METHOD_CLICK_MOUSE: &str = "click_mouse";
pub(crate) const METHOD_DOUBLE_CLICK_MOUSE: &str = "double_click_mouse";
pub(crate) const METHOD_DOUBLE_TAP_GESTURE: &str = "double_tap_gesture";
pub(crate) const METHOD_DRAG_MOUSE: &str = "drag_mouse";
#[cfg(feature = "diagnostics")]
pub(crate) const METHOD_GET_DIAGNOSTICS: &str = "get_diagnostics";
pub(crate) const METHOD_MOVE_MOUSE: &str = "move_mouse";
pub(crate) const METHOD_PINCH_GESTURE: &str = "pinch_gesture";
pub(crate) const METHOD_ROTATION_GESTURE: &str = "rotation_gesture";
pub(crate) const METHOD_SCREENSHOT: &str = "screenshot";
pub(crate) const METHOD_SCROLL_MOUSE: &str = "scroll_mouse";
pub(crate) const METHOD_SEND_KEYS: &str = "send_keys";
pub(crate) const METHOD_SEND_MOUSE_BUTTON: &str = "send_mouse_button";
pub(crate) const METHOD_SET_WINDOW_TITLE: &str = "set_window_title";
pub(crate) const METHOD_SHUTDOWN: &str = "shutdown";
pub(crate) const METHOD_TYPE_TEXT: &str = "type_text";

// environment variables
/// Environment variable that overrides the BRP extras HTTP port
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const BRP_EXTRAS_PORT_ENV_VAR: &str = "BRP_EXTRAS_PORT";

// error messages
pub(crate) const MISSING_REQUEST_PARAMETERS_MESSAGE: &str = "Missing request parameters";

// network constants
/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;
/// File extension used by screenshot output.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const IMAGE_EXTENSION_PNG: &str = "png";

// parameter fields
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PARAM_PATH: &str = "path";
pub(crate) const PARAM_TITLE: &str = "title";

// response fields
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_AVERAGE_FIELD: &str = "average";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_CURRENT_FIELD: &str = "current";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_FPS_FIELD: &str = "fps";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_FRAME_COUNT_FIELD: &str = "frame_count";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_FRAME_TIME_MS_FIELD: &str = "frame_time_ms";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_HISTORY_DURATION_SECS_FIELD: &str = "history_duration_secs";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_HISTORY_LEN_FIELD: &str = "history_len";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_MAX_HISTORY_LEN_FIELD: &str = "max_history_len";
#[cfg(feature = "diagnostics")]
pub(crate) const DIAGNOSTICS_SMOOTHED_FIELD: &str = "smoothed";
pub(crate) const RESPONSE_MESSAGE_FIELD: &str = "message";
pub(crate) const RESPONSE_NEW_TITLE_FIELD: &str = "new_title";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_NOTE_FIELD: &str = "note";
pub(crate) const RESPONSE_OLD_TITLE_FIELD: &str = "old_title";
pub(crate) const RESPONSE_PID_FIELD: &str = "pid";
pub(crate) const RESPONSE_STATUS_FIELD: &str = "status";
pub(crate) const RESPONSE_STATUS_SUCCESS: &str = "success";
pub(crate) const RESPONSE_SUCCESS_FIELD: &str = "success";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_WORKING_DIRECTORY_FIELD: &str = "working_directory";

// screenshot constants
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_SCREENSHOT_CAPTURE_ID_BYTES: usize = 128;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_CAPTURE_DEADLINE: Duration = Duration::from_secs(25);
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_CAPTURE_NOTE: &str =
    "Screenshot capture completed and the PNG was published.";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_ENTITY_NAME: &str = "BRP Screenshot Capture";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_STATUS_COMPLETED: &str = "completed";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const UNKNOWN_WORKING_DIRECTORY: &str = "unknown";

// shutdown constants
/// Number of frames to defer shutdown to allow the response to be sent
pub(crate) const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;
