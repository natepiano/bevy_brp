use bevy_brp_mcp_macros::ResultStruct;
use serde::Deserialize;
use serde::Serialize;

/// Error when multiple targets with the same name exist across packages
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub(super) struct PackageDisambiguationError {
    #[serde(rename = "available_package_names")]
    #[to_error_info]
    available_packages: Vec<String>,

    #[serde(rename = "target_name")]
    #[to_error_info]
    target: String,

    #[serde(rename = "target_type")]
    #[to_error_info]
    kind: String,

    #[to_message(
        message_template = "Found multiple {target_type}s named `{target_name}`. Please specify `package_name` to disambiguate."
    )]
    message_template: String,
}

/// Error when target exists but not in the specified package
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub(super) struct TargetNotFoundInPackage {
    #[serde(rename = "target_name")]
    #[to_error_info]
    target: String,

    #[serde(rename = "target_type")]
    #[to_error_info]
    kind: String,

    #[serde(rename = "searched_package_name")]
    #[to_error_info]
    searched_package: Option<String>,

    #[serde(rename = "available_package_names")]
    #[to_error_info]
    available_packages: Vec<String>,

    #[to_message(
        message_template = "{target_type} `{target_name}` not found in package `{searched_package_name}`. Available in: {available_package_names}"
    )]
    message_template: String,
}

/// Error when no targets found - apps only, we don't detect it for examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub(super) struct NoTargetsFoundError {
    #[serde(rename = "target_name")]
    #[to_error_info]
    target: String,

    #[serde(rename = "target_type")]
    #[to_error_info]
    kind: String,

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
