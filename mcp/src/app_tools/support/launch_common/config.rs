use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use bevy_brp_mcp_macros::ResultStruct;
use serde::Deserialize;
use serde::Serialize;

use super::build;
use crate::app_tools::instance_count::InstanceCount;
use crate::app_tools::launch_params::SearchOrder;
use crate::app_tools::support::build_freshness;
use crate::app_tools::support::build_freshness::FreshnessCheckResult;
use crate::app_tools::support::cargo_detector::BevyTarget;
use crate::app_tools::support::cargo_detector::TargetType;
use crate::brp_tools::Port;
use crate::error::Result;

/// Marker type for App launch configuration
#[derive(Clone)]
pub(super) struct App;

/// Marker type for Example launch configuration
#[derive(Clone)]
pub(super) struct Example;

/// Parameterized launch configuration for apps and examples
#[derive(Clone)]
pub(super) struct LaunchConfig<T> {
    target_name:    String,
    profile:        String,
    package_name:   Option<String>,
    port:           Port,
    instance_count: InstanceCount,
    env:            Option<HashMap<String, String>>,
    args:           Option<Vec<String>>,
    _phantom:       PhantomData<T>,
}

impl<T> LaunchConfig<T> {
    const fn new(
        target_name: String,
        profile: String,
        package_name: Option<String>,
        port: Port,
        instance_count: InstanceCount,
        env: Option<HashMap<String, String>>,
        args: Option<Vec<String>>,
    ) -> Self {
        Self {
            target_name,
            profile,
            package_name,
            port,
            instance_count,
            env,
            args,
            _phantom: PhantomData,
        }
    }
}

/// Represents a single launched instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchedInstance {
    pub pid:      u32,
    pub log_file: String,
    pub port:     u16,
}

/// Unified result type for launching Bevy apps and examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct LaunchResult {
    /// Name of the target that was launched (app or example)
    #[to_metadata(skip_if_none)]
    target_name:        Option<String>,
    /// Array of launched instances (1 or more)
    #[to_result]
    instances:          Vec<LaunchedInstance>,
    /// Working directory used for launch
    #[to_metadata(skip_if_none)]
    working_directory:  Option<String>,
    /// Build profile used (debug/release)
    #[to_metadata(skip_if_none)]
    profile:            Option<String>,
    /// Binary path of the launched app (only for apps, not examples)
    #[to_metadata(skip_if_none)]
    binary_path:        Option<String>,
    /// Launch duration in milliseconds
    #[to_metadata(skip_if_none)]
    launch_duration_ms: Option<u128>,
    /// Launch timestamp
    #[to_metadata(skip_if_none)]
    launch_timestamp:   Option<String>,
    /// Workspace information
    #[to_metadata(skip_if_none)]
    workspace:          Option<String>,
    /// Package name containing the example (only for examples)
    #[to_metadata(skip_if_none)]
    package_name:       Option<String>,
    /// Whether the target was launched as an "app" or "example"
    #[to_metadata(skip_if_none)]
    launched_as:        Option<String>,
    /// Available duplicate paths (for disambiguation errors)
    #[to_metadata(skip_if_none)]
    duplicate_paths:    Option<Vec<String>>,
    /// Message template for formatting responses
    #[to_message]
    message_template:   Option<String>,
}

/// Parameters extracted from launch requests
pub struct LaunchParams {
    pub target_name:    String,
    pub profile:        String,
    pub path:           Option<String>,
    pub package_name:   Option<String>,
    pub port:           Port,
    pub instance_count: InstanceCount,
    pub env:            Option<HashMap<String, String>>,
    pub search_order:   SearchOrder,
    pub args:           Option<Vec<String>>,
}

/// Trait for creating launch configs from params
pub(super) trait FromLaunchParams: LaunchConfigTrait + Sized + Send + Sync {
    fn from_params(params: &LaunchParams) -> Self;
}

/// Trait for configuring launch behavior for different target types (app vs example)
pub(super) trait LaunchConfigTrait: Clone {
    const TARGET_TYPE: TargetType;

    fn target_name(&self) -> &str;

    fn profile(&self) -> &str;

    fn package_name(&self) -> Option<&str>;

    fn port(&self) -> Port;

    fn instance_count(&self) -> InstanceCount;

    fn set_port(&mut self, port: Port);

    fn build_command(&self, target: &BevyTarget) -> Command;

    fn extra_log_info(&self, target: &BevyTarget) -> Option<String>;

    fn ensure_built(&self, target: &BevyTarget) -> Result<build::BuildState> {
        if Self::TARGET_TYPE == TargetType::App {
            match build_freshness::check_target_freshness(target, self.profile()) {
                FreshnessCheckResult::Fresh => return Ok(build::BuildState::Fresh),
                FreshnessCheckResult::Stale(reason) => {
                    tracing::debug!(
                        "Lock-free freshness check marked {} '{}' stale: {reason}",
                        Self::TARGET_TYPE,
                        self.target_name(),
                    );
                },
                FreshnessCheckResult::Unknown(reason) => {
                    tracing::debug!(
                        "Lock-free freshness check was inconclusive for {} '{}': {reason}",
                        Self::TARGET_TYPE,
                        self.target_name(),
                    );
                },
            }
        }

        let manifest_dir = build::validate_manifest_directory(&target.manifest_path)?;
        build::run_cargo_build(
            self.target_name(),
            Self::TARGET_TYPE,
            self.profile(),
            manifest_dir,
        )
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
        .map(|((pid, log_file), port)| LaunchedInstance {
            pid,
            log_file: log_file.display().to_string(),
            port: *port,
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
    let target_name_str = config.target_name();
    let message = format!(
        "Successfully launched {instance_count} instance(s) of {target_name_str} on ports {port_range}"
    );

    LaunchResult {
        target_name: Some(config.target_name().to_string()),
        instances,
        working_directory: std::env::current_dir()
            .ok()
            .map(|dir| dir.display().to_string()),
        profile: Some(config.profile().to_string()),
        launch_duration_ms: Some(launch_duration.as_millis()),
        launch_timestamp: Some(chrono::Utc::now().to_rfc3339()),
        workspace,
        package_name: if T::TARGET_TYPE == TargetType::Example {
            Some(target.package_name.clone())
        } else {
            None
        },
        binary_path: if T::TARGET_TYPE == TargetType::App {
            Some(
                target
                    .get_binary_path(config.profile())
                    .display()
                    .to_string(),
            )
        } else {
            None
        },
        launched_as: Some(T::TARGET_TYPE.to_string()),
        duplicate_paths: None,
        message_template: Some(message),
    }
}

impl FromLaunchParams for LaunchConfig<App> {
    fn from_params(params: &LaunchParams) -> Self {
        Self::new(
            params.target_name.clone(),
            params.profile.clone(),
            params.package_name.clone(),
            params.port,
            params.instance_count,
            params.env.clone(),
            params.args.clone(),
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<App> {
    const TARGET_TYPE: TargetType = TargetType::App;

    fn target_name(&self) -> &str { &self.target_name }

    fn profile(&self) -> &str { &self.profile }

    fn package_name(&self) -> Option<&str> { self.package_name.as_deref() }

    fn port(&self) -> Port { self.port }

    fn instance_count(&self) -> InstanceCount { self.instance_count }

    fn set_port(&mut self, port: Port) { self.port = port; }

    fn build_command(&self, target: &BevyTarget) -> Command {
        build::build_app_command(
            &target.get_binary_path(self.profile()),
            Some(self.port),
            self.env.as_ref(),
            self.args.as_deref(),
        )
    }

    fn extra_log_info(&self, _: &BevyTarget) -> Option<String> { None }
}

impl FromLaunchParams for LaunchConfig<Example> {
    fn from_params(params: &LaunchParams) -> Self {
        Self::new(
            params.target_name.clone(),
            params.profile.clone(),
            params.package_name.clone(),
            params.port,
            params.instance_count,
            params.env.clone(),
            params.args.clone(),
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<Example> {
    const TARGET_TYPE: TargetType = TargetType::Example;

    fn target_name(&self) -> &str { &self.target_name }

    fn profile(&self) -> &str { &self.profile }

    fn package_name(&self) -> Option<&str> { self.package_name.as_deref() }

    fn port(&self) -> Port { self.port }

    fn instance_count(&self) -> InstanceCount { self.instance_count }

    fn set_port(&mut self, port: Port) { self.port = port; }

    fn build_command(&self, _: &BevyTarget) -> Command {
        build::build_cargo_example_command(
            &self.target_name,
            self.profile(),
            Some(self.port),
            self.env.as_ref(),
            self.args.as_deref(),
        )
    }

    fn extra_log_info(&self, target: &BevyTarget) -> Option<String> {
        Some(format!("Package: {}", target.package_name))
    }
}
