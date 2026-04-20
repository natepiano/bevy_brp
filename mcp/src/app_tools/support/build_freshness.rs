use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use super::cargo_detector::BevyTarget;
use crate::error::Error;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FreshnessCheckResult {
    Fresh,
    Stale(String),
    Unknown(String),
}

pub(super) fn check_target_freshness(target: &BevyTarget, profile: &str) -> FreshnessCheckResult {
    if !target.is_app() {
        return FreshnessCheckResult::Unknown(
            "lock-free freshness checks are only supported for app binaries".to_string(),
        );
    }

    match try_check_target_freshness(target, profile) {
        Ok(result) => result,
        Err(error) => FreshnessCheckResult::Unknown(format!("{error}")),
    }
}

fn try_check_target_freshness(target: &BevyTarget, profile: &str) -> Result<FreshnessCheckResult> {
    let binary_path = target.get_binary_path(profile);
    if !binary_path.exists() {
        return Ok(FreshnessCheckResult::Stale(format!(
            "binary does not exist: {}",
            binary_path.display()
        )));
    }

    let binary_mtime = file_modified_time(&binary_path)?;
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

fn dep_info_path(target: &BevyTarget, profile: &str) -> PathBuf {
    target.get_binary_path(profile).with_extension("d")
}

fn extra_fingerprint_inputs(target: &BevyTarget) -> Vec<PathBuf> {
    let mut inputs = vec![target.manifest_path.clone()];

    let workspace_manifest = target.workspace_root.join("Cargo.toml");
    if workspace_manifest != target.manifest_path {
        inputs.push(workspace_manifest);
    }

    inputs.push(target.workspace_root.join("Cargo.lock"));
    inputs.extend(find_cargo_config_files(
        &target.manifest_path,
        &target.workspace_root,
    ));

    if let Some(package_dir) = target.manifest_path.parent() {
        inputs.push(package_dir.join("build.rs"));
    }

    inputs.push(target.workspace_root.join("rust-toolchain.toml"));
    inputs.push(target.workspace_root.join("rust-toolchain"));

    inputs
}

fn find_cargo_config_files(manifest_path: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    let mut configs = Vec::new();

    let Some(mut current_dir) = manifest_path.parent() else {
        return configs;
    };

    loop {
        configs.push(current_dir.join(".cargo").join("config.toml"));
        configs.push(current_dir.join(".cargo").join("config"));

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MissingInputPolicy {
    Ignore,
    TreatAsStale,
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
    let mut escaped = false;

    for ch in dependency_text.chars() {
        if escaped {
            match ch {
                '\n' | '\r' => {},
                _ => current.push(ch),
            }
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
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
    use std::thread;
    use std::time::Duration;

    use tempfile::tempdir;

    use super::*;
    use crate::app_tools::support::cargo_detector::TargetType;

    fn test_target(workspace_root: &Path, manifest_path: &Path, name: &str) -> BevyTarget {
        BevyTarget {
            name:           name.to_string(),
            target_type:    TargetType::App,
            package_name:   "pkg".to_string(),
            workspace_root: workspace_root.to_path_buf(),
            manifest_path:  manifest_path.to_path_buf(),
            relative_path:  PathBuf::new(),
            source_path:    PathBuf::new(),
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

    #[test]
    fn returns_fresh_when_binary_is_newer_than_inputs() {
        let temp_dir = tempdir().expect("temp dir");
        let workspace_root = temp_dir.path();
        let manifest_path = workspace_root.join("Cargo.toml");
        let src_path = workspace_root.join("src/main.rs");
        let binary_path = workspace_root.join("target/debug/demo");
        let dep_info_path = workspace_root.join("target/debug/demo.d");

        fs::create_dir_all(src_path.parent().expect("src parent")).expect("create src dir");
        fs::create_dir_all(binary_path.parent().expect("binary parent"))
            .expect("create target dir");
        fs::write(
            &manifest_path,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write manifest");
        fs::write(workspace_root.join("Cargo.lock"), "# lock\n").expect("write lock");
        fs::write(&src_path, "fn main() {}\n").expect("write source");

        thread::sleep(Duration::from_millis(20));
        fs::write(&binary_path, "binary").expect("write binary");
        fs::write(
            &dep_info_path,
            format!("{}: {}\n", binary_path.display(), src_path.display()),
        )
        .expect("write dep info");

        let target = test_target(workspace_root, &manifest_path, "demo");
        assert_eq!(
            check_target_freshness(&target, "debug"),
            FreshnessCheckResult::Fresh
        );
    }

    #[test]
    fn returns_stale_when_dependency_is_newer_than_binary() {
        let temp_dir = tempdir().expect("temp dir");
        let workspace_root = temp_dir.path();
        let manifest_path = workspace_root.join("Cargo.toml");
        let src_path = workspace_root.join("src/main.rs");
        let binary_path = workspace_root.join("target/debug/demo");
        let dep_info_path = workspace_root.join("target/debug/demo.d");

        fs::create_dir_all(src_path.parent().expect("src parent")).expect("create src dir");
        fs::create_dir_all(binary_path.parent().expect("binary parent"))
            .expect("create target dir");
        fs::write(
            &manifest_path,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write manifest");
        fs::write(workspace_root.join("Cargo.lock"), "# lock\n").expect("write lock");
        fs::write(&binary_path, "binary").expect("write binary");

        thread::sleep(Duration::from_millis(20));
        fs::write(&src_path, "fn main() {}\n").expect("write source");
        fs::write(
            &dep_info_path,
            format!("{}: {}\n", binary_path.display(), src_path.display()),
        )
        .expect("write dep info");

        let target = test_target(workspace_root, &manifest_path, "demo");
        assert!(matches!(
            check_target_freshness(&target, "debug"),
            FreshnessCheckResult::Stale(reason)
                if reason.contains("dependency is newer than binary")
        ));
    }

    #[test]
    fn returns_unknown_when_dep_info_is_missing() {
        let temp_dir = tempdir().expect("temp dir");
        let workspace_root = temp_dir.path();
        let manifest_path = workspace_root.join("Cargo.toml");
        let binary_path = workspace_root.join("target/debug/demo");

        fs::create_dir_all(binary_path.parent().expect("binary parent"))
            .expect("create target dir");
        fs::write(
            &manifest_path,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write manifest");
        fs::write(&binary_path, "binary").expect("write binary");

        let target = test_target(workspace_root, &manifest_path, "demo");
        assert!(matches!(
            check_target_freshness(&target, "debug"),
            FreshnessCheckResult::Unknown(reason)
                if reason.contains("dep-info file does not exist")
        ));
    }
}
