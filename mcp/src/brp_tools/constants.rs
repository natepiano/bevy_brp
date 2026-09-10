use std::ops::RangeInclusive;

// agent tool catalog constants
pub(super) const AGENT_TOOL_CATALOG_METHOD: &str = "brp_extras/agent_tools";
pub(super) const AGENT_TOOL_CATALOG_USAGE: &str =
    "Pass an entry's method and matching params to brp_execute.";
pub(super) const AGENT_TOOL_CATALOG_VERSION: u32 = 1;

// network constants
/// Environment variable name for BRP port
pub const BRP_EXTRAS_PORT_ENV_VAR: &str = "BRP_EXTRAS_PORT";
pub(super) const DEFAULT_BRP_EXTRAS_PORT: u16 = 15702;
/// Upper bound for `VALID_PORT_RANGE` and `app_tools::launch::orchestration::validate_port_range`.
/// The limit of 65534 rejects a last-port calculation saturated to `u16::MAX` by
/// `base_port.saturating_add(instance_count.saturating_sub(1))`.
pub const MAX_VALID_PORT: u16 = 65534;
/// Non-privileged ports start here
pub(super) const MIN_VALID_PORT: u16 = 1024;
pub(super) const VALID_PORT_RANGE: RangeInclusive<u16> = MIN_VALID_PORT..=MAX_VALID_PORT;

// query constants
pub(super) const COMPONENT_SELECTOR_ALL: &str = "all";
