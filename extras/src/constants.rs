//! Crate-level constants for `bevy_brp_extras`

// command constants
/// Command prefix for `brp_extras` methods
pub(crate) const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";

// network constants
/// Default port for remote control connections
///
/// This matches Bevy's `RemoteHttpPlugin` default port to ensure compatibility.
pub const DEFAULT_REMOTE_PORT: u16 = 15702;

// shutdown constants
/// Number of frames to defer shutdown to allow the response to be sent
pub(crate) const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;
