use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use error_stack::Report;
use tracing::debug;
use tracing::warn;

use super::build;
use super::config;
use crate::app_tools::launch_params::LaunchBevyBinaryParams;
use crate::app_tools::launch_params::SearchOrder;
use crate::app_tools::support::cargo_detector::BevyTarget;
use crate::app_tools::support::cargo_detector::TargetType;
use crate::app_tools::support::errors::AvailableTarget;
use crate::app_tools::support::errors::UnifiedTargetNotFoundError;
use crate::app_tools::support::process;
use crate::app_tools::support::scanning;
use crate::brp_tools::MAX_VALID_PORT;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;

fn prepare_launch_environment<T: config::LaunchConfigTrait>(
    config: &T,
    target: &BevyTarget,
) -> Result<(Command, PathBuf, PathBuf, std::fs::File)> {
    let manifest_dir = build::validate_manifest_directory(&target.manifest_path)?;
    let cmd = config.build_command(target);
    let (log_file_path, log_file_for_redirect) = build::setup_launch_logging(
        config.target_name(),
        T::TARGET_TYPE,
        config.profile(),
        &PathBuf::from(format!("{cmd:?}")),
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

fn validate_port_range(base_port: u16, instance_count: u16) -> Result<()> {
    if base_port.saturating_add(instance_count.saturating_sub(1)) > MAX_VALID_PORT {
        return Err(Error::tool_call_failed(format!(
            "Port range {base_port} to {} exceeds maximum valid port {MAX_VALID_PORT}",
            base_port.saturating_add(instance_count.saturating_sub(1)),
        ))
        .into());
    }
    Ok(())
}

fn launch_instances<T: config::LaunchConfigTrait>(
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
        let mut instance_config = config.clone();
        instance_config.set_port(port);

        let (cmd, manifest_dir, log_file_path, log_file_for_redirect) =
            prepare_launch_environment(&instance_config, target)?;

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

fn handle_target_discovery_error(error: Report<Error>) -> Report<Error> {
    if let Error::Structured { .. } = error.current_context() {
        return error;
    }

    let error_message = format!("{}", error.current_context());
    let details = serde_json::json!({
        "error": error_message,
        "error_chain": format!("{:?}", error)
    });
    Error::tool_call_failed_with_details(error_message, details).into()
}

pub fn launch_bevy_target(
    typed_params: LaunchBevyBinaryParams,
    roots: Vec<PathBuf>,
    default_profile: &'static str,
) -> Result<config::LaunchResult> {
    let params = typed_params.to_launch_params(default_profile);

    let search_roots = params
        .path
        .as_ref()
        .map_or(roots, |path| vec![PathBuf::from(path)]);

    let (first, second) = match params.search_order {
        SearchOrder::App => (TargetType::App, TargetType::Example),
        SearchOrder::Example => (TargetType::Example, TargetType::App),
    };

    let scope_path: Option<PathBuf> = params.path.as_ref().map(PathBuf::from);

    let mut first_targets =
        scanning::find_all_targets_by_name(&params.target_name, Some(first), &search_roots);
    if let Some(ref scope) = scope_path {
        first_targets = scanning::filter_targets_by_path_scope(first_targets, scope);
    }
    if !first_targets.is_empty() {
        return launch_found_target(first, first_targets, &params, &search_roots);
    }

    let mut second_targets =
        scanning::find_all_targets_by_name(&params.target_name, Some(second), &search_roots);
    if let Some(ref scope) = scope_path {
        second_targets = scanning::filter_targets_by_path_scope(second_targets, scope);
    }
    if !second_targets.is_empty() {
        return launch_found_target(second, second_targets, &params, &search_roots);
    }

    let mut all_targets = scanning::collect_all_bevy_targets(&search_roots);
    if let Some(ref scope) = scope_path {
        all_targets = scanning::filter_targets_by_path_scope(all_targets, scope);
    }
    let available: Vec<AvailableTarget> = all_targets
        .into_iter()
        .map(|target| AvailableTarget {
            name: target.name,
            kind: target.target_type.to_string(),
            path: target.relative_path.to_string_lossy().to_string(),
        })
        .collect();

    let error = UnifiedTargetNotFoundError::new(params.target_name, available);
    Err(Error::Structured {
        result: Box::new(error),
    }
    .into())
}

fn launch_found_target(
    target_type: TargetType,
    cached_targets: Vec<BevyTarget>,
    params: &config::LaunchParams,
    roots: &[PathBuf],
) -> Result<config::LaunchResult> {
    match target_type {
        TargetType::App => {
            let config =
                <config::LaunchConfig<config::App> as config::FromLaunchParams>::from_params(
                    params,
                );
            launch_target_with_cached(&config, roots, cached_targets)
        },
        TargetType::Example => {
            let config =
                <config::LaunchConfig<config::Example> as config::FromLaunchParams>::from_params(
                    params,
                );
            launch_target_with_cached(&config, roots, cached_targets)
        },
    }
}

fn launch_target_with_cached<T: config::LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
    cached_targets: Vec<BevyTarget>,
) -> Result<config::LaunchResult> {
    let launch_start = Instant::now();

    debug!("Environment variable: BRP_EXTRAS_PORT={}", config.port());

    let target = find_and_validate_target_with_cache(config, search_paths, cached_targets)
        .map_err(handle_target_discovery_error)?;

    let build_state = config.ensure_built(&target)?;
    match build_state {
        build::BuildState::Fresh => debug!("Target was already up to date, launching immediately"),
        build::BuildState::Rebuilt => debug!("Target was rebuilt before launch"),
        build::BuildState::NotFound => {
            warn!("Target not found in build output but build succeeded");
        },
    }

    let instance_count = *config.instance_count();
    let base_port = *config.port();

    validate_port_range(base_port, instance_count)?;

    let (all_pids, all_log_files, all_ports) =
        launch_instances(config, &target, instance_count, base_port)?;

    Ok(config::build_launch_result(
        all_pids,
        all_log_files,
        all_ports,
        config,
        &target,
        launch_start,
    ))
}

fn find_and_validate_target_with_cache<T: config::LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
    cached_targets: Vec<BevyTarget>,
) -> Result<BevyTarget> {
    scanning::find_required_target_with_package_name(
        config.target_name(),
        T::TARGET_TYPE,
        config.package_name(),
        search_paths,
        Some(cached_targets),
    )
    .map_err(Report::new)
}
