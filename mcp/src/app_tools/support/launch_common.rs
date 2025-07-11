use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use chrono;
use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};

use crate::error::{Error, report_to_mcp_error};
use crate::tool::{HandlerContext, HandlerResponse, HandlerResult, LocalHandler};

/// Marker type for App launch configuration
pub struct App;

/// Marker type for Example launch configuration
pub struct Example;

/// Parameterized launch configuration for apps and examples
pub struct LaunchConfig<T> {
    pub target_name: String,
    pub profile:     String,
    pub path:        Option<String>,
    pub port:        u16,
    _phantom:        PhantomData<T>,
}

impl<T> LaunchConfig<T> {
    /// Create a new launch configuration
    pub const fn new(
        target_name: String,
        profile: String,
        path: Option<String>,
        port: u16,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchResult {
    /// Status of the launch operation
    pub status:             String,
    /// Status message
    pub message:            String,
    /// Name of the target that was launched (app or example)
    pub target_name:        Option<String>,
    /// Process ID of the launched target
    pub pid:                Option<u32>,
    /// Port used for launch
    pub port:               Option<u16>,
    /// Working directory used for launch
    pub working_directory:  Option<String>,
    /// Build profile used (debug/release)
    pub profile:            Option<String>,
    /// Log file path for the launched target
    pub log_file:           Option<String>,
    /// Binary path of the launched app (only for apps, not examples)
    pub binary_path:        Option<String>,
    /// Launch duration in milliseconds
    pub launch_duration_ms: Option<u64>,
    /// Launch timestamp
    pub launch_timestamp:   Option<String>,
    /// Workspace information
    pub workspace:          Option<String>,
    /// Package name containing the example (only for examples)
    pub package_name:       Option<String>,
    /// Available duplicate paths (for disambiguation errors)
    pub duplicate_paths:    Option<Vec<String>>,
    /// Note about build behavior or other information
    pub note:               Option<String>,
}

impl HandlerResult for LaunchResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

use crate::app_tools::constants::{TARGET_TYPE_APP, TARGET_TYPE_EXAMPLE};
use crate::constants::{BRP_PORT_ENV_VAR, PARAM_PROFILE};
use crate::extractors::McpCallExtractor;
use crate::service;

/// Parameters extracted from launch requests
pub struct LaunchParams {
    pub target_name: String,
    pub profile:     String,
    pub path:        Option<String>,
    pub port:        u16,
}

/// Extract common launch parameters from an MCP request
pub fn extract_launch_params(
    extractor: &McpCallExtractor,
    target_param_name: &str,
    target_type_name: &str,
    default_profile: &str,
) -> Result<LaunchParams, McpError> {
    let target_name = extractor.get_required_string(target_param_name, target_type_name)?;
    let profile = extractor.get_optional_string(PARAM_PROFILE, default_profile);
    let path = extractor
        .get_optional_path()
        .as_deref()
        .map(ToString::to_string);
    let port = extractor.get_port()?;

    Ok(LaunchParams {
        target_name: target_name.to_string(),
        profile,
        path,
        port,
    })
}

/// Generic launch handler that can work with any `LaunchConfig` type
pub struct GenericLaunchHandler<T: FromLaunchParams> {
    target_param_name: &'static str,
    target_type_name:  &'static str,
    default_profile:   &'static str,
    _phantom:          PhantomData<T>,
}

impl<T: FromLaunchParams> GenericLaunchHandler<T> {
    /// Create a new generic launch handler
    pub const fn new(
        target_param_name: &'static str,
        target_type_name: &'static str,
        default_profile: &'static str,
    ) -> Self {
        Self {
            target_param_name,
            target_type_name,
            default_profile,
            _phantom: PhantomData,
        }
    }
}

impl<T: FromLaunchParams> LocalHandler for GenericLaunchHandler<T> {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        let extractor = McpCallExtractor::from_request(&ctx.request);

        // Extract parameters
        let params = match extract_launch_params(
            &extractor,
            self.target_param_name,
            self.target_type_name,
            self.default_profile,
        ) {
            Ok(params) => params,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let service = Arc::clone(&ctx.service);
        let context = ctx.context.clone();

        Box::pin(async move {
            // Get search paths
            let search_paths = service::fetch_roots_and_get_paths(service, context).await?;

            // Create config from params
            let config = T::from_params(&params);

            // Launch the target
            let result = launch_target(&config, &search_paths)?;

            Ok(Box::new(result) as Box<dyn HandlerResult>)
        })
    }
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
    fn port(&self) -> u16;

    /// Build the command to execute
    fn build_command(&self, target: &super::cargo_detector::BevyTarget) -> Command;

    /// Validate the target before launch (e.g., check if binary exists)
    fn validate_target(&self, target: &super::cargo_detector::BevyTarget) -> Result<(), McpError>;

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
pub fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path, McpError> {
    manifest_path.parent().ok_or_else(|| -> McpError {
        report_to_mcp_error(
            &error_stack::Report::new(Error::Configuration("Invalid manifest path".to_string()))
                .attach_printable("No parent directory found")
                .attach_printable(format!("Path: {}", manifest_path.display())),
        )
    })
}

/// Validates that a binary exists at the given path
pub fn validate_binary_exists(binary_path: &Path, profile: &str) -> Result<(), McpError> {
    if !binary_path.exists() {
        return Err(report_to_mcp_error(
            &error_stack::Report::new(Error::Configuration("Missing binary file".to_string()))
                .attach_printable(format!("Binary path: {}", binary_path.display()))
                .attach_printable(format!(
                    "Please build the app with 'cargo build{}' first",
                    if profile == "release" {
                        " --release"
                    } else {
                        ""
                    }
                )),
        ));
    }
    Ok(())
}

/// Sets BRP-related environment variables on a command
///
/// Currently sets:
/// - `BRP_PORT`: When a port is provided, sets this environment variable for `bevy_brp_extras` to
///   read
pub fn set_brp_env_vars(cmd: &mut Command, port: Option<u16>) {
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
    port: Option<u16>,
    extra_log_info: Option<&str>,
) -> Result<(PathBuf, std::fs::File), McpError> {
    use super::logging;

    // Create log file
    let (log_file_path, _) = logging::create_log_file(
        name,
        name_type,
        profile,
        command_or_binary,
        manifest_dir,
        port,
    )?;

    // Add extra info to log file if provided
    if let Some(extra_info) = extra_log_info {
        logging::append_to_log_file(&log_file_path, &format!("{extra_info}\n"))?;
    }

    // Open log file for stdout/stderr redirection
    let log_file_for_redirect = logging::open_log_file_for_redirect(&log_file_path)?;

    Ok((log_file_path, log_file_for_redirect))
}

/// Build cargo command for running examples
pub fn build_cargo_example_command(
    example_name: &str,
    profile: &str,
    port: Option<u16>,
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
pub fn build_app_command(binary_path: &Path, port: Option<u16>) -> Command {
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
) -> Result<LaunchResult, McpError> {
    use super::process;

    // Launch the process
    let pid = process::launch_detached_process(
        cmd,
        manifest_dir,
        log_file_for_redirect,
        config.target_name(),
        "launch",
    )?;

    // Calculate launch duration
    let launch_end = std::time::Instant::now();
    let launch_duration_ms =
        u64::try_from(launch_end.duration_since(launch_start).as_millis()).unwrap_or(u64::MAX);
    let launch_timestamp = chrono::Utc::now().to_rfc3339();

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
) -> Result<(Command, PathBuf, PathBuf, std::fs::File), McpError> {
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

/// Find and validate a Bevy target based on configuration
fn find_and_validate_target<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
) -> Result<super::cargo_detector::BevyTarget, Box<LaunchResult>> {
    use super::cargo_detector::TargetType;
    use super::scanning;

    // Determine target type
    let target_type = if T::TARGET_TYPE == TARGET_TYPE_APP {
        TargetType::App
    } else {
        TargetType::Example
    };

    // Find the target
    let target = match scanning::find_required_target_with_path(
        config.target_name(),
        target_type,
        config.path(),
        search_paths,
    ) {
        Ok(target) => target,
        Err(mcp_error) => {
            // Check if this is a path disambiguation error
            let error_msg = &mcp_error.message;
            if error_msg.contains("Found multiple") || error_msg.contains("not found at path") {
                let duplicate_paths = super::extract_duplicate_paths(error_msg);

                return Err(Box::new(LaunchResult {
                    status: "error".to_string(),
                    message: format!(
                        "Found multiple {}s named '{}'. Please specify which path to use.",
                        T::TARGET_TYPE,
                        config.target_name()
                    ),
                    target_name: Some(config.target_name().to_string()),
                    pid: None,
                    port: Some(config.port()),
                    working_directory: None,
                    profile: None,
                    log_file: None,
                    binary_path: None,
                    launch_duration_ms: None,
                    launch_timestamp: None,
                    workspace: None,
                    package_name: None,
                    duplicate_paths,
                    note: None,
                }));
            }
            return Err(Box::new(LaunchResult {
                status:             "error".to_string(),
                message:            mcp_error.message.to_string(),
                target_name:        Some(config.target_name().to_string()),
                pid:                None,
                port:               Some(config.port()),
                working_directory:  None,
                profile:            None,
                log_file:           None,
                binary_path:        None,
                launch_duration_ms: None,
                launch_timestamp:   None,
                workspace:          None,
                package_name:       None,
                duplicate_paths:    None,
                note:               None,
            }));
        }
    };

    // Validate the target
    if let Err(e) = config.validate_target(&target) {
        return Err(Box::new(LaunchResult {
            status:             "error".to_string(),
            message:            e.message.to_string(),
            target_name:        Some(config.target_name().to_string()),
            pid:                None,
            port:               Some(config.port()),
            working_directory:  None,
            profile:            None,
            log_file:           None,
            binary_path:        None,
            launch_duration_ms: None,
            launch_timestamp:   None,
            workspace:          None,
            package_name:       None,
            duplicate_paths:    None,
            note:               None,
        }));
    }

    Ok(target)
}

/// Generic function to launch a Bevy target (app or example)
pub fn launch_target<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
) -> Result<LaunchResult, McpError> {
    use std::time::Instant;

    use tracing::debug;

    let launch_start = Instant::now();

    // Log additional debug info
    debug!("Environment variable: BRP_PORT={}", config.port());

    // Find and validate the target
    let target = match find_and_validate_target(config, search_paths) {
        Ok(target) => target,
        Err(launch_result) => return Ok(*launch_result),
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

    fn port(&self) -> u16 {
        self.port
    }

    fn build_command(&self, target: &super::cargo_detector::BevyTarget) -> Command {
        build_app_command(&target.get_binary_path(self.profile()), Some(self.port))
    }

    fn validate_target(&self, target: &super::cargo_detector::BevyTarget) -> Result<(), McpError> {
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
            status: "success".to_string(),
            message: format!("Successfully launched '{}' (PID: {pid})", self.target_name),
            target_name: Some(self.target_name.clone()),
            pid: Some(pid),
            port: Some(self.port),
            working_directory: Some(working_directory.display().to_string()),
            profile: Some(self.profile.clone()),
            log_file: Some(log_file.display().to_string()),
            binary_path: Some(target.get_binary_path(self.profile()).display().to_string()),
            launch_duration_ms: Some(launch_duration_ms),
            launch_timestamp: Some(launch_timestamp),
            workspace,
            package_name: None,
            duplicate_paths: None,
            note: None,
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

    fn port(&self) -> u16 {
        self.port
    }

    fn build_command(&self, _target: &super::cargo_detector::BevyTarget) -> Command {
        build_cargo_example_command(&self.target_name, self.profile(), Some(self.port))
    }

    fn validate_target(&self, _target: &super::cargo_detector::BevyTarget) -> Result<(), McpError> {
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
            status: "success".to_string(),
            message: format!("Successfully launched '{}' (PID: {pid})", self.target_name),
            target_name: Some(self.target_name.clone()),
            pid: Some(pid),
            port: Some(self.port),
            working_directory: Some(working_directory.display().to_string()),
            profile: Some(self.profile.clone()),
            log_file: Some(log_file.display().to_string()),
            binary_path: None,
            launch_duration_ms: Some(launch_duration_ms),
            launch_timestamp: Some(launch_timestamp),
            workspace,
            package_name: Some(target.package_name.clone()),
            duplicate_paths: None,
            note: Some("Cargo will build the example if needed before running".to_string()),
        }
    }
}
