use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};

/// Error when multiple targets with the same name exist
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct PathDisambiguationError {
    #[to_error_info]
    available_paths: Vec<String>,

    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_message(
        message_template = "Found multiple {target_type}s named `{target_name}`. Please specify which path to use."
    )]
    message_template: String,
}

/// Error when target exists but not at the specified path
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct TargetNotFoundAtSpecifiedPath {
    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_error_info]
    searched_path: Option<String>,

    #[to_error_info]
    available_paths: Vec<String>,

    #[to_message(
        message_template = "{target_type} `{target_name}` exists but not at path `{searched_path}`. Available at: `{available_paths`"
    )]
    message_template: String,
}

/// Error when no targets found - apps only, we don't detect it for examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct NoTargetsFoundError {
    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_message(message_template = "No {target_type} named `{target_name}` found in workspace")]
    message_template: String,
}

/// Error when build is required
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct BuildRequiredError {
    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_error_info]
    profile: String,

    #[to_message(
        message_template = "{target_type} `{target_name}` needs to be built first. Run: cargo build --{profile}"
    )]
    message_template: String,
}
