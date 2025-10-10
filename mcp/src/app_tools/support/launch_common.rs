use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Command;

use bevy_brp_mcp_macros::ResultStruct;
use error_stack::Report;
use serde::{Deserialize, Serialize};

use super::errors::{NoTargetsFoundError, PathDisambiguationError, TargetNotFoundAtSpecifiedPath};
use super::process;
use crate::app_tools::support::cargo_detector::BevyTarget;
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, ParamStruct, ToolFn, ToolResult};

/// Marker type for App launch configuration
#[derive(Clone)]
pub struct App;

/// Marker type for Example launch configuration
#[derive(Clone)]
pub struct Example;

/// Parameterized launch configuration for apps and examples
#[derive(Clone)]
pub struct LaunchConfig<T> {
    pub target_name:    String,
    pub profile:        String,
    pub path:           Option<String>,
    pub port:           Port,
    pub instance_count: InstanceCount,
    _phantom:           PhantomData<T>,
}

impl<T> LaunchConfig<T> {
    /// Create a new launch configuration
    pub const fn new(
        target_name: String,
        profile: String,
        path: Option<String>,
        port: Port,
        instance_count: InstanceCount,
    ) -> Self {
        Self {
            target_name,
            profile,
            path,
            port,
            instance_count,
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
    /// Available duplicate paths (for disambiguation errors)
    #[to_metadata(skip_if_none)]
    duplicate_paths:    Option<Vec<String>>,
    /// Message template for formatting responses
    #[to_message]
    message_template:   Option<String>,
}

use crate::app_tools::instance_count::InstanceCount;
use crate::brp_tools::{BRP_EXTRAS_PORT_ENV_VAR, Port};

/// Parameters extracted from launch requests
pub struct LaunchParams {
    pub target_name:    String,
    pub profile:        String,
    pub path:           Option<String>,
    pub port:           Port,
    pub instance_count: InstanceCount,
}

/// Generic launch handler that can work with any `LaunchConfig` type
pub struct GenericLaunchHandler<T: FromLaunchParams, P: ToLaunchParams> {
    default_profile: &'static str,
    _phantom_config: PhantomData<T>,
    _phantom_params: PhantomData<P>,
}

impl<T: FromLaunchParams, P: ToLaunchParams> GenericLaunchHandler<T, P> {
    /// Create a new generic launch handler
    pub const fn new(default_profile: &'static str) -> Self {
        Self {
            default_profile,
            _phantom_config: PhantomData,
            _phantom_params: PhantomData,
        }
    }
}

impl<T: FromLaunchParams, P: ToLaunchParams + ParamStruct + for<'de> serde::Deserialize<'de>> ToolFn
    for GenericLaunchHandler<T, P>
{
    type Output = LaunchResult;
    type Params = P;

    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<'_, ToolResult<Self::Output, Self::Params>> {
        let default_profile = self.default_profile;
        Box::pin(async move {
            // Extract typed parameters - this returns framework error on failure
            let typed_params: P = ctx.extract_parameter_values()?;

            // Convert to LaunchParams
            let params = typed_params.to_launch_params(default_profile);
            // Port is available in params but not needed for launch

            // Get search paths
            let search_paths = ctx.roots;

            // Create config from params
            let config = T::from_params(&params);

            // Launch the target
            let result = launch_target(&config, &search_paths);

            Ok(ToolResult {
                result,
                params: Some(typed_params),
            })
        })
    }
}

/// Trait for converting typed parameters to `LaunchParams`
pub trait ToLaunchParams: Send + Sync {
    /// Convert to `LaunchParams` with the given default profile
    fn to_launch_params(&self, default_profile: &str) -> LaunchParams;
}

/// Trait for creating launch configs from params
pub trait FromLaunchParams: LaunchConfigTrait + Sized + Send + Sync {
    /// Create a new instance from launch parameters
    fn from_params(params: &LaunchParams) -> Self;
}

/// Trait for configuring launch behavior for different target types (app vs example)
pub trait LaunchConfigTrait: Clone {
    /// The target type constant (App or Example)
    const TARGET_TYPE: TargetType;

    /// Get the name of the target being launched
    fn target_name(&self) -> &str;

    /// Get the build profile ("debug" or "release")
    fn profile(&self) -> &str;

    /// Get the optional path for disambiguation
    fn path(&self) -> Option<&str>;

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
pub fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path> {
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
pub fn set_brp_env_vars(cmd: &mut Command, port: Option<Port>) {
    if let Some(port) = port {
        cmd.env(BRP_EXTRAS_PORT_ENV_VAR, port.to_string());
    }
}

/// Setup logging for launch operations and return log file handles
pub fn setup_launch_logging(
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
pub fn build_cargo_example_command(
    example_name: &str,
    profile: &str,
    port: Option<Port>,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example").arg(example_name);

    // Add profile flag if release
    if profile == "release" {
        cmd.arg("--release");
    }

    // Set BRP-related environment variables
    set_brp_env_vars(&mut cmd, port);

    cmd
}

/// Build command for running app binaries
pub fn build_app_command(binary_path: &Path, port: Option<Port>) -> Command {
    let mut cmd = Command::new(binary_path);
    set_brp_env_vars(&mut cmd, port);
    cmd
}

use super::cargo_detector::TargetType;

/// Represents the state of a build target after cargo build
#[derive(Debug, Clone, Copy)]
pub enum BuildState {
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
    use tracing::{debug, info};

    match build_state {
        BuildState::NotFound => {
            debug!(
                "Target '{}' not found in build output, assuming it was built",
                target_name
            );
        }
        BuildState::Fresh => {
            debug!("{} '{}' was already up to date", target_type, target_name);
        }
        BuildState::Rebuilt => {
            info!("{} '{}' was built successfully", target_type, target_name);
        }
    }
}

/// Run cargo build for a target and block until completion
pub fn run_cargo_build(
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

/// Create error details for `ToolError` with common fields populated
fn create_error_details<T: LaunchConfigTrait>(
    config: &T,
    duplicate_paths: Option<Vec<String>>,
) -> serde_json::Value {
    serde_json::json!({
        "target_name": config.target_name(),
        "target_type": T::TARGET_TYPE,
        "profile": config.profile(),
        "path": config.path(),
        "port": config.port(),
        "duplicate_paths": duplicate_paths
    })
}

/// Find and validate a Bevy target based on configuration
fn find_and_validate_target<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
) -> Result<BevyTarget> {
    use super::scanning;

    // Get the target type from the config
    let target_type = T::TARGET_TYPE;

    // First, find all targets with the given name to check for duplicates
    let all_targets =
        scanning::find_all_targets_by_name(config.target_name(), Some(target_type), search_paths);

    // If multiple targets exist, we always want to include their paths
    let duplicate_paths = if all_targets.len() > 1 {
        Some(
            all_targets
                .iter()
                .map(|target| target.relative_path.to_string_lossy().to_string())
                .collect(),
        )
    } else {
        None
    };

    // Find the specific target with path disambiguation (reuse all_targets to avoid duplicate scan)
    let target = match scanning::find_required_target_with_path(
        config.target_name(),
        target_type,
        config.path(),
        search_paths,
        Some(all_targets.clone()),
    ) {
        Ok(target) => target,
        Err(err) => {
            use crate::error::Error;

            // For any other error when duplicates exist, return disambiguation error with paths
            if let Some(available_paths) = duplicate_paths {
                let path_disambiguation_error = PathDisambiguationError::new(
                    available_paths,
                    config.target_name().to_string(),
                    T::TARGET_TYPE.to_string(),
                );

                Err(Error::Structured {
                    result: Box::new(path_disambiguation_error),
                })?;
            }

            // For non-duplicate errors, determine appropriate structured error
            match all_targets.len() {
                0 => {
                    // No targets found at all
                    let no_targets_error = NoTargetsFoundError::new(
                        config.target_name().to_string(),
                        T::TARGET_TYPE.to_string(),
                    );
                    return Err(Error::Structured {
                        result: Box::new(no_targets_error),
                    })?;
                }
                1 => {
                    // Exactly one target exists but path disambiguation failed
                    let available_paths: Vec<String> = all_targets
                        .iter()
                        .map(|target| target.relative_path.to_string_lossy().to_string())
                        .collect();
                    let target_not_found_error = TargetNotFoundAtSpecifiedPath::new(
                        config.target_name().to_string(),
                        T::TARGET_TYPE.to_string(),
                        config.path().map(std::string::ToString::to_string),
                        available_paths,
                    );
                    return Err(Error::Structured {
                        result: Box::new(target_not_found_error),
                    })?;
                }
                _ => {
                    // This should not happen due to duplicate_paths logic above, but fallback
                    return Err(Report::new(Error::tool_call_failed_with_details(
                        err.to_string(),
                        create_error_details(config, None),
                    )));
                }
            }
        }
    };

    Ok(target)
}

/// Validate that the port range for multi-instance launching is within bounds
fn validate_port_range(base_port: u16, instance_count: usize) -> Result<()> {
    use crate::brp_tools::MAX_VALID_PORT;

    // Convert instance_count to u16, failing if it's too large
    let count_u16 = u16::try_from(instance_count).map_err(|_| {
        Error::tool_call_failed(format!(
            "Instance count {} is too large (maximum is {})",
            instance_count,
            u16::MAX
        ))
    })?;

    // MAX_VALID_PORT is imported from brp_tools::constants (65534)
    if base_port.saturating_add(count_u16.saturating_sub(1)) > MAX_VALID_PORT {
        return Err(Error::tool_call_failed(format!(
            "Port range {} to {} exceeds maximum valid port {}",
            base_port,
            base_port.saturating_add(count_u16.saturating_sub(1)),
            MAX_VALID_PORT
        ))
        .into());
    }
    Ok(())
}

/// Launch multiple instances of a target
fn launch_instances<T: LaunchConfigTrait>(
    config: &T,
    target: &BevyTarget,
    instance_count: usize,
    base_port: u16,
) -> Result<(Vec<u32>, Vec<PathBuf>, Vec<u16>)> {
    let mut all_pids = Vec::new();
    let mut all_log_files = Vec::new();
    let mut all_ports = Vec::new();

    for i in 0..instance_count {
        // Use saturating conversion - validated in validate_port_range that this won't overflow
        let i_u16 = u16::try_from(i).unwrap_or(u16::MAX);
        let port = Port(base_port.saturating_add(i_u16));

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

/// Generic function to launch a Bevy target (app or example)
pub fn launch_target<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
) -> Result<LaunchResult> {
    use std::time::Instant;

    use tracing::debug;

    let launch_start = Instant::now();

    // Log additional debug info
    debug!("Environment variable: BRP_EXTRAS_PORT={}", config.port());

    // Find and validate the target
    let target =
        find_and_validate_target(config, search_paths).map_err(handle_target_discovery_error)?;

    // Ensure the target is built (blocks until compilation completes if needed)
    let build_state = config.ensure_built(&target)?;
    match build_state {
        BuildState::Fresh => debug!("Target was already up to date, launching immediately"),
        BuildState::Rebuilt => debug!("Target was rebuilt before launch"),
        BuildState::NotFound => {
            use tracing::warn;
            warn!("Target not found in build output but build succeeded");
        }
    }

    let instance_count = *config.instance_count();
    let base_port = *config.port();

    // Validate entire port range fits within valid bounds
    validate_port_range(base_port, instance_count)?;

    // Launch all instances
    let (all_pids, all_log_files, all_ports) =
        launch_instances(config, &target, instance_count, base_port)?;

    // Build unified result (works for both single and multi)
    Ok(build_launch_result(
        all_pids,
        all_log_files,
        all_ports,
        config,
        &target,
        launch_start,
    ))
}

impl FromLaunchParams for LaunchConfig<App> {
    fn from_params(params: &LaunchParams) -> Self {
        Self::new(
            params.target_name.clone(),
            params.profile.clone(),
            params.path.clone(),
            params.port,
            params.instance_count,
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<App> {
    const TARGET_TYPE: TargetType = TargetType::App;

    fn target_name(&self) -> &str {
        &self.target_name
    }

    fn profile(&self) -> &str {
        &self.profile
    }

    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn port(&self) -> Port {
        self.port
    }

    fn instance_count(&self) -> InstanceCount {
        self.instance_count
    }

    fn set_port(&mut self, port: Port) {
        self.port = port;
    }

    fn build_command(&self, target: &BevyTarget) -> Command {
        build_app_command(&target.get_binary_path(self.profile()), Some(self.port))
    }

    fn extra_log_info(&self, _target: &BevyTarget) -> Option<String> {
        None
    }
}

impl FromLaunchParams for LaunchConfig<Example> {
    fn from_params(params: &LaunchParams) -> Self {
        Self::new(
            params.target_name.clone(),
            params.profile.clone(),
            params.path.clone(),
            params.port,
            params.instance_count,
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<Example> {
    const TARGET_TYPE: TargetType = TargetType::Example;

    fn target_name(&self) -> &str {
        &self.target_name
    }

    fn profile(&self) -> &str {
        &self.profile
    }

    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn port(&self) -> Port {
        self.port
    }

    fn instance_count(&self) -> InstanceCount {
        self.instance_count
    }

    fn set_port(&mut self, port: Port) {
        self.port = port;
    }

    fn build_command(&self, _target: &BevyTarget) -> Command {
        build_cargo_example_command(&self.target_name, self.profile(), Some(self.port))
    }

    fn extra_log_info(&self, target: &BevyTarget) -> Option<String> {
        Some(format!("Package: {}", target.package_name))
    }
}
