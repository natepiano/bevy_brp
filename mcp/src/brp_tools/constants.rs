use std::ops::RangeInclusive;

// Network constants
/// Environment variable name for BRP port
pub const BRP_EXTRAS_PORT_ENV_VAR: &str = "BRP_EXTRAS_PORT";

/// Network/Port Constants
pub(super) const DEFAULT_BRP_EXTRAS_PORT: u16 = 15702;

pub const MAX_VALID_PORT: u16 = 65534; // Leave room for calculations

/// valid ports
pub(super) const MIN_VALID_PORT: u16 = 1024; // Non-privileged ports start here

pub(super) const VALID_PORT_RANGE: RangeInclusive<u16> = MIN_VALID_PORT..=MAX_VALID_PORT;
