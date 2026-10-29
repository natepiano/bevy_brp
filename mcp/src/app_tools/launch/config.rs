use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use bevy_brp_mcp_macros::ResultStruct;
use serde::Deserialize;
use serde::Serialize;

use super::build;
use super::build::BuildScope;
use super::build::BuildState;
use super::build_freshness;
use super::build_freshness::FreshnessCheckResult;
use crate::app_tools::constants::CARGO_MANIFEST_FILE;
use crate::app_tools::constants::PROFILE_RELEASE;
use crate::app_tools::instance_count::InstanceCount;
use crate::app_tools::launch_params::SearchOrder;
use crate::app_tools::targets::BevyTarget;
use crate::app_tools::targets::DevFeatures;
use crate::app_tools::targets::TargetType;
use crate::brp_tools::Port;
use crate::error::Result;

/// Marker type for `App` launch configuration
#[derive(Clone)]
pub(super) struct App;

/// Marker type for `Example` launch configuration
#[derive(Clone)]
pub(super) struct Example;

/// Parameterized launch configuration for apps and examples
#[derive(Clone)]
pub(super) struct LaunchConfig<T> {
    target:         String,
    profile:        String,
    package:        Option<String>,
    port:           Port,
    instance_count: InstanceCount,
    env:            Option<HashMap<String, String>>,
    arguments:      Option<Vec<String>>,
    phantom_data:   PhantomData<T>,
}

impl<T> LaunchConfig<T> {
    const fn new(
        target: String,
        profile: String,
        package: Option<String>,
        port: Port,
        instance_count: InstanceCount,
        env: Option<HashMap<String, String>>,
        arguments: Option<Vec<String>>,
    ) -> Self {
        Self {
            target,
            profile,
            package,
            port,
            instance_count,
            env,
            arguments,
            phantom_data: PhantomData,
        }
    }
}

/// Represents a single launched instance
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LaunchedInstance {
    pid:      u32,
    log_file: String,
    port:     u16,
}

/// Unified result type for launching Bevy apps and examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct LaunchResult {
    /// Name of the target that was launched (app or example)
    #[serde(rename = "target_name")]
    #[to_metadata(skip_if_none)]
    target:            Option<String>,
    /// Array of launched instances (1 or more)
    #[to_result]
    instances:         Vec<LaunchedInstance>,
    /// Working directory of the launched processes: the target package's manifest directory
    #[to_metadata(skip_if_none)]
    working_directory: Option<String>,
    /// Build profile used (debug/release)
    #[to_metadata(skip_if_none)]
    profile:           Option<String>,
    /// Binary path of the launched app or example
    #[to_metadata(skip_if_none)]
    binary_path:       Option<String>,
    /// Launch duration in milliseconds
    #[serde(rename = "launch_duration_ms")]
    #[to_metadata(skip_if_none)]
    duration_ms:       Option<u128>,
    /// Launch timestamp
    #[serde(rename = "launch_timestamp")]
    #[to_metadata(skip_if_none)]
    timestamp:         Option<String>,
    /// Workspace information
    #[to_metadata(skip_if_none)]
    workspace:         Option<String>,
    /// Package name containing the example (only for examples)
    #[serde(rename = "package_name")]
    #[to_metadata(skip_if_none)]
    package:           Option<String>,
    /// Whether the target was launched as an "app" or "example"
    #[to_metadata(skip_if_none)]
    launched_as:       Option<String>,
    /// Available duplicate paths (for disambiguation errors)
    #[to_metadata(skip_if_none)]
    duplicate_paths:   Option<Vec<String>>,
    /// Message template for formatting responses
    #[to_message]
    message_template:  Option<String>,
}

/// Parameters extracted from launch requests
pub(crate) struct LaunchParams {
    pub target:         String,
    pub profile:        String,
    pub path:           Option<String>,
    pub package:        Option<String>,
    pub port:           Port,
    pub instance_count: InstanceCount,
    pub env:            Option<HashMap<String, String>>,
    pub search_order:   SearchOrder,
    pub args:           Option<Vec<String>>,
}

/// Trait for configuring launch behavior for different target types (app vs example)
pub(super) trait LaunchConfigTrait: Clone {
    const TARGET_TYPE: TargetType;

    fn target(&self) -> &str;

    fn profile(&self) -> &str;

    fn package(&self) -> Option<&str>;

    fn port(&self) -> Port;

    fn instance_count(&self) -> InstanceCount;

    fn set_port(&mut self, port: Port);

    fn build_command(&self, target: &BevyTarget) -> Command;

    fn extra_log_info(&self, target: &BevyTarget) -> Option<String>;

    fn ensure_built(&self, target: &BevyTarget, scope: BuildScope) -> Result<BuildState> {
        let features =
            dev_features_for_profile(&target.dev_features, self.profile(), &target.workspace_root)?;
        if Self::TARGET_TYPE == TargetType::App {
            let freshness =
                build_freshness::check_target_freshness(target, self.profile(), features);
            match &freshness {
                FreshnessCheckResult::Fresh => {},
                FreshnessCheckResult::Stale(reason) => tracing::debug!(
                    "Lock-free freshness check marked {} '{}' stale: {reason}",
                    Self::TARGET_TYPE,
                    self.target(),
                ),
                FreshnessCheckResult::Unknown(reason) => tracing::debug!(
                    "Lock-free freshness check was inconclusive for {} '{}': {reason}",
                    Self::TARGET_TYPE,
                    self.target(),
                ),
            }
            if matches!(freshness, FreshnessCheckResult::Fresh) {
                return Ok(BuildState::Fresh);
            }
        }

        let manifest_dir = build::validate_manifest_directory(&target.manifest)?;
        let build_state = build::run_cargo_build(
            self.target(),
            Self::TARGET_TYPE,
            self.profile(),
            manifest_dir,
            &target.workspace_root,
            scope,
            features,
        )?;
        if Self::TARGET_TYPE == TargetType::App {
            build_freshness::write_built_dev_features(
                &target.get_binary_path(self.profile()),
                features,
            );
        }
        Ok(build_state)
    }
}

pub(super) fn dev_features_for_profile<'a>(
    dev_features: &'a DevFeatures,
    profile: &str,
    workspace_root: &Path,
) -> Result<&'a [String]> {
    if profile == PROFILE_RELEASE {
        return Ok(&[]);
    }

    dev_features.for_launch(&workspace_root.join(CARGO_MANIFEST_FILE))
}

impl From<&LaunchParams> for LaunchConfig<App> {
    fn from(params: &LaunchParams) -> Self {
        Self::new(
            params.target.clone(),
            params.profile.clone(),
            params.package.clone(),
            params.port,
            params.instance_count,
            params.env.clone(),
            params.args.clone(),
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<App> {
    const TARGET_TYPE: TargetType = TargetType::App;

    fn target(&self) -> &str { &self.target }

    fn profile(&self) -> &str { &self.profile }

    fn package(&self) -> Option<&str> { self.package.as_deref() }

    fn port(&self) -> Port { self.port }

    fn instance_count(&self) -> InstanceCount { self.instance_count }

    fn set_port(&mut self, port: Port) { self.port = port; }

    fn build_command(&self, target: &BevyTarget) -> Command {
        build::build_app_command(
            &target.get_binary_path(self.profile()),
            Some(self.port),
            self.env.as_ref(),
            self.arguments.as_deref(),
        )
    }

    fn extra_log_info(&self, _: &BevyTarget) -> Option<String> { None }
}

impl From<&LaunchParams> for LaunchConfig<Example> {
    fn from(params: &LaunchParams) -> Self {
        Self::new(
            params.target.clone(),
            params.profile.clone(),
            params.package.clone(),
            params.port,
            params.instance_count,
            params.env.clone(),
            params.args.clone(),
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<Example> {
    const TARGET_TYPE: TargetType = TargetType::Example;

    fn target(&self) -> &str { &self.target }

    fn profile(&self) -> &str { &self.profile }

    fn package(&self) -> Option<&str> { self.package.as_deref() }

    fn port(&self) -> Port { self.port }

    fn instance_count(&self) -> InstanceCount { self.instance_count }

    fn set_port(&mut self, port: Port) { self.port = port; }

    fn build_command(&self, target: &BevyTarget) -> Command {
        build::build_app_command(
            &target.get_binary_path(self.profile()),
            Some(self.port),
            self.env.as_ref(),
            self.arguments.as_deref(),
        )
    }

    fn extra_log_info(&self, target: &BevyTarget) -> Option<String> {
        Some(format!("Package: {}", target.package_name))
    }
}

pub(super) fn build_launch_result<T: LaunchConfigTrait>(
    all_pids: Vec<u32>,
    all_log_files: Vec<PathBuf>,
    all_ports: Vec<u16>,
    config: &T,
    target: &BevyTarget,
    launch_start: Instant,
) -> LaunchResult {
    let launch_duration = launch_start.elapsed();

    let instances: Vec<LaunchedInstance> = all_pids
        .into_iter()
        .zip(all_log_files.iter())
        .zip(all_ports.iter())
        .map(|((process_id, log_file), port)| LaunchedInstance {
            pid:      process_id,
            log_file: log_file.display().to_string(),
            port:     *port,
        })
        .collect();

    let workspace = target
        .workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(String::from);

    let port_range = if all_ports.len() == 1 {
        all_ports[0].to_string()
    } else {
        format!("{}-{}", all_ports[0], all_ports[all_ports.len() - 1])
    };

    let instance_count = all_ports.len();
    let message = format!(
        "Successfully launched {instance_count} instance(s) of {} on ports {port_range}",
        config.target()
    );

    LaunchResult {
        target: Some(config.target().to_string()),
        instances,
        working_directory: build::validate_manifest_directory(&target.manifest)
            .ok()
            .map(|manifest_dir| manifest_dir.display().to_string()),
        profile: Some(config.profile().to_string()),
        duration_ms: Some(launch_duration.as_millis()),
        timestamp: Some(chrono::Utc::now().to_rfc3339()),
        workspace,
        package: if T::TARGET_TYPE == TargetType::Example {
            Some(target.package_name.clone())
        } else {
            None
        },
        binary_path: Some(
            target
                .get_binary_path(config.profile())
                .display()
                .to_string(),
        ),
        launched_as: Some(T::TARGET_TYPE.to_string()),
        duplicate_paths: None,
        message_template: Some(message),
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "tests should panic on unexpected values"
)]
mod tests {
    use std::path::Path;

    use super::dev_features_for_profile;
    use crate::app_tools::constants::CARGO_MANIFEST_FILE;
    use crate::app_tools::constants::PROFILE_DEBUG;
    use crate::app_tools::constants::PROFILE_RELEASE;
    use crate::app_tools::constants::WORKSPACE_DEV_FEATURES_LABEL;
    use crate::app_tools::targets::DevFeatures;

    #[test]
    fn non_release_profiles_use_workspace_dev_features() {
        let dev_features = DevFeatures::Listed(vec![
            "bevy/dynamic_linking".to_string(),
            "app/dev".to_string(),
        ]);
        for profile in [PROFILE_DEBUG, "custom"] {
            let features =
                dev_features_for_profile(&dev_features, profile, Path::new("/workspace"))
                    .expect("valid workspace dev features");
            assert_eq!(features, ["bevy/dynamic_linking", "app/dev"]);
        }
    }

    #[test]
    fn release_ignores_listed_and_malformed_dev_features() {
        for dev_features in [
            DevFeatures::Listed(vec!["bevy/dynamic_linking".to_string()]),
            DevFeatures::Malformed("a string: \"bevy/dynamic_linking\"".to_string()),
        ] {
            let features =
                dev_features_for_profile(&dev_features, PROFILE_RELEASE, Path::new("/workspace"))
                    .expect("release must ignore workspace dev features");
            assert_eq!(features, Vec::<String>::new());
        }
    }

    #[test]
    fn non_release_malformed_dev_features_report_workspace_manifest() {
        let dev_features = DevFeatures::Malformed("a number: 42".to_string());
        let workspace_manifest = Path::new("/workspace").join(CARGO_MANIFEST_FILE);
        for profile in [PROFILE_DEBUG, "custom"] {
            let error = dev_features_for_profile(&dev_features, profile, Path::new("/workspace"))
                .expect_err("malformed workspace dev features must fail non-release launches");
            let message = error.to_string();
            assert!(
                message.contains(&workspace_manifest.display().to_string()),
                "{message}"
            );
            assert!(message.contains(WORKSPACE_DEV_FEATURES_LABEL), "{message}");
            assert!(message.contains("a number: 42"), "{message}");
        }
    }
}
