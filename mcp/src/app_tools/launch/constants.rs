// cargo arguments
pub(super) const CARGO_RELEASE_FLAG: &str = "--release";

// cargo build json fields
pub(super) const BUILD_OUTPUT_FRESH_FIELD: &str = "fresh";
pub(super) const BUILD_OUTPUT_NAME_FIELD: &str = "name";
pub(super) const BUILD_OUTPUT_TARGET_FIELD: &str = "target";

// error details
pub(super) const ERROR_CHAIN_FIELD: &str = "error_chain";
pub(super) const ERROR_FIELD: &str = "error";

// filesystem paths
pub(super) const BUILD_SCRIPT_FILE: &str = "build.rs";
pub(super) const CARGO_CONFIG_DIR: &str = ".cargo";
pub(super) const CARGO_CONFIG_FILE: &str = "config";
pub(super) const CARGO_CONFIG_TOML_FILE: &str = "config.toml";
pub(super) const CARGO_LOCK_FILE: &str = "Cargo.lock";
pub(super) const DEP_INFO_EXTENSION: &str = "d";
pub(super) const RUST_TOOLCHAIN_FILE: &str = "rust-toolchain";
pub(super) const RUST_TOOLCHAIN_TOML_FILE: &str = "rust-toolchain.toml";

// logging
pub(super) const LOG_WRITE_ERROR_MESSAGE: &str = "Failed to write to log file";
