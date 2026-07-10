//! Crate-level constants for `bevy_brp_extras`

// command constants
/// Command prefix for `brp_extras` methods
pub(crate) const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";
pub(crate) const METHOD_CLICK_MOUSE: &str = "click_mouse";
pub(crate) const METHOD_DOUBLE_CLICK_MOUSE: &str = "double_click_mouse";
pub(crate) const METHOD_DOUBLE_TAP_GESTURE: &str = "double_tap_gesture";
pub(crate) const METHOD_DRAG_MOUSE: &str = "drag_mouse";
pub(crate) const METHOD_FIND_ENTITIES_BY_NAME: &str = "find_entities_by_name";
pub(crate) const METHOD_GET_DIAGNOSTICS: &str = "get_diagnostics";
pub(crate) const METHOD_MOVE_MOUSE: &str = "move_mouse";
pub(crate) const METHOD_PINCH_GESTURE: &str = "pinch_gesture";
pub(crate) const METHOD_ROTATION_GESTURE: &str = "rotation_gesture";
pub(crate) const METHOD_SCREENSHOT: &str = "screenshot";
pub(crate) const METHOD_SCREENSHOT_ENTITY: &str = "screenshot_entity";
pub(crate) const METHOD_SCROLL_MOUSE: &str = "scroll_mouse";
pub(crate) const METHOD_SEND_KEYS: &str = "send_keys";
pub(crate) const METHOD_SEND_MOUSE_BUTTON: &str = "send_mouse_button";
pub(crate) const METHOD_SET_WINDOW_TITLE: &str = "set_window_title";
pub(crate) const METHOD_SHUTDOWN: &str = "shutdown";
pub(crate) const METHOD_SNAPSHOT: &str = "snapshot";
pub(crate) const METHOD_TYPE_TEXT: &str = "type_text";

// environment variables
/// Environment variable that overrides the BRP extras HTTP port
pub(crate) const BRP_EXTRAS_PORT_ENV_VAR: &str = "BRP_EXTRAS_PORT";

// error messages
pub(crate) const MISSING_REQUEST_PARAMETERS_MESSAGE: &str = "Missing request parameters";

// network constants
/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;
/// File extension used by screenshot output.
pub(crate) const IMAGE_EXTENSION_PNG: &str = "png";

// parameter fields
pub(crate) const PARAM_NAME: &str = "name";
pub(crate) const PARAM_PATH: &str = "path";
pub(crate) const PARAM_ROOT: &str = "root";
pub(crate) const PARAM_TITLE: &str = "title";

// snapshot / screenshot_entity response fields
pub(crate) const RESPONSE_RECT_FIELD: &str = "rect";
pub(crate) const RESPONSE_YAML_FIELD: &str = "yaml";

// find_entities_by_name response fields
pub(crate) const RESPONSE_ENTITY_FIELD: &str = "entity";
pub(crate) const RESPONSE_NAME_FIELD: &str = "name";

// response fields
pub(crate) const DIAGNOSTICS_AVERAGE_FIELD: &str = "average";
pub(crate) const DIAGNOSTICS_CURRENT_FIELD: &str = "current";
pub(crate) const DIAGNOSTICS_FPS_FIELD: &str = "fps";
pub(crate) const DIAGNOSTICS_FRAME_COUNT_FIELD: &str = "frame_count";
pub(crate) const DIAGNOSTICS_FRAME_TIME_MS_FIELD: &str = "frame_time_ms";
pub(crate) const DIAGNOSTICS_HISTORY_DURATION_SECS_FIELD: &str = "history_duration_secs";
pub(crate) const DIAGNOSTICS_HISTORY_LEN_FIELD: &str = "history_len";
pub(crate) const DIAGNOSTICS_MAX_HISTORY_LEN_FIELD: &str = "max_history_len";
pub(crate) const DIAGNOSTICS_SMOOTHED_FIELD: &str = "smoothed";
pub(crate) const RESPONSE_MESSAGE_FIELD: &str = "message";
pub(crate) const RESPONSE_NEW_TITLE_FIELD: &str = "new_title";
pub(crate) const RESPONSE_NOTE_FIELD: &str = "note";
pub(crate) const RESPONSE_OLD_TITLE_FIELD: &str = "old_title";
pub(crate) const RESPONSE_PID_FIELD: &str = "pid";
pub(crate) const RESPONSE_STATUS_FIELD: &str = "status";
pub(crate) const RESPONSE_STATUS_SUCCESS: &str = "success";
pub(crate) const RESPONSE_SUCCESS_FIELD: &str = "success";
pub(crate) const RESPONSE_WORKING_DIRECTORY_FIELD: &str = "working_directory";

// screenshot response
pub(crate) const SCREENSHOT_CAPTURE_NOTE: &str =
    "Screenshot capture initiated. File I/O will be performed asynchronously on background thread.";
pub(crate) const UNKNOWN_WORKING_DIRECTORY: &str = "unknown";

// shutdown constants
/// Number of frames to defer shutdown to allow the response to be sent
pub(crate) const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;
