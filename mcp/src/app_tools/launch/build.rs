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

use super::constants::BUILD_OUTPUT_FRESH_FIELD;
use super::constants::BUILD_OUTPUT_NAME_FIELD;
use super::constants::BUILD_OUTPUT_TARGET_FIELD;
use super::constants::CARGO_RELEASE_FLAG;
use super::logging;
use crate::app_tools::constants::CARGO_BUILD_SUBCOMMAND;
use crate::app_tools::constants::CARGO_COMMAND_NAME;
use crate::app_tools::constants::CARGO_MESSAGE_FORMAT_JSON_FLAG;
use crate::app_tools::constants::CARGO_WORKSPACE_FLAG;
use crate::app_tools::constants::PROFILE_RELEASE;
use crate::app_tools::targets::TargetType;
use crate::brp_tools::BRP_EXTRAS_PORT_ENV_VAR;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;

/// Which packages cargo selects when it builds a launch target.
///
/// Cargo unifies features over the packages it selects, not over the workspace, so
/// `cargo build --example x` run inside one package resolves a different feature set
/// for a shared dependency than `cargo build --workspace` resolves for the same
/// dependency. Those two sets compile to separate artifacts, so a developer who runs
/// `cargo clippy --workspace` and then launches a target compiles the dependency graph
/// twice. Selecting the whole workspace makes a launch build reuse what a workspace
/// build already produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuildScope {
    /// Select every workspace member, resolving features the way a bare
    /// `cargo build --workspace` does. Cargo applies `--bin` and `--example` to every
    /// package it selects, so this is only correct when the target name is unique
    /// within the workspace.
    Workspace,
    /// Select only the package that holds the target. Required when more than one
    /// member of the workspace defines a target under this name, because building
    /// both would write them to a single output path.
    Package,
}

impl BuildScope {
    /// Directory cargo runs in under this scope.
    ///
    /// Cargo selects packages relative to its working directory, so the workspace scope
    /// has to run from the workspace root for `--workspace` to mean the whole workspace.
    const fn build_dir<'a>(self, manifest_dir: &'a Path, workspace_root: &'a Path) -> &'a Path {
        match self {
            Self::Workspace => workspace_root,
            Self::Package => manifest_dir,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BuildState {
    NotFound,
    Fresh,
    Rebuilt,
}

pub(super) fn validate_manifest_directory(manifest_path: &Path) -> Result<&Path> {
    manifest_path.parent().ok_or_else(|| {
        Report::new(Error::FileOrPathNotFound(
            "Invalid manifest path".to_string(),
        ))
        .attach("No parent directory found")
        .attach(format!("Path: {}", manifest_path.display()))
    })
}

fn set_brp_env_vars(command: &mut Command, port: Option<Port>) {
    if let Some(port) = port {
        command.env(BRP_EXTRAS_PORT_ENV_VAR, port.to_string());
    }
}

fn set_user_env_vars(command: &mut Command, env: Option<&HashMap<String, String>>) {
    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            command.env(key, value);
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

pub(super) fn build_app_command(
    binary_path: &Path,
    port: Option<Port>,
    env: Option<&HashMap<String, String>>,
    command_line_arguments: Option<&[String]>,
) -> Command {
    let mut command = Command::new(binary_path);
    if let Some(user_arguments) = command_line_arguments {
        command.args(user_arguments);
    }
    set_brp_env_vars(&mut command, port);
    set_user_env_vars(&mut command, env);
    command
}

fn build_cargo_command(
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    manifest_dir: &Path,
    workspace_root: &Path,
    scope: BuildScope,
) -> Command {
    let mut command = Command::new(CARGO_COMMAND_NAME);
    command.current_dir(scope.build_dir(manifest_dir, workspace_root));
    command.arg(CARGO_BUILD_SUBCOMMAND);
    if scope == BuildScope::Workspace {
        command.arg(CARGO_WORKSPACE_FLAG);
    }

    target_type.add_cargo_args(&mut command, target_name);

    if profile == PROFILE_RELEASE {
        command.arg(CARGO_RELEASE_FLAG);
    }

    command.arg(CARGO_MESSAGE_FORMAT_JSON_FLAG);

    command
}

fn execute_build_command(
    command: &mut Command,
    target_name: &str,
    target_type: TargetType,
    profile: &str,
    build_dir: &Path,
) -> Result<Output> {
    debug!("Running cargo build for {target_type} '{target_name}' with command: {command:?}");

    let output = command.output().map_err(|e| {
        Error::ProcessManagement(format!(
            "Failed to run cargo build for {target_type} '{target_name}' (profile: {profile}, dir: {}): {e}",
            build_dir.display()
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ProcessManagement(format!(
            "Cargo build failed for {target_type} '{target_name}' (profile: {profile}, dir: {}): {stderr}",
            build_dir.display()
        ))
        .into());
    }

    Ok(output)
}

fn parse_build_output(stdout: &[u8], target_name: &str) -> BuildState {
    let stdout_str = String::from_utf8_lossy(stdout);

    for line in stdout_str.lines() {
        if let Ok(json) = serde_json::from_str::<Value>(line)
            && let Some(target) = json.get(BUILD_OUTPUT_TARGET_FIELD)
            && let Some(name) = target.get(BUILD_OUTPUT_NAME_FIELD)
            && name.as_str() == Some(target_name)
        {
            return json
                .get(BUILD_OUTPUT_FRESH_FIELD)
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
    workspace_root: &Path,
    scope: BuildScope,
) -> Result<BuildState> {
    let mut command = build_cargo_command(
        target_name,
        target_type,
        profile,
        manifest_dir,
        workspace_root,
        scope,
    );
    let output = execute_build_command(
        &mut command,
        target_name,
        target_type,
        profile,
        scope.build_dir(manifest_dir, workspace_root),
    )?;
    let build_state = parse_build_output(&output.stdout, target_name);
    log_build_result(build_state, target_name, target_type);

    Ok(build_state)
}
