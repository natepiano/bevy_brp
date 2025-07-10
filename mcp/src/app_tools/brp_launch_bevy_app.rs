use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono;
use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::constants::{DEFAULT_PROFILE, PARAM_APP_NAME, PARAM_PROFILE};
use super::support::cargo_detector::{BevyTarget, TargetType};
use super::support::{launch_common, process, scanning};
use crate::app_tools::constants::PROFILE_RELEASE;
use crate::error::{Error, report_to_mcp_error};
use crate::extractors::McpCallExtractor;
use crate::handler::{HandlerContext, HandlerResponse, HandlerResult, LocalHandler};
use crate::service;

/// Result from launching a Bevy app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchBevyAppResult {
    /// Status of the launch operation
    pub status:             String,
    /// Status message
    pub message:            String,
    /// App name that was launched
    pub app_name:           Option<String>,
    /// Process ID of the launched app
    pub pid:                Option<u32>,
    /// Port used for launch
    pub port:               Option<u16>,
    /// Working directory used for launch
    pub working_directory:  Option<String>,
    /// Build profile used (debug/release)
    pub profile:            Option<String>,
    /// Log file path for the launched app
    pub log_file:           Option<String>,
    /// Binary path of the launched app
    pub binary_path:        Option<String>,
    /// Launch duration in milliseconds
    pub launch_duration_ms: Option<u64>,
    /// Launch timestamp
    pub launch_timestamp:   Option<String>,
    /// Workspace information
    pub workspace:          Option<String>,
    /// Available duplicate paths (for disambiguation errors)
    pub duplicate_paths:    Option<Vec<String>>,
}

impl HandlerResult for LaunchBevyAppResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct LaunchBevyApp;

impl LocalHandler for LaunchBevyApp {
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_> {
        let extractor = McpCallExtractor::from_request(&ctx.request);
        let app_name = match extractor.get_required_string(PARAM_APP_NAME, "app name") {
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
            handle_impl(&app_name, &profile, path.as_deref(), port, service, context)
                .await
                .map(|result| Box::new(result) as Box<dyn HandlerResult>)
        })
    }
}

async fn handle_impl(
    app_name: &str,
    profile: &str,
    path: Option<&str>,
    port: u16,
    service: Arc<crate::McpService>,
    context: rmcp::service::RequestContext<rmcp::RoleServer>,
) -> Result<LaunchBevyAppResult, McpError> {
    // Get search paths
    let search_paths = service::fetch_roots_and_get_paths(service, context).await?;

    // Launch the app
    launch_bevy_app(app_name, profile, path, port, &search_paths)
}

pub fn launch_bevy_app(
    app_name: &str,
    profile: &str,
    path: Option<&str>,
    port: u16,
    search_paths: &[PathBuf],
) -> Result<LaunchBevyAppResult, McpError> {
    let launch_start = Instant::now();

    let app = match find_and_validate_app(app_name, path, search_paths, port)? {
        AppDiscoveryResult::Found(app) => app,
        AppDiscoveryResult::DisambiguationError(result) => return Ok(result),
    };

    validate_binary_exists(&app, profile)?;
    let manifest_dir = launch_common::validate_manifest_directory(&app.manifest_path)?;

    log_launch_info(
        app_name,
        manifest_dir,
        profile,
        &app.get_binary_path(profile),
    );

    let pid = execute_launch(app_name, profile, port, &app, manifest_dir)?;

    create_success_result(
        app_name,
        profile,
        &app,
        manifest_dir,
        pid,
        launch_start,
        port,
    )
}

enum AppDiscoveryResult {
    Found(BevyTarget),
    DisambiguationError(LaunchBevyAppResult),
}

fn find_and_validate_app(
    app_name: &str,
    path: Option<&str>,
    search_paths: &[PathBuf],
    port: u16,
) -> Result<AppDiscoveryResult, McpError> {
    match scanning::find_required_target_with_path(app_name, TargetType::App, path, search_paths) {
        Ok(app) => Ok(AppDiscoveryResult::Found(app)),
        Err(mcp_error) => {
            if let Some(disambiguation_result) =
                handle_disambiguation_error(&mcp_error, app_name, port)
            {
                return Ok(AppDiscoveryResult::DisambiguationError(
                    disambiguation_result,
                ));
            }
            Err(mcp_error)
        }
    }
}

fn handle_disambiguation_error(
    mcp_error: &McpError,
    app_name: &str,
    port: u16,
) -> Option<LaunchBevyAppResult> {
    let error_msg = &mcp_error.message;
    if error_msg.contains("Found multiple") || error_msg.contains("not found at path") {
        let duplicate_paths = extract_duplicate_paths(error_msg);

        let error_response = super::support::create_disambiguation_error(
            "app",
            app_name,
            duplicate_paths.unwrap_or_default(),
        );

        return Some(LaunchBevyAppResult {
            status:             error_response["status"]
                .as_str()
                .unwrap_or("error")
                .to_string(),
            message:            error_response["message"].as_str().unwrap_or("").to_string(),
            app_name:           error_response["app_name"].as_str().map(String::from),
            pid:                None,
            port:               Some(port),
            working_directory:  None,
            profile:            None,
            log_file:           None,
            binary_path:        None,
            launch_duration_ms: None,
            launch_timestamp:   None,
            workspace:          None,
            duplicate_paths:    error_response["duplicate_paths"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
        });
    }
    None
}

fn extract_duplicate_paths(error_msg: &str) -> Option<Vec<String>> {
    if error_msg.contains("Found multiple") {
        let lines: Vec<&str> = error_msg.lines().collect();
        let mut paths = Vec::new();
        for line in &lines[1..] {
            if let Some(path) = line.strip_prefix("- ") {
                paths.push(path.to_string());
            }
        }
        if paths.is_empty() { None } else { Some(paths) }
    } else {
        None
    }
}

fn validate_binary_exists(app: &BevyTarget, profile: &str) -> Result<(), McpError> {
    let binary_path = app.get_binary_path(profile);
    if !binary_path.exists() {
        return Err(report_to_mcp_error(
            &error_stack::Report::new(Error::Configuration("Missing binary file".to_string()))
                .attach_printable(format!("Binary path: {}", binary_path.display()))
                .attach_printable(format!(
                    "Please build the app with 'cargo build{}' first",
                    if profile == PROFILE_RELEASE {
                        " --release"
                    } else {
                        ""
                    }
                )),
        ));
    }
    Ok(())
}

fn log_launch_info(
    app_name: &str,
    manifest_dir: &std::path::Path,
    profile: &str,
    binary_path: &std::path::Path,
) {
    debug!("Launching app {} from {}", app_name, manifest_dir.display());
    debug!("Working directory: {}", manifest_dir.display());
    debug!("CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    debug!("Profile: {}", profile);
    debug!("Binary path: {}", binary_path.display());
}

fn execute_launch(
    app_name: &str,
    profile: &str,
    port: u16,
    app: &BevyTarget,
    manifest_dir: &std::path::Path,
) -> Result<u32, McpError> {
    let binary_path = app.get_binary_path(profile);

    let (_, log_file_for_redirect) = launch_common::setup_launch_logging(
        app_name,
        "App",
        profile,
        &binary_path,
        manifest_dir,
        Some(port),
        None,
    )?;

    let cmd = launch_common::build_app_command(&binary_path, Some(port));

    process::launch_detached_process(
        &cmd,
        manifest_dir,
        log_file_for_redirect,
        app_name,
        "launch",
    )
}

fn create_success_result(
    app_name: &str,
    profile: &str,
    app: &BevyTarget,
    manifest_dir: &std::path::Path,
    pid: u32,
    launch_start: Instant,
    port: u16,
) -> Result<LaunchBevyAppResult, McpError> {
    let launch_end = Instant::now();
    let launch_duration_ms = launch_end.duration_since(launch_start).as_millis();

    debug!("Launch duration: {}ms", launch_duration_ms);

    debug!("Environment variable: BRP_PORT={}", port);

    let launch_duration_ms = u64::try_from(launch_duration_ms).unwrap_or(u64::MAX);
    let launch_timestamp = chrono::Utc::now().to_rfc3339();

    let workspace = app
        .workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(String::from);

    let binary_path = app.get_binary_path(profile);
    let (log_file_path, _) = launch_common::setup_launch_logging(
        app_name,
        "App",
        profile,
        &binary_path,
        manifest_dir,
        Some(port),
        None,
    )?;

    Ok(LaunchBevyAppResult {
        status: "success".to_string(),
        message: format!("Successfully launched '{app_name}' (PID: {pid})"),
        app_name: Some(app_name.to_string()),
        pid: Some(pid),
        port: Some(port),
        working_directory: Some(manifest_dir.display().to_string()),
        profile: Some(profile.to_string()),
        log_file: Some(log_file_path.display().to_string()),
        binary_path: Some(binary_path.display().to_string()),
        launch_duration_ms: Some(launch_duration_ms),
        launch_timestamp: Some(launch_timestamp),
        workspace,
        duplicate_paths: None,
    })
}
