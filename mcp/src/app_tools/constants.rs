use std::ops::RangeInclusive;
use std::time::Duration;

// instance count constants
/// Maximum number of instances (100)
pub(super) const MAX_INSTANCE_COUNT: u16 = 100;
/// Minimum number of instances (1)
pub(super) const MIN_INSTANCE_COUNT: u16 = 1;
/// Valid range for instance count
pub(super) const VALID_INSTANCE_RANGE: RangeInclusive<u16> =
    MIN_INSTANCE_COUNT..=MAX_INSTANCE_COUNT;

// profile constants
pub(super) const DEFAULT_PROFILE: &str = PROFILE_DEBUG;
pub(super) const PROFILE_DEBUG: &str = "debug";
pub(super) const PROFILE_RELEASE: &str = "release";

// status polling constants
/// Maximum number of retries when checking BRP port responsiveness
pub(super) const STATUS_MAX_RETRIES: u32 = 5;
/// Delay between BRP status poll retries
pub(super) const STATUS_POLL_INTERVAL: Duration = std::time::Duration::from_millis(500);
