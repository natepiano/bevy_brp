//! Crate-level constants for `bevy_brp_extras`

/// Command prefix for `brp_extras` methods
pub(crate) const EXTRAS_COMMAND_PREFIX: &str = "brp_extras/";

/// Number of frames to defer shutdown to allow the response to be sent
pub(crate) const DEFERRED_SHUTDOWN_FRAMES: u32 = 10;
