use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use bevy_brp_mcp_macros::ResultStruct;
use error_stack::Report;
use serde::Deserialize;
use serde::Serialize;

use super::build_freshness;
use super::build_freshness::FreshnessCheckResult;
use super::cargo_detector::BevyTarget;
use super::process;
use crate::app_tools::launch_params::LaunchBevyBinaryParams;
use crate::app_tools::launch_params::SearchOrder;
use crate::error::Error;
use crate::error::Result;

/// Marker type for App launch configuration
#[derive(Clone)]
struct App;

/// Marker type for Example launch configuration
#[derive(Clone)]
struct Example;

/// Parameterized launch configuration for apps and examples
#[derive(Clone)]
struct LaunchConfig<T> {
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
    /// Create a new launch configuration
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
#[allow(clippy::too_many_arguments)]
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

use crate::app_tools::instance_count::InstanceCount;
use crate::brp_tools::BRP_EXTRAS_PORT_ENV_VAR;
use crate::brp_tools::Port;

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

/// Trait for converting typed parameters to `LaunchParams`
pub trait ToLaunchParams: Send + Sync {
    /// Convert to `LaunchParams` with the given default profile
    fn to_launch_params(&self, default_profile: &str) -> LaunchParams;
}

/// Trait for creating launch configs from params
trait FromLaunchParams: LaunchConfigTrait + Sized + Send + Sync {
    /// Create a new instance from launch parameters
    fn from_params(params: &LaunchParams) -> Self;
}

/// Trait for configuring launch behavior for different target types (app vs example)
trait LaunchConfigTrait: Clone {
    /// The target type constant (App or Example)
    const TARGET_TYPE: TargetType;

    /// Get the name of the target being launched
    fn target_name(&self) -> &str;

    /// Get the build profile ("debug" or "release")
    fn profile(&self) -> &str;

    /// Get the optional package name for disambiguation
    fn package_name(&self) -> Option<&str>;

    /// Get the BRP port
    fn port(&self) -> Port;

    /// Get the instance count for launching multiple instances
    fn instance_count(&self) -> InstanceCount;

    /// Set the port (needed for multi-instance launches)
    fn set_port(&mut self, port: Port);

    /// Build the command to execute
    fn build_command(&self, target: &BevyTarget) -> Command;

    /// Get any extra log info specific to this target type
    fn extra_log_info(&self, target: &BevyTarget) -> Option<String>;

    /// Ensure the target is built, blocking until compilation completes if needed
    /// Returns the build state indicating whether it was fresh, rebuilt, or not found
    fn ensure_built(&self, target: &BevyTarget) -> Result<BuildState> {
        if Self::TARGET_TYPE == TargetType::App {
            match build_freshness::check_target_freshness(target, self.profile()) {
                FreshnessCheckResult::Fresh => return Ok(BuildState::Fresh),
                FreshnessCheckResult::Stale(reason) => {
                    tracing::debug!(
                        "Lock-free freshness check marked {} '{}' stale: {}",
                        Self::TARGET_TYPE,
                        self.target_name(),
                        reason
                    );
                },
                FreshnessCheckResult::Unknown(reason) => {
                    tracing::debug!(
                        "Lock-free freshness check was inconclusive for {} '{}': {}",
                        Self::TARGET_TYPE,
                        self.target_name(),
                        reason
                    );
                },
            }
        }

        let manifest_dir = validate_manifest_directory(&target.manifest_path)?;
        run_cargo_build(
            self.target_name(),
            Self::TARGET_TYPE,
            self.profile(),
            manifest_dir,
        )
    }
}

/// Validates and extracts the manifest directory from a manifest path
fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path> {
    manifest_path.parent().ok_or_else(|| {
        error_stack::Report::new(Error::FileOrPathNotFound(
            "Invalid manifest path".to_string(),
        ))
        .attach("No parent directory found")
        .attach(format!("Path: {}", manifest_path.display()))
    })
}

/// Sets BRP-related environment variables on a command
///
/// Currently sets:
/// - `BRP_PORT`: When a port is provided, sets this environment variable for `bevy_brp_extras` to
///   read
fn set_brp_env_vars(cmd: &mut Command, port: Option<Port>) {
    if let Some(port) = port {
        cmd.env(BRP_EXTRAS_PORT_ENV_VAR, port.to_string());
    }
}

/// Sets user-specified environment variables on a command
fn set_user_env_vars(cmd: &mut Command, env: Option<&HashMap<String, String>>) {
    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }
}

/// Setup logging for launch operations and return log file handles
fn setup_launch_logging(
    name: &str,
    target_type: TargetType,
    profile: &str,
    binary_path: &Path,
    manifest_dir: &Path,
    port: Port,
    extra_log_info: Option<&str>,
) -> Result<(PathBuf, std::fs::File)> {
    use super::logging;

    // Create log file
    let (log_file_path, _) =
        logging::create_log_file(name, target_type, profile, binary_path, manifest_dir, port)
            .map_err(|e| Error::tool_call_failed(format!("Failed to create log file: {e}")))?;

    // Add extra info to log file if provided
    if let Some(extra_info) = extra_log_info {
        logging::append_to_log_file(&log_file_path, &format!("{extra_info}\n"))
            .map_err(|e| Error::tool_call_failed(format!("Failed to append to log file: {e}")))?;
    }

    // Open log file for stdout/stderr redirection
    let log_file_for_redirect =
        logging::open_log_file_for_redirect(&log_file_path).map_err(|e| {
            Error::tool_call_failed(format!("Failed to open log file for redirect: {e}"))
        })?;

    Ok((log_file_path, log_file_for_redirect))
}

/// Build cargo command for running examples
fn build_cargo_example_command(
    example_name: &str,
    profile: &str,
    port: Option<Port>,
    env: Option<&HashMap<String, String>>,
    args: Option<&[String]>,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example").arg(example_name);

    // Add profile flag if release
    if profile == "release" {
        cmd.arg("--release");
    }

    // Separate cargo args from app args with `--`
    if let Some(user_args) = args {
        cmd.arg("--").args(user_args);
    }

    // Set BRP-related environment variables
    set_brp_env_vars(&mut cmd, port);

    // Set user-specified environment variables
    set_user_env_vars(&mut cmd, env);

    cmd
}

/// Build command for running app binaries
fn build_app_command(
    binary_path: &Path,
    port: Option<Port>,
    env: Option<&HashMap<String, String>>,
    args: Option<&[String]>,
) -> Command {
    let mut cmd = Command::new(binary_path);
    if let Some(user_args) = args {
        cmd.args(user_args);
    }
    set_brp_env_vars(&mut cmd, port);
    set_user_env_vars(&mut cmd, env);
    cmd
}

use super::cargo_detector::TargetType;

/// Represents the state of a build target after cargo build
#[derive(Debug, Clone, Copy)]
enum BuildState {
    NotFound,
    Fresh,
    Rebuilt,
}

/// Build a cargo command for the given target
fn build_cargo_command(
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    manifest_dir: &Path,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(manifest_dir);
    cmd.arg("build");

    // Add target-specific arguments
    target_type.add_cargo_args(&mut cmd, target_name);

    // Add profile flag if release
    if profile == "release" {
        cmd.arg("--release");
    }

    // Use JSON output to track freshness
    cmd.arg("--message-format=json");

    cmd
}

/// Execute cargo build command and validate output
fn execute_build_command(
    cmd: &mut Command,
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    manifest_dir: &Path,
) -> Result<std::process::Output> {
    use tracing::debug;

    debug!(
        "Running cargo build for {} '{}' with args: {:?}",
        target_type, target_name, cmd
    );

    let output = cmd.output().map_err(|e| {
        Error::ProcessManagement(format!(
            "Failed to run cargo build for {target_type} '{target_name}' (profile: {profile}, dir: {}): {e}",
            manifest_dir.display()
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ProcessManagement(format!(
            "Cargo build failed for {target_type} '{target_name}' (profile: {profile}, dir: {}): {stderr}",
            manifest_dir.display()
        ))
        .into());
    }

    Ok(output)
}

/// Parse cargo build JSON output to determine build state
fn parse_build_output(stdout: &[u8], target_name: &str) -> BuildState {
    use serde_json::Value;

    let stdout_str = String::from_utf8_lossy(stdout);

    for line in stdout_str.lines() {
        if let Ok(json) = serde_json::from_str::<Value>(line)
            && let Some(target) = json.get("target")
            && let Some(name) = target.get("name")
            && name.as_str() == Some(target_name)
        {
            return json
                .get("fresh")
                .and_then(serde_json::Value::as_bool)
                .map_or(BuildState::Rebuilt, |is_fresh| {
                    if is_fresh {
                        BuildState::Fresh
                    } else {
                        BuildState::Rebuilt
                    }
                });
        }
    }

    BuildState::NotFound
}

/// Log the build result based on build state
fn log_build_result(build_state: BuildState, target_name: &str, target_type: TargetType) {
    use tracing::debug;
    use tracing::info;

    match build_state {
        BuildState::NotFound => {
            debug!(
                "Target '{}' not found in build output, assuming it was built",
                target_name
            );
        },
        BuildState::Fresh => {
            debug!("{} '{}' was already up to date", target_type, target_name);
        },
        BuildState::Rebuilt => {
            info!("{} '{}' was built successfully", target_type, target_name);
        },
    }
}

/// Run cargo build for a target and block until completion
fn run_cargo_build(
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    manifest_dir: &Path,
) -> Result<BuildState> {
    let mut cmd = build_cargo_command(target_name, target_type, profile, manifest_dir);
    let output = execute_build_command(&mut cmd, target_name, target_type, profile, manifest_dir)?;
    let build_state = parse_build_output(&output.stdout, target_name);
    log_build_result(build_state, target_name, target_type);

    Ok(build_state)
}

/// Build unified result from collected vectors
fn build_launch_result<T: LaunchConfigTrait>(
    all_pids: Vec<u32>,
    all_log_files: Vec<PathBuf>,
    all_ports: Vec<u16>,
    config: &T,
    target: &BevyTarget,
    launch_start: std::time::Instant,
) -> LaunchResult {
    let launch_duration = launch_start.elapsed();

    // Build instances array
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

    // Create port range string for message
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

/// Prepare the launch environment including command, logging, and directory setup
fn prepare_launch_environment<T: LaunchConfigTrait>(
    config: &T,
    target: &BevyTarget,
) -> Result<(Command, PathBuf, PathBuf, std::fs::File)> {
    // Get manifest directory
    let manifest_dir = validate_manifest_directory(&target.manifest_path)?;

    // Build command
    let cmd = config.build_command(target);

    // Setup logging
    let (log_file_path, log_file_for_redirect) = setup_launch_logging(
        config.target_name(),
        T::TARGET_TYPE,
        config.profile(),
        &PathBuf::from(format!("{cmd:?}")), // Convert command to path for logging
        manifest_dir,
        config.port(),
        config.extra_log_info(target).as_deref(),
    )?;

    Ok((
        cmd,
        manifest_dir.to_path_buf(),
        log_file_path,
        log_file_for_redirect,
    ))
}

/// Validate that the port range for multi-instance launching is within bounds
fn validate_port_range(base_port: u16, instance_count: u16) -> Result<()> {
    use crate::brp_tools::MAX_VALID_PORT;

    if base_port.saturating_add(instance_count.saturating_sub(1)) > MAX_VALID_PORT {
        return Err(Error::tool_call_failed(format!(
            "Port range {base_port} to {} exceeds maximum valid port {MAX_VALID_PORT}",
            base_port.saturating_add(instance_count.saturating_sub(1)),
        ))
        .into());
    }
    Ok(())
}

/// Launch multiple instances of a target
fn launch_instances<T: LaunchConfigTrait>(
    config: &T,
    target: &BevyTarget,
    instance_count: u16,
    base_port: u16,
) -> Result<(Vec<u32>, Vec<PathBuf>, Vec<u16>)> {
    let mut all_pids = Vec::new();
    let mut all_log_files = Vec::new();
    let mut all_ports = Vec::new();

    for i in 0..instance_count {
        let port = Port(base_port.saturating_add(i));

        // Create a modified config with the updated port for this instance
        let mut instance_config = config.clone();
        instance_config.set_port(port);

        // Prepare launch environment with the instance-specific config
        let (cmd, manifest_dir, log_file_path, log_file_for_redirect) =
            prepare_launch_environment(&instance_config, target)?;

        // Use launch_detached_process for proper zombie prevention and process group isolation
        let pid = process::launch_detached_process(
            &cmd,
            &manifest_dir,
            log_file_for_redirect,
            config.target_name(),
        )?;

        all_pids.push(pid);
        all_log_files.push(log_file_path);
        all_ports.push(port.0);
    }

    Ok((all_pids, all_log_files, all_ports))
}

/// Handle target discovery errors and convert to appropriate error types
fn handle_target_discovery_error(error: Report<Error>) -> Report<Error> {
    // Check if this is a structured error that should be preserved
    if let Error::Structured { .. } = error.current_context() {
        // Preserve structured errors as-is
        return error;
    }

    // Convert other errors to ToolError with details
    let error_message = format!("{}", error.current_context());
    let details = serde_json::json!({
        "error": error_message,
        "error_chain": format!("{:?}", error)
    });
    Error::tool_call_failed_with_details(error_message, details).into()
}

/// Launch a Bevy target using unified search: tries one target type first, then the other.
/// The `search_order` parameter determines which type is tried first.
pub fn launch_bevy_target(
    typed_params: LaunchBevyBinaryParams,
    roots: Vec<PathBuf>,
    default_profile: &'static str,
) -> Result<LaunchResult> {
    use super::errors::AvailableTarget;
    use super::errors::UnifiedTargetNotFoundError;
    use super::scanning;

    let params = typed_params.to_launch_params(default_profile);

    // Use `path` as search root override if provided, otherwise use MCP workspace roots
    let search_roots = params
        .path
        .as_ref()
        .map_or(roots, |path| vec![PathBuf::from(path)]);

    // Determine search order
    let (first, second) = match params.search_order {
        SearchOrder::App => (TargetType::App, TargetType::Example),
        SearchOrder::Example => (TargetType::Example, TargetType::App),
    };

    // When a user-specified path is provided, post-filter targets to only those
    // whose manifest directory is under that path. Cargo metadata resolves workspace
    // members up to the workspace root, which can expand the scope beyond what the user intended.
    let scope_path = params.path.as_ref().map(PathBuf::from);

    // Try first type
    let mut first_targets =
        scanning::find_all_targets_by_name(&params.target_name, Some(first), &search_roots);
    if let Some(ref scope) = scope_path {
        first_targets = scanning::filter_targets_by_path_scope(first_targets, scope);
    }
    if !first_targets.is_empty() {
        return launch_found_target(first, first_targets, &params, &search_roots);
    }

    // Try second type
    let mut second_targets =
        scanning::find_all_targets_by_name(&params.target_name, Some(second), &search_roots);
    if let Some(ref scope) = scope_path {
        second_targets = scanning::filter_targets_by_path_scope(second_targets, scope);
    }
    if !second_targets.is_empty() {
        return launch_found_target(second, second_targets, &params, &search_roots);
    }

    // Neither found — build enriched error with ALL available targets
    let mut all_targets = scanning::collect_all_bevy_targets(&search_roots);
    if let Some(ref scope) = scope_path {
        all_targets = scanning::filter_targets_by_path_scope(all_targets, scope);
    }
    let available: Vec<AvailableTarget> = all_targets
        .into_iter()
        .map(|t| AvailableTarget {
            name: t.name,
            kind: t.target_type.to_string(),
            path: t.relative_path.to_string_lossy().to_string(),
        })
        .collect();

    let error = UnifiedTargetNotFoundError::new(params.target_name, available);
    Err(Error::Structured {
        result: Box::new(error),
    }
    .into())
}

/// Launch a target that was found by name, handling disambiguation and dispatch
fn launch_found_target(
    target_type: TargetType,
    cached_targets: Vec<BevyTarget>,
    params: &LaunchParams,
    roots: &[PathBuf],
) -> Result<LaunchResult> {
    match target_type {
        TargetType::App => {
            let config = LaunchConfig::<App>::from_params(params);
            // Pass cached_targets through find_and_validate_target path
            launch_target_with_cached(&config, roots, cached_targets)
        },
        TargetType::Example => {
            let config = LaunchConfig::<Example>::from_params(params);
            launch_target_with_cached(&config, roots, cached_targets)
        },
    }
}

/// Generic function to launch a Bevy target with pre-cached scan results
fn launch_target_with_cached<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
    cached_targets: Vec<BevyTarget>,
) -> Result<LaunchResult> {
    use std::time::Instant;

    use tracing::debug;

    let launch_start = Instant::now();

    debug!("Environment variable: BRP_EXTRAS_PORT={}", config.port());

    // Find and validate the target using cached scan results
    let target = find_and_validate_target_with_cache(config, search_paths, cached_targets)
        .map_err(handle_target_discovery_error)?;

    // Ensure the target is built (blocks until compilation completes if needed)
    let build_state = config.ensure_built(&target)?;
    match build_state {
        BuildState::Fresh => debug!("Target was already up to date, launching immediately"),
        BuildState::Rebuilt => debug!("Target was rebuilt before launch"),
        BuildState::NotFound => {
            use tracing::warn;
            warn!("Target not found in build output but build succeeded");
        },
    }

    let instance_count = *config.instance_count();
    let base_port = *config.port();

    validate_port_range(base_port, instance_count)?;

    let (all_pids, all_log_files, all_ports) =
        launch_instances(config, &target, instance_count, base_port)?;

    Ok(build_launch_result(
        all_pids,
        all_log_files,
        all_ports,
        config,
        &target,
        launch_start,
    ))
}

/// Find and validate a target using pre-cached scan results
fn find_and_validate_target_with_cache<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
    cached_targets: Vec<BevyTarget>,
) -> Result<BevyTarget> {
    use super::scanning;

    // Delegate to scanning which now handles all error cases (disambiguation, not-found-in-package)
    scanning::find_required_target_with_package_name(
        config.target_name(),
        T::TARGET_TYPE,
        config.package_name(),
        search_paths,
        Some(cached_targets),
    )
    .map_err(|e| Report::new(e))
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
        build_app_command(
            &target.get_binary_path(self.profile()),
            Some(self.port),
            self.env.as_ref(),
            self.args.as_deref(),
        )
    }

    fn extra_log_info(&self, _target: &BevyTarget) -> Option<String> { None }
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

    fn build_command(&self, _target: &BevyTarget) -> Command {
        build_cargo_example_command(
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
