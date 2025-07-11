use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono;
use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::constants::{DEFAULT_PROFILE, PROFILE_RELEASE};
use super::support::cargo_detector::TargetType;
use super::support::{cargo_detector, launch_common, process, scanning};
use crate::constants::{PARAM_EXAMPLE_NAME, PARAM_PROFILE};
use crate::extractors::McpCallExtractor;
use crate::service;
use crate::service::HandlerContext;
use crate::tool::{HandlerResponse, HandlerResult, LocalHandler};

/// Result from launching a Bevy example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchBevyExampleResult {
    /// Status of the launch operation
    pub status:             String,
    /// Status message
    pub message:            String,
    /// Example name that was launched
    pub example_name:       Option<String>,
    /// Process ID of the launched example
    pub pid:                Option<u32>,
    /// Port used for launch
    pub port:               Option<u16>,
    /// Working directory used for launch
    pub working_directory:  Option<String>,
    /// Build profile used (debug/release)
    pub profile:            Option<String>,
    /// Log file path for the launched example
    pub log_file:           Option<String>,
    /// Launch duration in milliseconds
    pub launch_duration_ms: Option<u64>,
    /// Launch timestamp
    pub launch_timestamp:   Option<String>,
    /// Workspace information
    pub workspace:          Option<String>,
    /// Package name containing the example
    pub package_name:       Option<String>,
    /// Available duplicate paths (for disambiguation errors)
    pub duplicate_paths:    Option<Vec<String>>,
    /// Note about build behavior
    pub note:               Option<String>,
}

impl HandlerResult for LaunchBevyExampleResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct LaunchBevyExample;

impl LocalHandler for LaunchBevyExample {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        let extractor = McpCallExtractor::from_request(&ctx.request);
        let example_name = match extractor.get_required_string(PARAM_EXAMPLE_NAME, "example name") {
            Ok(name) => name.to_string(),
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let profile = extractor.get_optional_string(PARAM_PROFILE, DEFAULT_PROFILE);
        let path = extractor
            .get_optional_path()
            .as_deref()
            .map(ToString::to_string);
        let port = match extractor.get_port() {
            Ok(p) => p,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        let service = Arc::clone(&ctx.service);
        let context = ctx.context.clone();

        Box::pin(async move {
            handle_impl(
                &example_name,
                &profile,
                path.as_deref(),
                port,
                service,
                context,
            )
            .await
            .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    example_name: &str,
    profile: &str,
    path: Option<&str>,
    port: u16,
    service: Arc<crate::McpService>,
    context: rmcp::service::RequestContext<rmcp::RoleServer>,
) -> Result<LaunchBevyExampleResult, McpError> {
    // Get search paths
    let search_paths = service::fetch_roots_and_get_paths(service, context).await?;

    // Launch the example
    launch_bevy_example(example_name, profile, path, port, &search_paths)
}

pub fn launch_bevy_example(
    example_name: &str,
    profile: &str,
    path: Option<&str>,
    port: u16,
    search_paths: &[PathBuf],
) -> Result<LaunchBevyExampleResult, McpError> {
    let launch_start = Instant::now();

    // Find and validate the example
    let (example, manifest_dir_buf, find_duration) =
        match find_and_validate_example(example_name, path, search_paths) {
            Ok(result) => result,
            Err(mcp_error) => {
                // Check if this is a path disambiguation error
                let error_msg = &mcp_error.message;
                if error_msg.contains("Found multiple") || error_msg.contains("not found at path") {
                    // Parse duplicate paths from error message
                    let duplicate_paths = if error_msg.contains("Found multiple") {
                        // Extract paths from error message like "Found multiple example named
                        // 'basic_app' at:\n- hana\n- hana-brp-extras-2-1"
                        let lines: Vec<&str> = error_msg.lines().collect();
                        let mut paths = Vec::new();
                        for line in &lines[1..] {
                            // Skip first line
                            if let Some(path) = line.strip_prefix("- ") {
                                paths.push(path.to_string());
                            }
                        }
                        if paths.is_empty() { None } else { Some(paths) }
                    } else {
                        None
                    };

                    // Return structured error response with minimal fields
                    let error_response = super::support::create_disambiguation_error(
                        "example",
                        example_name,
                        duplicate_paths.unwrap_or_default(),
                    );

                    // Convert the JSON value to our typed result
                    // We only populate the fields that are in the error response
                    return Ok(LaunchBevyExampleResult {
                        status:             error_response["status"]
                            .as_str()
                            .unwrap_or("error")
                            .to_string(),
                        message:            error_response["message"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        example_name:       error_response["example_name"]
                            .as_str()
                            .map(String::from),
                        pid:                None,
                        port:               Some(port),
                        working_directory:  None,
                        profile:            None,
                        log_file:           None,
                        launch_duration_ms: None,
                        launch_timestamp:   None,
                        workspace:          None,
                        package_name:       None,
                        duplicate_paths:    error_response["duplicate_paths"].as_array().map(
                            |arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            },
                        ),
                        note:               None,
                    });
                }
                return Err(mcp_error);
            }
        };
    let manifest_dir = manifest_dir_buf.as_path();

    // Setup launch environment
    let cargo_command = build_cargo_command_string(example_name, profile);
    collect_debug_info_if_enabled(example_name, &cargo_command, profile, manifest_dir);

    // Execute the launch process
    let launch_params = LaunchProcessParams {
        example_name,
        profile,
        port,
        manifest_dir,
        package_name: &example.package_name,
        cargo_command: &cargo_command,
        launch_start,
        find_duration,
    };
    let launch_result = execute_launch_process(launch_params)?;

    // Build and return response
    build_launch_response(
        example_name,
        &example,
        launch_result,
        launch_start,
        profile,
        port,
    )
}

struct LaunchResult {
    pid:           u32,
    log_file_path: PathBuf,
    launch_end:    Instant,
}

fn find_and_validate_example(
    example_name: &str,
    path: Option<&str>,
    search_paths: &[PathBuf],
) -> Result<(cargo_detector::BevyTarget, PathBuf, Duration), McpError> {
    let find_start = Instant::now();
    let example = match scanning::find_required_target_with_path(
        example_name,
        TargetType::Example,
        path,
        search_paths,
    ) {
        Ok(example) => example,
        Err(mcp_error) => {
            // Check if this is a path disambiguation error
            let error_msg = &mcp_error.message;
            if error_msg.contains("Found multiple") || error_msg.contains("not found at path") {
                // Convert to proper tool response
                return Err(mcp_error); // Let the caller handle conversion
            }
            return Err(mcp_error);
        }
    };
    let find_duration = find_start.elapsed();

    let manifest_path = example.manifest_path.clone();
    let manifest_dir = launch_common::validate_manifest_directory(&manifest_path)?;

    Ok((example, manifest_dir.to_path_buf(), find_duration))
}

fn build_cargo_command_string(example_name: &str, profile: &str) -> String {
    format!(
        "cargo run --example {example_name} {}",
        if profile == PROFILE_RELEASE {
            "--release"
        } else {
            ""
        }
    )
    .trim()
    .to_string()
}

fn collect_debug_info_if_enabled(
    example_name: &str,
    cargo_command: &str,
    profile: &str,
    manifest_dir: &Path,
) {
    debug!(
        "Launching example {} from {}",
        example_name,
        manifest_dir.display()
    );
    debug!("Working directory: {}", manifest_dir.display());
    debug!("CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    debug!("Profile: {}", profile);
    debug!("Command: {}", cargo_command);
}

/// Parameters for executing the launch process
struct LaunchProcessParams<'a> {
    example_name:  &'a str,
    profile:       &'a str,
    port:          u16,
    manifest_dir:  &'a Path,
    package_name:  &'a str,
    cargo_command: &'a str,
    launch_start:  Instant,
    find_duration: Duration,
}

fn execute_launch_process(params: LaunchProcessParams<'_>) -> Result<LaunchResult, McpError> {
    // Setup logging
    let log_setup_start = Instant::now();
    let (log_file_path, log_file_for_redirect) = launch_common::setup_launch_logging(
        params.example_name,
        "Example",
        params.profile,
        &PathBuf::from(params.cargo_command),
        params.manifest_dir,
        Some(params.port),
        Some(&format!("Package: {}", params.package_name)),
    )?;
    let log_setup_duration = log_setup_start.elapsed();

    // Build cargo command
    let cmd_setup_start = Instant::now();
    let cmd = launch_common::build_cargo_example_command(
        params.example_name,
        params.profile,
        Some(params.port),
    );
    let cmd_setup_duration = cmd_setup_start.elapsed();

    // Launch the process
    let spawn_start = Instant::now();
    let pid = process::launch_detached_process(
        &cmd,
        params.manifest_dir,
        log_file_for_redirect,
        params.example_name,
        "spawn",
    )?;
    let spawn_duration = spawn_start.elapsed();
    let launch_end = Instant::now();

    // Collect complete debug info
    let launch_duration_ms = launch_end.duration_since(params.launch_start).as_millis();

    debug!("Launch duration: {}ms", launch_duration_ms);
    debug!("Package: {}", params.package_name);
    debug!(
        "TIMING - Find example: {}ms",
        params.find_duration.as_millis()
    );
    debug!("TIMING - Log setup: {}ms", log_setup_duration.as_millis());
    debug!(
        "TIMING - Command setup: {}ms",
        cmd_setup_duration.as_millis()
    );
    debug!("TIMING - Spawn process: {}ms", spawn_duration.as_millis());

    debug!("Environment variable: BRP_PORT={}", params.port);

    Ok(LaunchResult {
        pid,
        log_file_path,
        launch_end,
    })
}

fn build_launch_response(
    example_name: &str,
    example: &cargo_detector::BevyTarget,
    launch_result: LaunchResult,
    launch_start: Instant,
    profile: &str,
    port: u16,
) -> Result<LaunchBevyExampleResult, McpError> {
    let manifest_dir = launch_common::validate_manifest_directory(&example.manifest_path)?;

    let launch_duration_ms = u64::try_from(
        launch_result
            .launch_end
            .duration_since(launch_start)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    let launch_timestamp = chrono::Utc::now().to_rfc3339();

    // Extract workspace name
    let workspace = example
        .workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(String::from);

    debug!("Environment variable: BRP_PORT={}", port);

    Ok(LaunchBevyExampleResult {
        status: "success".to_string(),
        message: format!(
            "Successfully launched '{example_name}' (PID: {})",
            launch_result.pid
        ),
        example_name: Some(example_name.to_string()),
        port: Some(port),
        pid: Some(launch_result.pid),
        working_directory: Some(manifest_dir.display().to_string()),
        profile: Some(profile.to_string()),
        log_file: Some(launch_result.log_file_path.display().to_string()),
        launch_duration_ms: Some(launch_duration_ms),
        launch_timestamp: Some(launch_timestamp),
        workspace,
        package_name: Some(example.package_name.clone()),
        duplicate_paths: None, // No duplicates for successful launches
        note: Some("Cargo will build the example if needed before running".to_string()),
    })
}
