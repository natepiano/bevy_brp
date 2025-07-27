use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Command;

use bevy_brp_mcp_macros::ResultStruct;
use chrono::Utc;
use error_stack::Report;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

/// Marker type for App launch configuration
pub struct App;

/// Marker type for Example launch configuration
pub struct Example;

/// Parameterized launch configuration for apps and examples
pub struct LaunchConfig<T> {
    pub target_name: String,
    pub profile:     String,
    pub path:        Option<String>,
    pub port:        Port,
    _phantom:        PhantomData<T>,
}

impl<T> LaunchConfig<T> {
    /// Create a new launch configuration
    pub const fn new(
        target_name: String,
        profile: String,
        path: Option<String>,
        port: Port,
    ) -> Self {
        Self {
            target_name,
            profile,
            path,
            port,
            _phantom: PhantomData,
        }
    }
}

/// Unified result type for launching Bevy apps and examples
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
#[allow(clippy::too_many_arguments)]
pub struct LaunchResult {
    /// Name of the target that was launched (app or example)
    #[to_metadata(skip_if_none)]
    target_name:        Option<String>,
    /// Process ID of the launched target
    #[to_metadata(skip_if_none)]
    pid:                Option<u32>,
    /// Working directory used for launch
    #[to_metadata(skip_if_none)]
    working_directory:  Option<String>,
    /// Build profile used (debug/release)
    #[to_metadata(skip_if_none)]
    profile:            Option<String>,
    /// Log file path for the launched target
    #[to_metadata(skip_if_none)]
    log_file:           Option<String>,
    /// Binary path of the launched app (only for apps, not examples)
    #[to_metadata(skip_if_none)]
    binary_path:        Option<String>,
    /// Launch duration in milliseconds
    #[to_metadata(skip_if_none)]
    launch_duration_ms: Option<u64>,
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
    #[to_message(message_template = "Successfully launched {target_name} (PID: {pid})")]
    message_template:   String,
}

use crate::app_tools::constants::{TARGET_TYPE_APP, TARGET_TYPE_EXAMPLE};
use crate::brp_tools::{BRP_PORT_ENV_VAR, Port};

/// Parameters extracted from launch requests
pub struct LaunchParams {
    pub target_name: String,
    pub profile:     String,
    pub path:        Option<String>,
    pub port:        Port,
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

impl<T: FromLaunchParams, P: ToLaunchParams + for<'de> serde::Deserialize<'de>> ToolFn
    for GenericLaunchHandler<T, P>
{
    type Output = LaunchResult;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output>> {
        let default_profile = self.default_profile;
        Box::pin(async move {
            // Extract typed parameters - this returns framework error on failure
            let typed_params: P = ctx.extract_parameter_values()?;

            // Convert to LaunchParams
            let params = typed_params.to_launch_params(default_profile);
            let port = params.port;

            // Get search paths
            let search_paths = ctx.roots;

            // Create config from params
            let config = T::from_params(&params);

            // Launch the target
            let result = launch_target(&config, &search_paths);

            Ok(ToolResult::with_port(result, port))
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
pub trait LaunchConfigTrait {
    /// The target type constant ("app" or "example")
    const TARGET_TYPE: &'static str;

    /// Get the name of the target being launched
    fn target_name(&self) -> &str;

    /// Get the build profile ("debug" or "release")
    fn profile(&self) -> &str;

    /// Get the optional path for disambiguation
    fn path(&self) -> Option<&str>;

    /// Get the BRP port
    fn port(&self) -> Port;

    /// Build the command to execute
    fn build_command(&self, target: &super::cargo_detector::BevyTarget) -> Command;

    /// Validate the target before launch (e.g., check if binary exists)
    fn validate_target(&self, target: &super::cargo_detector::BevyTarget) -> Result<()>;

    /// Get any extra log info specific to this target type
    fn extra_log_info(&self, target: &super::cargo_detector::BevyTarget) -> Option<String>;

    /// Convert to unified `LaunchResult` on success
    fn to_launch_result(
        &self,
        pid: u32,
        log_file: PathBuf,
        working_directory: PathBuf,
        launch_duration_ms: u64,
        launch_timestamp: String,
        target: &super::cargo_detector::BevyTarget,
    ) -> LaunchResult;
}

/// Validates and extracts the manifest directory from a manifest path
pub fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path> {
    manifest_path.parent().ok_or_else(|| {
        error_stack::Report::new(Error::FileOrPathNotFound(
            "Invalid manifest path".to_string(),
        ))
        .attach_printable("No parent directory found")
        .attach_printable(format!("Path: {}", manifest_path.display()))
    })
}

/// Validates that a binary exists at the given path
pub fn validate_binary_exists(binary_path: &Path, profile: &str) -> Result<()> {
    if !binary_path.exists() {
        return Err(error_stack::Report::new(Error::FileOrPathNotFound(
            "Missing binary file".to_string(),
        ))
        .attach_printable(format!("Binary path: {}", binary_path.display()))
        .attach_printable(format!(
            "Please build the app with 'cargo build{}' first",
            if profile == "release" {
                " --release"
            } else {
                ""
            }
        )));
    }
    Ok(())
}

/// Sets BRP-related environment variables on a command
///
/// Currently sets:
/// - `BRP_PORT`: When a port is provided, sets this environment variable for `bevy_brp_extras` to
///   read
pub fn set_brp_env_vars(cmd: &mut Command, port: Option<Port>) {
    if let Some(port) = port {
        cmd.env(BRP_PORT_ENV_VAR, port.to_string());
    }
}

/// Setup logging for launch operations and return log file handles
pub fn setup_launch_logging(
    name: &str,
    name_type: &str, // "App" or "Example"
    profile: &str,
    command_or_binary: &Path,
    manifest_dir: &Path,
    port: Option<Port>,
    extra_log_info: Option<&str>,
) -> Result<(PathBuf, std::fs::File)> {
    use super::logging;

    // Create log file
    let (log_file_path, _) = logging::create_log_file(
        name,
        name_type,
        profile,
        command_or_binary,
        manifest_dir,
        port,
    )
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

/// Execute the process and build the launch result
fn execute_and_build_result<T: LaunchConfigTrait>(
    config: &T,
    cmd: &Command,
    manifest_dir: &Path,
    log_file_path: PathBuf,
    log_file_for_redirect: std::fs::File,
    target: &super::cargo_detector::BevyTarget,
    launch_start: std::time::Instant,
) -> Result<LaunchResult> {
    use super::process;

    // Launch the process
    let pid = process::launch_detached_process(
        cmd,
        manifest_dir,
        log_file_for_redirect,
        config.target_name(),
        "launch",
    )
    .map_err(|e| Error::tool_call_failed(e.to_string()))?;

    // Calculate launch duration
    let launch_end = std::time::Instant::now();
    let launch_duration_ms =
        u64::try_from(launch_end.duration_since(launch_start).as_millis()).unwrap_or(u64::MAX);
    let launch_timestamp = Utc::now().to_rfc3339();

    // Build result
    Ok(config.to_launch_result(
        pid,
        log_file_path,
        manifest_dir.to_path_buf(),
        launch_duration_ms,
        launch_timestamp,
        target,
    ))
}

/// Prepare the launch environment including command, logging, and directory setup
fn prepare_launch_environment<T: LaunchConfigTrait>(
    config: &T,
    target: &super::cargo_detector::BevyTarget,
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
        Some(config.port()),
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
) -> Result<super::cargo_detector::BevyTarget> {
    use super::cargo_detector::TargetType;
    use super::scanning;

    // Determine target type
    let target_type = if T::TARGET_TYPE == TARGET_TYPE_APP {
        TargetType::App
    } else {
        TargetType::Example
    };

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

    // Find the specific target with path disambiguation
    let target = match scanning::find_required_target_with_path(
        config.target_name(),
        target_type,
        config.path(),
        search_paths,
    ) {
        Ok(target) => target,
        Err(err) => {
            use crate::error::Error;

            // Check if this is already a PathDisambiguation error
            if let Error::PathDisambiguation {
                message,
                available_paths,
                ..
            } = &err
            {
                // Use the original error message and paths
                return Err(error_stack::Report::new(
                    Error::tool_call_failed_with_details(
                        message.clone(),
                        create_error_details(config, Some(available_paths.clone())),
                    ),
                ));
            }

            // For any other error when duplicates exist, return disambiguation error with paths
            if duplicate_paths.is_some() {
                let message = config.path().map_or_else(
                    || {
                        // No path provided
                        format!(
                            "Found multiple {}s named '{}'. Please specify which path to use.",
                            T::TARGET_TYPE,
                            config.target_name()
                        )
                    },
                    |path| {
                        // User provided a path but it didn't match
                        format!(
                            "Found multiple {}s named '{}'. The path '{}' does not match any available paths.",
                            T::TARGET_TYPE,
                            config.target_name(),
                            path
                        )
                    }
                );

                return Err(error_stack::Report::new(
                    Error::tool_call_failed_with_details(
                        message,
                        create_error_details(config, duplicate_paths),
                    ),
                ));
            }

            // For non-duplicate errors, return standard error
            return Err(Report::new(Error::tool_call_failed_with_details(
                err.to_string(),
                create_error_details(config, None),
            )));
        }
    };

    // Validate the target
    if let Err(e) = config.validate_target(&target) {
        let error_details = create_error_details(config, None);
        return Err(Report::new(Error::tool_call_failed_with_details(
            (*e.current_context()).to_string(),
            error_details,
        )));
    }

    Ok(target)
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
    debug!("Environment variable: BRP_PORT={}", config.port());

    // Find and validate the target
    let target = match find_and_validate_target(config, search_paths) {
        Ok(target) => target,
        Err(launch_result) => {
            // Convert error to ToolError with details
            let error_message = format!("{}", launch_result.current_context());
            let details = serde_json::json!({
                "error": error_message,
                "error_chain": format!("{:?}", launch_result)
            });
            return Err(Error::tool_call_failed_with_details(error_message, details).into());
        }
    };

    // Prepare launch environment
    let (cmd, manifest_dir, log_file_path, log_file_for_redirect) =
        prepare_launch_environment(config, &target)?;

    // Execute and build result
    execute_and_build_result(
        config,
        &cmd,
        &manifest_dir,
        log_file_path,
        log_file_for_redirect,
        &target,
        launch_start,
    )
}

impl FromLaunchParams for LaunchConfig<App> {
    fn from_params(params: &LaunchParams) -> Self {
        Self::new(
            params.target_name.clone(),
            params.profile.clone(),
            params.path.clone(),
            params.port,
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<App> {
    const TARGET_TYPE: &'static str = TARGET_TYPE_APP;

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

    fn build_command(&self, target: &super::cargo_detector::BevyTarget) -> Command {
        build_app_command(&target.get_binary_path(self.profile()), Some(self.port))
    }

    fn validate_target(&self, target: &super::cargo_detector::BevyTarget) -> Result<()> {
        let binary_path = target.get_binary_path(self.profile());
        validate_binary_exists(&binary_path, self.profile())
    }

    fn extra_log_info(&self, _target: &super::cargo_detector::BevyTarget) -> Option<String> {
        None
    }

    fn to_launch_result(
        &self,
        pid: u32,
        log_file: PathBuf,
        working_directory: PathBuf,
        launch_duration_ms: u64,
        launch_timestamp: String,
        target: &super::cargo_detector::BevyTarget,
    ) -> LaunchResult {
        let workspace = target
            .workspace_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(String::from);

        LaunchResult {
            target_name: Some(self.target_name.clone()),
            pid: Some(pid),
            working_directory: Some(working_directory.display().to_string()),
            profile: Some(self.profile.clone()),
            log_file: Some(log_file.display().to_string()),
            binary_path: Some(target.get_binary_path(self.profile()).display().to_string()),
            launch_duration_ms: Some(launch_duration_ms),
            launch_timestamp: Some(launch_timestamp),
            workspace,
            package_name: None,
            duplicate_paths: None,
            message_template: "Successfully launched {{target_name}} (PID: {{pid}})".to_string(),
        }
    }
}

impl FromLaunchParams for LaunchConfig<Example> {
    fn from_params(params: &LaunchParams) -> Self {
        Self::new(
            params.target_name.clone(),
            params.profile.clone(),
            params.path.clone(),
            params.port,
        )
    }
}

impl LaunchConfigTrait for LaunchConfig<Example> {
    const TARGET_TYPE: &'static str = TARGET_TYPE_EXAMPLE;

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

    fn build_command(&self, _target: &super::cargo_detector::BevyTarget) -> Command {
        build_cargo_example_command(&self.target_name, self.profile(), Some(self.port))
    }

    fn validate_target(&self, _target: &super::cargo_detector::BevyTarget) -> Result<()> {
        // Examples don't need binary validation - cargo will build them if needed
        Ok(())
    }

    fn extra_log_info(&self, target: &super::cargo_detector::BevyTarget) -> Option<String> {
        Some(format!("Package: {}", target.package_name))
    }

    fn to_launch_result(
        &self,
        pid: u32,
        log_file: PathBuf,
        working_directory: PathBuf,
        launch_duration_ms: u64,
        launch_timestamp: String,
        target: &super::cargo_detector::BevyTarget,
    ) -> LaunchResult {
        let workspace = target
            .workspace_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(String::from);

        LaunchResult {
            target_name: Some(self.target_name.clone()),
            pid: Some(pid),
            working_directory: Some(working_directory.display().to_string()),
            profile: Some(self.profile.clone()),
            log_file: Some(log_file.display().to_string()),
            binary_path: None,
            launch_duration_ms: Some(launch_duration_ms),
            launch_timestamp: Some(launch_timestamp),
            workspace,
            package_name: Some(target.package_name.clone()),
            duplicate_paths: None,
            message_template: "Successfully launched example {{target_name}} (PID: {{pid}})"
                .to_string(),
        }
    }
}
