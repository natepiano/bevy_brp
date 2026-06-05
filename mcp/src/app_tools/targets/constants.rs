// directory names
pub(super) const CARGO_EXAMPLES_DIRECTORY: &str = "examples";
pub(super) const CARGO_SRC_DIRECTORY: &str = "src";
pub(super) const HIDDEN_DIRECTORY_PREFIX: char = '.';
pub(super) const TARGET_DIRECTORY_NAME: &str = "target";

// file extensions
pub(super) const RUST_SOURCE_EXTENSION: &str = "rs";

// package and feature names
pub(super) const BEVY_CRATE_NAME: &str = "bevy";
pub(super) const BEVY_REMOTE_FEATURE: &str = "bevy_remote";
pub(super) const MCP_CRATE_NAME: &str = "bevy_brp_mcp";

// response fields
pub(super) const BRP_LEVEL_FIELD: &str = "brp_level";
pub(super) const BUILD_BUILT_FIELD: &str = "built";
pub(super) const BUILDS_FIELD: &str = "builds";
pub(super) const KIND_FIELD: &str = "kind";
pub(super) const NAME_FIELD: &str = "name";
pub(super) const PACKAGE_NAME_FIELD: &str = "package_name";
pub(super) const PATH_FIELD: &str = "path";
pub(super) const RELATIVE_PATH_FIELD: &str = "relative_path";
pub(super) const WORKSPACE_ROOT_FIELD: &str = "workspace_root";

// source probes
pub(super) const BEVY_REMOTE_GLOB_IMPORT_PREFIX: &str = "use bevy::remote::{";
pub(super) const BEVY_REMOTE_PLUGIN_IMPORT: &str = "use bevy::remote::RemotePlugin";
pub(super) const BEVY_REMOTE_REMOTE_GLOB_IMPORT_PREFIX: &str = "use bevy_remote::{";
pub(super) const BEVY_REMOTE_REMOTE_PLUGIN_IMPORT: &str = "use bevy_remote::RemotePlugin";
pub(super) const BRP_EXTRAS_GLOB_IMPORT_PREFIX: &str = "use bevy_brp_extras::{";
pub(super) const BRP_EXTRAS_PLUGIN_IMPORT: &str = "use bevy_brp_extras::BrpExtrasPlugin";
pub(super) const BRP_EXTRAS_PLUGIN_NAME: &str = "BrpExtrasPlugin";
pub(super) const CURRENT_DIRECTORY_SEGMENT: &str = ".";
pub(super) const REMOTE_PLUGIN_NAME: &str = "RemotePlugin";

// target kinds
pub(super) const TARGET_KIND_APP: &str = "app";
pub(super) const TARGET_KIND_EXAMPLE: &str = "example";
