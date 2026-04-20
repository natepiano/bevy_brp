use bevy_brp_mcp_macros::ResultStruct;
use serde::Deserialize;
use serde::Serialize;

/// Error when multiple targets with the same name exist across packages
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub(super) struct PackageDisambiguationError {
    #[to_error_info]
    available_package_names: Vec<String>,

    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_message(
        message_template = "Found multiple {target_type}s named `{target_name}`. Please specify `package_name` to disambiguate."
    )]
    message_template: String,
}

/// Error when target exists but not in the specified package
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub(super) struct TargetNotFoundInPackage {
    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_error_info]
    searched_package_name: Option<String>,

    #[to_error_info]
    available_package_names: Vec<String>,

    #[to_message(
        message_template = "{target_type} `{target_name}` not found in package `{searched_package_name}`. Available in: {available_package_names}"
    )]
    message_template: String,
}

/// Error when no targets found - apps only, we don't detect it for examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub(super) struct NoTargetsFoundError {
    #[to_error_info]
    target_name: String,

    #[to_error_info]
    target_type: String,

    #[to_message(message_template = "No {target_type} named `{target_name}` found in workspace")]
    message_template: String,
}

/// An available target for enriched not-found errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableTarget {
    pub name: String,
    pub kind: String,
    pub path: String,
}

/// Error when no app or example with the given name was found across all target types
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct UnifiedTargetNotFoundError {
    #[to_error_info]
    target_name: String,

    #[to_error_info]
    available_targets: Vec<AvailableTarget>,

    #[to_message(message_template = "No app or example named `{target_name}` found")]
    message_template: String,
}
