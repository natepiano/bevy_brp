use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use error_stack::Report;
use serde_json::Value;
use tracing::debug;
use tracing::info;

use super::logging;
use crate::app_tools::targets::TargetType;
use crate::brp_tools::BRP_EXTRAS_PORT_ENV_VAR;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;

pub(super) fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path> {
    manifest_path.parent().ok_or_else(|| {
        Report::new(Error::FileOrPathNotFound(
            "Invalid manifest path".to_string(),
        ))
        .attach("No parent directory found")
        .attach(format!("Path: {}", manifest_path.display()))
    })
}

fn set_brp_env_vars(cmd: &mut Command, port: Option<Port>) {
    if let Some(port) = port {
        cmd.env(BRP_EXTRAS_PORT_ENV_VAR, port.to_string());
    }
}

fn set_user_env_vars(cmd: &mut Command, env: Option<&HashMap<String, String>>) {
    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }
}

pub(super) fn setup_launch_logging(
    name: &str,
    target_type: TargetType,
    profile: &str,
    binary_path: &Path,
    manifest_dir: &Path,
    port: Port,
    extra_log_info: Option<&str>,
) -> Result<(PathBuf, File)> {
    let (log_file_path, _) =
        logging::create_log_file(name, target_type, profile, binary_path, manifest_dir, port)
            .map_err(|e| Error::tool_call_failed(format!("Failed to create log file: {e}")))?;

    if let Some(extra_info) = extra_log_info {
        logging::append_to_log_file(&log_file_path, &format!("{extra_info}\n"))
            .map_err(|e| Error::tool_call_failed(format!("Failed to append to log file: {e}")))?;
    }

    let log_file_for_redirect =
        logging::open_log_file_for_redirect(&log_file_path).map_err(|e| {
            Error::tool_call_failed(format!("Failed to open log file for redirect: {e}"))
        })?;

    Ok((log_file_path, log_file_for_redirect))
}

pub(super) fn build_cargo_example_command(
    example_name: &str,
    profile: &str,
    port: Option<Port>,
    env: Option<&HashMap<String, String>>,
    args: Option<&[String]>,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example").arg(example_name);

    if profile == "release" {
        cmd.arg("--release");
    }

    if let Some(user_args) = args {
        cmd.arg("--").args(user_args);
    }

    set_brp_env_vars(&mut cmd, port);
    set_user_env_vars(&mut cmd, env);

    cmd
}

pub(super) fn build_app_command(
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

#[derive(Debug, Clone, Copy)]
pub(super) enum BuildState {
    NotFound,
    Fresh,
    Rebuilt,
}

fn build_cargo_command(
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    manifest_dir: &Path,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(manifest_dir);
    cmd.arg("build");

    target_type.add_cargo_args(&mut cmd, target_name);

    if profile == "release" {
        cmd.arg("--release");
    }

    cmd.arg("--message-format=json");

    cmd
}

fn execute_build_command(
    cmd: &mut Command,
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    manifest_dir: &Path,
) -> Result<Output> {
    debug!("Running cargo build for {target_type} '{target_name}' with args: {cmd:?}");

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

fn parse_build_output(stdout: &[u8], target_name: &str) -> BuildState {
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

fn log_build_result(build_state: BuildState, target_name: &str, target_type: TargetType) {
    match build_state {
        BuildState::NotFound => {
            debug!("Target '{target_name}' not found in build output, assuming it was built");
        },
        BuildState::Fresh => {
            debug!("{target_type} '{target_name}' was already up to date");
        },
        BuildState::Rebuilt => {
            info!("{target_type} '{target_name}' was built successfully");
        },
    }
}

pub(super) fn run_cargo_build(
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
