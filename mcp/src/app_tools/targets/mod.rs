mod cargo_detector;
mod collection_strategy;
mod errors;
mod list_common;
mod scanning;

pub(super) use cargo_detector::BevyTarget;
pub(super) use cargo_detector::TargetType;
pub(super) use errors::AvailableTarget;
pub(super) use errors::UnifiedTargetNotFoundError;
pub use list_common::collect_all_bevy_targets;
pub(super) use scanning::collect_all_bevy_targets as scan_bevy_targets;
pub(super) use scanning::filter_targets_by_path_scope;
pub(super) use scanning::find_all_targets_by_name;
pub(super) use scanning::find_required_target_with_package_name;
