//! Constants for mouse input simulation

/// Maximum duration for timed mouse button releases (60 seconds)
pub(super) const MAX_MOUSE_DURATION_MS: u32 = 60_000;

/// Default duration for mouse button presses (100 milliseconds)
pub(super) const DEFAULT_MOUSE_DURATION_MS: u32 = 100;

/// Default delay between clicks for double click (250 milliseconds)
pub(super) const DEFAULT_DOUBLE_CLICK_DELAY_MS: u32 = 250;
