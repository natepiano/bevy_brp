use std::path::PathBuf;

use tracing::debug;

use super::project_discovery;
use super::relative_paths;
use crate::app_tools::targets::cargo_detector::BevyTarget;
use crate::app_tools::targets::cargo_detector::CargoDetector;
use crate::app_tools::targets::cargo_detector::TargetType;
use crate::app_tools::targets::errors::NoTargetsFoundError;
use crate::app_tools::targets::errors::PackageDisambiguationError;
use crate::error::Error;

/// Collect all Bevy targets (apps and examples) across search paths without name filtering.
///
/// Used for enriched not-found errors that show every available target.
pub fn collect_all_bevy_targets(search_paths: &[PathBuf]) -> Vec<BevyTarget> {
    let mut targets = Vec::new();
    for path in project_discovery::iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::try_from(path.as_path()) {
            let mut found = detector.find_bevy_targets();
            for target in &mut found {
                let manifest_dir = target
                    .manifest_path
                    .parent()
                    .unwrap_or(&target.manifest_path);
                target.relative_path =
                    relative_paths::compute_relative_path(manifest_dir, search_paths);
            }
            targets.extend(found);
        }
    }
    targets
}

/// Find all targets by name across search paths, filtered by target type when provided.
pub fn find_all_targets_by_name(
    target_name: &str,
    target_type: Option<TargetType>,
    search_paths: &[PathBuf],
) -> Vec<BevyTarget> {
    let mut targets = Vec::new();

    for path in project_discovery::iter_cargo_project_paths(search_paths) {
        if let Ok(detector) = CargoDetector::try_from(path.as_path()) {
            let found_targets = detector.find_bevy_targets();
            for mut target in found_targets {
                if target.name == target_name {
                    if let Some(required_type) = target_type
                        && target.target_type != required_type
                    {
                        continue;
                    }

                    let manifest_dir = target
                        .manifest_path
                        .parent()
                        .unwrap_or(&target.manifest_path);
                    target.relative_path =
                        relative_paths::compute_relative_path(manifest_dir, search_paths);
                    targets.push(target);
                }
            }
        }
    }

    targets
}

/// Find a required target by name with optional `package_name` disambiguation.
///
/// When multiple targets share the same name across different packages,
/// `package_name` filters by exact match against `BevyTarget::package_name`.
///
/// When `cached_targets` is provided, these results are used instead of
/// scanning again.
pub fn find_required_target_with_package_name(
    target_name: &str,
    target_type: TargetType,
    package_name: Option<&str>,
    search_paths: &[PathBuf],
    cached_targets: Option<Vec<BevyTarget>>,
) -> Result<BevyTarget, Error> {
    let target_type_str = match target_type {
        TargetType::App => "app",
        TargetType::Example => "example",
    };

    debug!("Searching for {target_type_str} '{target_name}'");
    if let Some(package_name) = package_name {
        debug!("With package_name filter: {package_name}");
    }

    let all_targets = cached_targets.unwrap_or_else(|| {
        debug!("No cached targets provided, scanning filesystem");
        find_all_targets_by_name(target_name, Some(target_type), search_paths)
    });
    debug!("Found {} matching {target_type_str}(s)", all_targets.len());

    let filtered = if let Some(package_name) = package_name {
        let matched: Vec<_> = all_targets
            .iter()
            .filter(|target| target.package_name == package_name)
            .cloned()
            .collect();

        if matched.is_empty() {
            let available: Vec<String> = all_targets
                .iter()
                .map(|target| target.package_name.clone())
                .collect();

            let error = super::super::errors::TargetNotFoundInPackage::new(
                target_name.to_string(),
                target_type_str.to_string(),
                Some(package_name.to_string()),
                available,
            );
            return Err(Error::Structured {
                result: Box::new(error),
            });
        }

        matched
    } else {
        all_targets
    };

    match filtered.len() {
        0 => {
            let error =
                NoTargetsFoundError::new(target_name.to_string(), target_type_str.to_string());
            Err(Error::Structured {
                result: Box::new(error),
            })
        },
        1 => {
            let mut filtered = filtered.into_iter();
            filtered.next().map_or_else(
                || {
                    let error = NoTargetsFoundError::new(
                        target_name.to_string(),
                        target_type_str.to_string(),
                    );
                    Err(Error::Structured {
                        result: Box::new(error),
                    })
                },
                Ok,
            )
        },
        _ => {
            let available: Vec<String> = filtered
                .iter()
                .map(|target| target.package_name.clone())
                .collect();

            let error = PackageDisambiguationError::new(
                available,
                target_name.to_string(),
                target_type_str.to_string(),
            );
            Err(Error::Structured {
                result: Box::new(error),
            })
        },
    }
}
