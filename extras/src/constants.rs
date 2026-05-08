//! Crate-level constants for `bevy_brp_extras`

// command constants
/// Command prefix for `brp_extras` methods
pub(crate) const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";

// environment variables
/// Environment variable that overrides the BRP extras HTTP port
pub(crate) const BRP_EXTRAS_PORT_ENV_VAR: &str = "BRP_EXTRAS_PORT";

// network constants
/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;
/// File extension used by screenshot output.
pub(crate) const IMAGE_EXTENSION_PNG: &str = "png";

// parameter fields
pub(crate) const PARAM_PATH: &str = "path";
pub(crate) const PARAM_TITLE: &str = "title";

// shutdown constants
/// Number of frames to defer shutdown to allow the response to be sent
pub(crate) const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;
