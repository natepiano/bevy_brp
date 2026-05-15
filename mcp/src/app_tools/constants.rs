use std::ops::RangeInclusive;
use std::time::Duration;

// build artifact paths
pub(super) const EXAMPLES_PATH_SEGMENT: &str = "/examples/";
pub(super) const MCP_BINARY_NAME: &str = "bevy_brp_mcp";
pub(super) const TARGET_DEBUG_PATH: &str = "target/debug";
pub(super) const TARGET_RELEASE_PATH: &str = "target/release";

// cargo constants
pub(super) const CARGO_BIN_FLAG: &str = "--bin";
pub(super) const CARGO_COMMAND_NAME: &str = "cargo";
pub(super) const CARGO_EXAMPLE_FLAG: &str = "--example";
pub(super) const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
pub(super) const CARGO_RUN_SUBCOMMAND: &str = "run";

// instance count constants
/// Maximum number of instances (100)
pub(super) const MAX_INSTANCE_COUNT: u16 = 100;
/// Minimum number of instances (1)
pub(super) const MIN_INSTANCE_COUNT: u16 = 1;
/// Valid range for instance count
pub(super) const VALID_INSTANCE_RANGE: RangeInclusive<u16> =
    MIN_INSTANCE_COUNT..=MAX_INSTANCE_COUNT;

// json fields
pub(super) const MANIFEST_PATH_FIELD: &str = "manifest_path";
pub(super) const PID_FIELD: &str = "pid";

// profile constants
pub(super) const DEFAULT_PROFILE: &str = PROFILE_DEBUG;
pub(super) const PROFILE_DEBUG: &str = "debug";
pub(super) const PROFILE_RELEASE: &str = "release";

// status polling constants
/// Maximum number of retries when checking BRP port responsiveness
pub(super) const STATUS_MAX_RETRIES: u32 = 5;
/// Delay between BRP status poll retries
pub(super) const STATUS_POLL_INTERVAL: Duration = std::time::Duration::from_millis(500);
