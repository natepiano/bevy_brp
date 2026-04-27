mod project_discovery;
mod relative_paths;
mod target_lookup;

pub use project_discovery::iter_cargo_project_paths;
pub use relative_paths::compute_relative_path;
pub use relative_paths::filter_targets_by_path_scope;
pub use target_lookup::collect_all_bevy_targets as scan_bevy_targets;
pub use target_lookup::find_all_targets_by_name;
pub use target_lookup::find_required_target_with_package_name;
