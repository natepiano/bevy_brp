// Profile constants
pub(super) const DEFAULT_PROFILE: &str = PROFILE_DEBUG;
pub(super) const PROFILE_DEBUG: &str = "debug";
pub(super) const PROFILE_RELEASE: &str = "release";

// Status polling constants
/// Maximum number of retries when checking BRP port responsiveness
pub(super) const STATUS_MAX_RETRIES: u32 = 5;
/// Delay between BRP status poll retries
pub(super) const STATUS_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
