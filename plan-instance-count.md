# Plan: Multi-Instance Launch Support (instance_count)

## Overview
Add support for launching multiple instances of the same Bevy app/example on sequential ports using an `instance_count` parameter.

## Design Principles
1. **Simplicity**: Keep the existing code structure intact - just loop the existing launch logic
2. **Backward Compatible**: Default to 1 instance (existing behavior)
3. **Minimal Changes**: Reuse existing functions and structures where possible

## Implementation Steps

### 1. Create InstanceCount Type ✅ DRAFTED
**File**: `mcp/src/app_tools/instance_count.rs` (already created, not yet integrated)
- Type is fully implemented with:
  - `InstanceCount(pub usize)` wrapper type
  - Validation for 1-100 range
  - Default of 1 instance
  - Deserialize implementation with both number and string support
  - Display implementation
  - Constants for min/max/range validation

### 2. Add Module Registration
**File**: `mcp/src/app_tools/mod.rs`
```rust
mod instance_count;
// Note: Don't need pub use - internal only
```

### 3. Update Parameter Structs
**Files**: `brp_launch_bevy_app.rs`, `brp_launch_bevy_example.rs`
```rust
// Add to both LaunchBevyAppParams and LaunchBevyExampleParams:
#[to_metadata(skip_if_none)]
pub instance_count: Option<InstanceCount>,

// Update ToLaunchParams impl to pass it through
```

### 4. Update Support Structures
**File**: `support/launch_common.rs`

#### 4a. LaunchParams
```rust
pub struct LaunchParams {
    // ... existing fields ...
    pub instance_count: Option<InstanceCount>,
}
```

#### 4b. LaunchConfig
```rust
pub struct LaunchConfig<T> {
    // ... existing fields ...
    pub instance_count: Option<InstanceCount>,
}

// Update new() constructor
// Update FromLaunchParams impls
```

#### 4c. LaunchConfigTrait
```rust
trait LaunchConfigTrait {
    // ... existing methods ...
    fn instance_count(&self) -> Option<InstanceCount>;
}

// Implement in both LaunchConfig<App> and LaunchConfig<Example>
```

### 5. Core Launch Logic Changes
**File**: `support/launch_common.rs`

#### 5a. New LaunchResult Structure - Clean Instance Array
```rust
/// Represents a single launched instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchedInstance {
    pub pid: u32,
    pub log_file: String,
    pub port: u16,
}

/// Result of launching one or more instances
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct LaunchResult {
    // Core fields
    target_name: Option<String>,

    // Array of launched instances (1 or more)
    instances: Vec<LaunchedInstance>,

    // Common fields (same for all instances)
    working_directory: Option<String>,
    profile: Option<String>,
    binary_path: Option<String>,
    launch_duration_ms: Option<u64>,
    launch_timestamp: Option<String>,
    workspace: Option<String>,
    package_name: Option<String>,
    duplicate_paths: Option<Vec<String>>,

    // Update message template to show instance count
    #[to_message(message_template = "Successfully launched {target_name} ({instance_count} instance(s))")]
}

impl LaunchResult {
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
```

#### 5b. Modify launch_target Function - UNIFIED LOOP APPROACH
```rust
pub fn launch_target<T: LaunchConfigTrait>(
    config: &T,
    search_paths: &[PathBuf],
) -> Result<LaunchResult> {
    use std::time::Instant;
    let launch_start = Instant::now();

    // Find and validate the target
    let target = find_and_validate_target(config, search_paths)?;

    // Ensure the target is built
    config.ensure_built(&target)?;

    let instance_count = config.instance_count().map(|ic| ic.0).unwrap_or(1);

    // SINGLE UNIFIED LOOP - handles both single (1) and multi (N) instances
    let mut all_pids = Vec::new();
    let mut all_log_files = Vec::new();
    let mut all_ports = Vec::new();

    for i in 0..instance_count {
        let port = Port(config.port().0 + i as u16);

        // Prepare launch environment with specific port
        let (mut cmd, manifest_dir, log_file_path, log_file_for_redirect) =
            prepare_launch_environment_with_port(config, &target, port)?;

        // Set working directory and spawn process
        cmd.current_dir(&manifest_dir);
        cmd.stdout(log_file_for_redirect.try_clone()?);
        cmd.stderr(log_file_for_redirect);

        let child = cmd.spawn()?;
        let pid = child.id();

        all_pids.push(pid);
        all_log_files.push(log_file_path);
        all_ports.push(port.0);
    }

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
```

#### 5c. Helper Functions
```rust
// Modified prepare function that accepts port override
fn prepare_launch_environment_with_port<T: LaunchConfigTrait>(
    config: &T,
    target: &BevyTarget,
    port: Port,
) -> Result<(Command, PathBuf, PathBuf, std::fs::File)> {
    // Get manifest directory
    let manifest_dir = validate_manifest_directory(&target.manifest_path)?;

    // Build command with port override
    let mut cmd = config.build_command(target);
    cmd.env("BRP_EXTRAS_PORT", port.0.to_string());

    // Setup logging with port in filename
    let (log_file_path, log_file_for_redirect) = setup_launch_logging(
        config.target_name(),
        T::TARGET_TYPE,
        config.profile(),
        &PathBuf::from(format!("{cmd:?}")),
        manifest_dir,
        Some(port),  // Use provided port
        config.extra_log_info(target).as_deref(),
    )?;

    Ok((
        cmd,
        manifest_dir.to_path_buf(),
        log_file_path,
        log_file_for_redirect,
    ))
}

// Build unified result from collected vectors
fn build_launch_result<T: LaunchConfigTrait>(
    all_pids: Vec<u32>,
    all_log_files: Vec<PathBuf>,
    all_ports: Vec<u16>,
    config: &T,
    target: &BevyTarget,
    launch_start: Instant,
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

    LaunchResult {
        target_name: Some(config.target_name().to_string()),
        instances,

        // Common fields (same for all instances)
        working_directory: Some(std::env::current_dir().unwrap().display().to_string()),
        profile: Some(config.profile().to_string()),
        launch_duration_ms: Some(launch_duration.as_millis() as u64),
        launch_timestamp: Some(chrono::Utc::now().to_rfc3339()),
        workspace: target.workspace.clone(),
        package_name: target.package_name.clone(),
        binary_path: target.binary_path.as_ref().map(|p| p.display().to_string()),
        duplicate_paths: None,
        message_template: String::new(),
    }
}
```

### 6. Log File Naming
**File**: `support/logging.rs`
- Modify to include port in filename when multiple instances
- Format: `bevy_brp_mcp_{name}_port{port}_{timestamp}_{instance}.log`

### 7. Update Help Text
**Files**: `help_text/brp_launch_bevy_app.txt`, `help_text/brp_launch_bevy_example.txt`
- Document instance_count parameter
- Explain sequential port assignment
- Note use case for parallel testing

### 8. Update CHANGELOG.md
- Add entry under "Added" section about instance_count support

## Key Design Decisions

1. **Single Unified Loop**: Use one loop that runs 1 to N times, treating single-instance as just count=1
2. **Clean Data Structure**: Use a `LaunchedInstance` struct containing pid, log_file, and port, with `LaunchResult` containing an array of these
3. **Port Assignment**: Sequential from base port (port, port+1, port+2, ...)
4. **Validation**: Max 100 instances, ensure port+count doesn't exceed 65534
5. **Log Files**: Each instance gets unique log with port in filename

## Testing Strategy

1. Test single instance (default) - should work exactly as before
2. Test multiple instances (2-5) - verify all launch
3. Test max instances (100) - verify validation
4. Test port overflow (port 65500 + 100 instances) - should error
5. Test with both apps and examples

## Migration Notes

- Existing code continues to work unchanged
- instance_count is optional and defaults to 1
- Result structure is backward compatible (pid field still exists for single instance)