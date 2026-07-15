//! Crate-level constants for `bevy_brp_extras`

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

// agent tool catalog constants
pub(crate) const AGENT_TOOLS_CATALOG_VERSION: u32 = 1;
pub(crate) const BACKING_METHOD_MISSING_REASON: &str = "backing_method_missing";
pub(crate) const BACKING_METHOD_WATCHING_REASON: &str = "backing_method_watching";

// command constants
/// Command prefix for `brp_extras` methods
pub(crate) const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";
pub(crate) const METHOD_AGENT_TOOLS: &str = "agent_tools";
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
pub(crate) const PARAM_CAMERA: &str = "camera";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PARAM_ENTITY: &str = "entity";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PARAM_PATH: &str = "path";
pub(crate) const PARAM_TITLE: &str = "title";

// response fields
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const CAMERA_CANDIDATES_FIELD: &str = "camera_candidates";
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
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_BOUNDS_KIND_FIELD: &str = "bounds_kind";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_CAPTURE_KIND_FIELD: &str = "capture_kind";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_HEIGHT_FIELD: &str = "height";
pub(crate) const RESPONSE_MESSAGE_FIELD: &str = "message";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_NAME_FIELD: &str = "name";
pub(crate) const RESPONSE_NEW_TITLE_FIELD: &str = "new_title";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_NOTE_FIELD: &str = "note";
pub(crate) const RESPONSE_OLD_TITLE_FIELD: &str = "old_title";
pub(crate) const RESPONSE_PID_FIELD: &str = "pid";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_REASON_FIELD: &str = "reason";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_RECT_FIELD: &str = "rect";
pub(crate) const RESPONSE_STATUS_FIELD: &str = "status";
pub(crate) const RESPONSE_STATUS_SUCCESS: &str = "success";
pub(crate) const RESPONSE_SUCCESS_FIELD: &str = "success";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_WIDTH_FIELD: &str = "width";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_WORKING_DIRECTORY_FIELD: &str = "working_directory";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_X_FIELD: &str = "x";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const RESPONSE_Y_FIELD: &str = "y";

// screenshot constants
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_BOUNDS_KIND_AABB: &str = "aabb";
#[cfg(all(feature = "ui", not(target_arch = "wasm32")))]
pub(crate) const SCREENSHOT_BOUNDS_KIND_UI: &str = "ui";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_CAMERA_REASON_AMBIGUOUS: &str = "ambiguous_camera";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_CAPTURE_DEADLINE: Duration = Duration::from_secs(25);
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_CAPTURE_KIND_ENTITY: &str = "entity";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_CAPTURE_NOTE: &str =
    "Screenshot capture completed and the PNG was published.";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_ENTITY_NAME: &str = "BRP Screenshot Capture";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_STATUS_COMPLETED: &str = "completed";
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SCREENSHOT_ZERO_PADDING: u32 = 0;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const UNKNOWN_WORKING_DIRECTORY: &str = "unknown";

// shutdown constants
/// Number of frames to defer shutdown to allow the response to be sent
pub(crate) const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;
