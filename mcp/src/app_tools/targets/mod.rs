mod cargo_detector;
mod collection_strategy;
mod constants;
mod errors;
mod scanning;

pub(super) use cargo_detector::BevyTarget;
pub(super) use cargo_detector::TargetType;
pub use collection_strategy::collect_all_bevy_targets;
pub(super) use errors::AvailableTarget;
pub(super) use errors::UnifiedTargetNotFoundError;
pub(super) use scanning::filter_targets_by_path_scope;
pub(super) use scanning::find_all_targets_by_name;
pub(super) use scanning::find_required_target_with_package_name;
pub(super) use scanning::resolve_search_paths;
pub(super) use scanning::scan_bevy_targets;
