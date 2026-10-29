use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use super::constants::BUILD_SCRIPT_FILE;
use super::constants::CARGO_CONFIG_DIR;
use super::constants::CARGO_CONFIG_FILE;
use super::constants::CARGO_CONFIG_TOML_FILE;
use super::constants::CARGO_LOCK_FILE;
use super::constants::DEP_INFO_EXTENSION;
use super::constants::DEV_FEATURES_RECORD_SUFFIX;
use super::constants::RUST_TOOLCHAIN_FILE;
use super::constants::RUST_TOOLCHAIN_TOML_FILE;
use crate::app_tools::constants::CARGO_MANIFEST_FILE;
use crate::app_tools::targets::BevyTarget;
use crate::error::Error;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FreshnessCheckResult {
    Fresh,
    Stale(String),
    Unknown(String),
}

enum BuiltDevFeatures {
    Recorded(Vec<String>),
    Unrecorded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MissingInputPolicy {
    Ignore,
    TreatAsStale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackslashState {
    ReadingToken,
    Escaped,
}

impl BackslashState {
    const fn is_escaped(self) -> bool { matches!(self, Self::Escaped) }
}

pub(super) fn check_target_freshness(
    target: &BevyTarget,
    profile: &str,
    features: &[String],
) -> FreshnessCheckResult {
    if !target.is_app() {
        return FreshnessCheckResult::Unknown(
            "lock-free freshness checks are only supported for app binaries".to_string(),
        );
    }

    try_check_target_freshness(target, profile, features)
        .unwrap_or_else(|error| FreshnessCheckResult::Unknown(format!("{error}")))
}

fn try_check_target_freshness(
    target: &BevyTarget,
    profile: &str,
    features: &[String],
) -> Result<FreshnessCheckResult> {
    let binary_path = target.get_binary_path(profile);
    if !binary_path.exists() {
        return Ok(FreshnessCheckResult::Stale(format!(
            "binary does not exist: {}",
            binary_path.display()
        )));
    }

    let binary_mtime = file_modified_time(&binary_path)?;
    let record_path = dev_features_record_path(&binary_path);
    match read_built_dev_features(&record_path, binary_mtime)? {
        BuiltDevFeatures::Recorded(built_features) if built_features != features => {
            return Ok(FreshnessCheckResult::Stale(format!(
                "binary dev features {built_features:?} differ from wanted dev features {features:?}"
            )));
        },
        BuiltDevFeatures::Unrecorded if !features.is_empty() => {
            return Ok(FreshnessCheckResult::Stale(format!(
                "binary has no dev features record newer than it: {}",
                record_path.display()
            )));
        },
        BuiltDevFeatures::Recorded(_) | BuiltDevFeatures::Unrecorded => {},
    }
    let dep_info_path = dep_info_path(target, profile);
    if !dep_info_path.exists() {
        return Ok(FreshnessCheckResult::Unknown(format!(
            "dep-info file does not exist: {}",
            dep_info_path.display()
        )));
    }

    let dep_info_contents = fs::read_to_string(&dep_info_path).map_err(|error| {
        Error::FileOperation(format!(
            "Failed to read dep-info file {}: {error}",
            dep_info_path.display()
        ))
    })?;
    let dep_info_dir = dep_info_path
        .parent()
        .ok_or_else(|| Error::FileOrPathNotFound("Dep-info file has no parent directory".into()))?;
    let dependencies = parse_dep_info_dependencies(&dep_info_contents, dep_info_dir);

    if dependencies.is_empty() {
        return Ok(FreshnessCheckResult::Unknown(format!(
            "dep-info file had no dependencies: {}",
            dep_info_path.display()
        )));
    }

    for dependency in dependencies {
        let Some(staleness_reason) = compare_input_to_binary(&dependency, binary_mtime)? else {
            continue;
        };
        return Ok(FreshnessCheckResult::Stale(staleness_reason));
    }

    for input in extra_fingerprint_inputs(target) {
        let Some(staleness_reason) = compare_optional_input_to_binary(&input, binary_mtime)? else {
            continue;
        };
        return Ok(FreshnessCheckResult::Stale(staleness_reason));
    }

    Ok(FreshnessCheckResult::Fresh)
}

fn dev_features_record_path(binary_path: &Path) -> PathBuf {
    let mut path = binary_path.as_os_str().to_os_string();
    path.push(DEV_FEATURES_RECORD_SUFFIX);
    PathBuf::from(path)
}

fn read_built_dev_features(
    record_path: &Path,
    binary_mtime: SystemTime,
) -> Result<BuiltDevFeatures> {
    let metadata = match fs::metadata(record_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(BuiltDevFeatures::Unrecorded);
        },
        Err(error) => {
            return Err(Error::FileOperation(format!(
                "Failed to read dev features record metadata {}: {error}",
                record_path.display()
            ))
            .into());
        },
    };
    let record_mtime = metadata.modified().map_err(|error| {
        Error::FileOperation(format!(
            "Failed to read modification time for {}: {error}",
            record_path.display()
        ))
    })?;
    if record_mtime < binary_mtime {
        return Ok(BuiltDevFeatures::Unrecorded);
    }
    let contents = fs::read_to_string(record_path).map_err(|error| {
        Error::FileOperation(format!(
            "Failed to read dev features record {}: {error}",
            record_path.display()
        ))
    })?;

    Ok(BuiltDevFeatures::Recorded(
        contents.lines().map(String::from).collect(),
    ))
}

pub(super) fn write_built_dev_features(binary_path: &Path, features: &[String]) {
    let record_path = dev_features_record_path(binary_path);
    let mut contents = features.join("\n");
    if !features.is_empty() {
        contents.push('\n');
    }
    if let Err(error) = fs::write(&record_path, contents) {
        tracing::warn!(
            path = %record_path.display(),
            %error,
            "Failed to write dev features record"
        );
    }
}

fn dep_info_path(target: &BevyTarget, profile: &str) -> PathBuf {
    target
        .get_binary_path(profile)
        .with_extension(DEP_INFO_EXTENSION)
}

fn extra_fingerprint_inputs(target: &BevyTarget) -> Vec<PathBuf> {
    let mut inputs = vec![target.manifest.clone()];

    let workspace_manifest = target.workspace_root.join(CARGO_MANIFEST_FILE);
    if workspace_manifest != target.manifest {
        inputs.push(workspace_manifest);
    }

    inputs.push(target.workspace_root.join(CARGO_LOCK_FILE));
    inputs.extend(find_cargo_config_files(
        &target.manifest,
        &target.workspace_root,
    ));

    if let Some(package_dir) = target.manifest.parent() {
        inputs.push(package_dir.join(BUILD_SCRIPT_FILE));
    }

    inputs.push(target.workspace_root.join(RUST_TOOLCHAIN_TOML_FILE));
    inputs.push(target.workspace_root.join(RUST_TOOLCHAIN_FILE));

    inputs
}

fn find_cargo_config_files(manifest_path: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    let mut configs = Vec::new();

    let Some(mut current_dir) = manifest_path.parent() else {
        return configs;
    };

    loop {
        configs.push(
            current_dir
                .join(CARGO_CONFIG_DIR)
                .join(CARGO_CONFIG_TOML_FILE),
        );
        configs.push(current_dir.join(CARGO_CONFIG_DIR).join(CARGO_CONFIG_FILE));

        if current_dir == workspace_root {
            break;
        }

        let Some(parent) = current_dir.parent() else {
            break;
        };
        current_dir = parent;
    }

    configs
}

fn compare_input_to_binary(input_path: &Path, binary_mtime: SystemTime) -> Result<Option<String>> {
    compare_path_to_binary(
        input_path,
        binary_mtime,
        MissingInputPolicy::TreatAsStale,
        "dependency listed in dep-info is missing",
        "dependency is newer than binary",
    )
}

fn compare_optional_input_to_binary(
    input_path: &Path,
    binary_mtime: SystemTime,
) -> Result<Option<String>> {
    compare_path_to_binary(
        input_path,
        binary_mtime,
        MissingInputPolicy::Ignore,
        "",
        "build input is newer than binary",
    )
}

fn compare_path_to_binary(
    input_path: &Path,
    binary_mtime: SystemTime,
    missing_input_policy: MissingInputPolicy,
    missing_reason: &str,
    stale_reason: &str,
) -> Result<Option<String>> {
    if !input_path.exists() {
        return Ok((missing_input_policy == MissingInputPolicy::TreatAsStale)
            .then(|| format!("{missing_reason}: {}", input_path.display())));
    }

    let input_mtime = file_modified_time(input_path)?;
    Ok((input_mtime > binary_mtime).then(|| format!("{stale_reason}: {}", input_path.display())))
}

fn file_modified_time(path: &Path) -> Result<SystemTime> {
    fs::metadata(path)
        .map_err(|error| {
            Error::FileOperation(format!(
                "Failed to read metadata for {}: {error}",
                path.display()
            ))
        })?
        .modified()
        .map_err(|error| {
            Error::FileOperation(format!(
                "Failed to read modification time for {}: {error}",
                path.display()
            ))
            .into()
        })
}

fn parse_dep_info_dependencies(contents: &str, base_dir: &Path) -> Vec<PathBuf> {
    let Some((_, dependency_text)) = contents.split_once(':') else {
        return Vec::new();
    };

    let mut dependencies = Vec::new();
    let mut current = String::new();
    let mut backslash_state = BackslashState::ReadingToken;

    for ch in dependency_text.chars() {
        if backslash_state.is_escaped() {
            match ch {
                '\n' | '\r' => {},
                _ => current.push(ch),
            }
            backslash_state = BackslashState::ReadingToken;
            continue;
        }

        match ch {
            '\\' => backslash_state = BackslashState::Escaped,
            c if c.is_whitespace() => {
                push_dependency(&mut dependencies, &mut current, base_dir);
            },
            _ => current.push(ch),
        }
    }

    push_dependency(&mut dependencies, &mut current, base_dir);
    dependencies
}

fn push_dependency(dependencies: &mut Vec<PathBuf>, current: &mut String, base_dir: &Path) {
    if current.is_empty() {
        return;
    }

    let raw_path = std::mem::take(current);
    let path = PathBuf::from(&raw_path);
    if path.is_absolute() {
        dependencies.push(path);
    } else {
        dependencies.push(base_dir.join(path));
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests should panic on unexpected values"
)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::time::SystemTime;

    use tempfile::tempdir;

    use super::CARGO_LOCK_FILE;
    use super::DEV_FEATURES_RECORD_SUFFIX;
    use super::FreshnessCheckResult;
    use super::check_target_freshness;
    use super::dep_info_path;
    use super::parse_dep_info_dependencies;
    use super::write_built_dev_features;
    use crate::app_tools::constants::CARGO_MANIFEST_FILE;
    use crate::app_tools::constants::PROFILE_DEBUG;
    use crate::app_tools::targets::BevyTarget;
    use crate::app_tools::targets::DevFeatures;
    use crate::app_tools::targets::TargetType;

    const INPUT_TIMESTAMP_SECONDS: u64 = 100;
    const BINARY_TIMESTAMP_SECONDS: u64 = 200;
    const RECORD_TIMESTAMP_SECONDS: u64 = 300;

    fn test_target(workspace_root: &Path, manifest_path: &Path, name: &str) -> BevyTarget {
        BevyTarget {
            name:           name.to_string(),
            target_type:    TargetType::App,
            package_name:   "pkg".to_string(),
            workspace_root: workspace_root.to_path_buf(),
            manifest:       manifest_path.to_path_buf(),
            relative:       PathBuf::new(),
            source:         PathBuf::new(),
            dev_features:   DevFeatures::Unset,
        }
    }

    #[test]
    fn parses_dep_info_with_escaped_spaces_and_line_continuations() {
        let base_dir = Path::new("/tmp");
        let dependencies = parse_dep_info_dependencies(
            "target/debug/demo: /tmp/one.rs /tmp/two\\ with\\ spaces.rs \\\n             /tmp/three.rs",
            base_dir,
        );

        assert_eq!(
            dependencies,
            vec![
                PathBuf::from("/tmp/one.rs"),
                PathBuf::from("/tmp/two with spaces.rs"),
                PathBuf::from("/tmp/three.rs"),
            ]
        );
    }

    fn set_modified_time(path: &Path, seconds: u64) {
        fs::File::options()
            .write(true)
            .open(path)
            .expect("open file to set timestamp")
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
            .expect("set modification time");
    }

    fn fresh_test_target(workspace_root: &Path, name: &str) -> BevyTarget {
        let manifest_path = workspace_root.join(CARGO_MANIFEST_FILE);
        let target = test_target(workspace_root, &manifest_path, name);
        let src_path = workspace_root.join("src/main.rs");
        let lock_path = workspace_root.join(CARGO_LOCK_FILE);
        let binary_path = target.get_binary_path(PROFILE_DEBUG);
        let dep_info_path = dep_info_path(&target, PROFILE_DEBUG);

        fs::create_dir_all(src_path.parent().expect("src parent")).expect("create src dir");
        fs::create_dir_all(binary_path.parent().expect("binary parent"))
            .expect("create target dir");
        fs::write(
            &manifest_path,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write manifest");
        fs::write(&lock_path, "# lock\n").expect("write lock");
        fs::write(&src_path, "fn main() {}\n").expect("write source");
        fs::write(&binary_path, "binary").expect("write binary");
        fs::write(
            &dep_info_path,
            format!("{}: {}\n", binary_path.display(), src_path.display()),
        )
        .expect("write dep info");
        for path in [&manifest_path, &lock_path, &src_path] {
            set_modified_time(path, INPUT_TIMESTAMP_SECONDS);
        }
        set_modified_time(&binary_path, BINARY_TIMESTAMP_SECONDS);
        target
    }

    fn test_record_path(target: &BevyTarget) -> PathBuf {
        let mut path = target.get_binary_path(PROFILE_DEBUG).into_os_string();
        path.push(DEV_FEATURES_RECORD_SUFFIX);
        PathBuf::from(path)
    }

    #[test]
    fn returns_fresh_without_record_when_wanted_dev_features_are_empty() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        assert_eq!(
            check_target_freshness(&target, PROFILE_DEBUG, &[]),
            FreshnessCheckResult::Fresh
        );
    }

    #[test]
    fn returns_stale_when_dependency_is_newer_than_binary() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        set_modified_time(
            &temp_dir.path().join("src/main.rs"),
            RECORD_TIMESTAMP_SECONDS,
        );
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &[]),
            FreshnessCheckResult::Stale(reason)
                if reason.contains("dependency is newer than binary")
        ));
    }

    #[test]
    fn returns_unknown_when_dep_info_is_missing() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        fs::remove_file(dep_info_path(&target, PROFILE_DEBUG)).expect("remove dep info");
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &[]),
            FreshnessCheckResult::Unknown(reason)
                if reason.contains("dep-info file does not exist")
        ));
    }

    #[test]
    fn returns_fresh_when_record_matches_wanted_dev_features() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        let record_path = test_record_path(&target);
        fs::write(&record_path, "bevy/dynamic_linking\napp/dev\n").expect("write record");
        let features = vec!["bevy/dynamic_linking".to_string(), "app/dev".to_string()];
        for timestamp in [BINARY_TIMESTAMP_SECONDS, RECORD_TIMESTAMP_SECONDS] {
            set_modified_time(&record_path, timestamp);
            assert_eq!(
                check_target_freshness(&target, PROFILE_DEBUG, &features),
                FreshnessCheckResult::Fresh
            );
        }
    }

    #[test]
    fn returns_stale_without_record_when_dev_features_are_wanted() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        fs::remove_file(dep_info_path(&target, PROFILE_DEBUG)).expect("remove dep info");
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &["app/dev".to_string()]),
            FreshnessCheckResult::Stale(reason)
                if reason.contains("no dev features record newer than")
                    && reason.contains(&test_record_path(&target).display().to_string())
        ));
    }

    #[test]
    fn returns_stale_when_binary_was_rebuilt_after_matching_dev_features_record() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        let record_path = test_record_path(&target);
        fs::write(&record_path, "app/dev\n").expect("write record");
        set_modified_time(&record_path, BINARY_TIMESTAMP_SECONDS);
        set_modified_time(
            &target.get_binary_path(PROFILE_DEBUG),
            RECORD_TIMESTAMP_SECONDS,
        );
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &["app/dev".to_string()]),
            FreshnessCheckResult::Stale(reason)
                if reason.contains("no dev features record newer than")
                    && reason.contains(&record_path.display().to_string())
        ));
    }

    #[test]
    fn returns_stale_when_recorded_dev_features_have_different_contents_or_order() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        let record_path = test_record_path(&target);
        fs::write(&record_path, "bevy/dynamic_linking\napp/dev\n").expect("write record");
        set_modified_time(&record_path, RECORD_TIMESTAMP_SECONDS);
        fs::remove_file(dep_info_path(&target, PROFILE_DEBUG)).expect("remove dep info");
        for features in [
            vec!["app/other".to_string()],
            vec!["app/dev".to_string(), "bevy/dynamic_linking".to_string()],
        ] {
            assert!(matches!(
                check_target_freshness(&target, PROFILE_DEBUG, &features),
                FreshnessCheckResult::Stale(reason)
                    if reason.contains("bevy/dynamic_linking")
                        && reason.contains("app/dev")
                        && reason.contains(&format!("{features:?}"))
            ));
        }
    }

    #[test]
    fn returns_stale_when_record_has_dev_features_but_wanted_set_is_empty() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        let record_path = test_record_path(&target);
        fs::write(&record_path, "app/dev\n").expect("write record");
        set_modified_time(&record_path, RECORD_TIMESTAMP_SECONDS);
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &[]),
            FreshnessCheckResult::Stale(reason)
                if reason.contains("app/dev") && reason.contains("[]")
        ));
    }

    #[test]
    fn written_dev_features_record_round_trips_for_dotted_binary_names_and_empty_sets() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo.app");
        let binary_path = target.get_binary_path(PROFILE_DEBUG);
        let record_path = test_record_path(&target);
        let features = vec!["bevy/dynamic_linking".to_string(), "app/dev".to_string()];
        write_built_dev_features(&binary_path, &features);
        assert_eq!(
            fs::read_to_string(&record_path).expect("read record"),
            "bevy/dynamic_linking\napp/dev\n"
        );
        assert_eq!(
            check_target_freshness(&target, PROFILE_DEBUG, &features),
            FreshnessCheckResult::Fresh
        );

        write_built_dev_features(&binary_path, &[]);
        assert_eq!(fs::read(&record_path).expect("read empty record"), b"");
        assert_eq!(
            check_target_freshness(&target, PROFILE_DEBUG, &[]),
            FreshnessCheckResult::Fresh
        );
    }

    #[test]
    fn returns_unknown_when_dev_features_record_cannot_be_read_as_text() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        let record_path = test_record_path(&target);
        fs::write(&record_path, [0xff]).expect("write invalid record");
        set_modified_time(&record_path, RECORD_TIMESTAMP_SECONDS);
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &["app/dev".to_string()]),
            FreshnessCheckResult::Unknown(reason)
                if reason.contains("Failed to read dev features record")
                    && reason.contains(&record_path.display().to_string())
        ));
    }

    #[test]
    fn returns_stale_without_reading_dev_features_record_older_than_binary() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        let record_path = test_record_path(&target);
        fs::write(&record_path, [0xff]).expect("write invalid record");
        set_modified_time(&record_path, INPUT_TIMESTAMP_SECONDS);
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &["app/dev".to_string()]),
            FreshnessCheckResult::Stale(reason)
                if reason.contains("no dev features record newer than")
        ));
    }

    #[test]
    fn returns_stale_for_missing_binary_before_reading_dev_features_record() {
        let temp_dir = tempdir().expect("temp dir");
        let target = fresh_test_target(temp_dir.path(), "demo");
        fs::write(test_record_path(&target), [0xff]).expect("write invalid record");
        fs::remove_file(target.get_binary_path(PROFILE_DEBUG)).expect("remove binary");
        assert!(matches!(
            check_target_freshness(&target, PROFILE_DEBUG, &["app/dev".to_string()]),
            FreshnessCheckResult::Stale(reason) if reason.contains("binary does not exist")
        ));
    }
}
