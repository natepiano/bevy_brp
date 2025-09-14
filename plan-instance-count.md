# Plan: Multi-Instance Launch Support (instance_count)

## Overview
Add support for launching multiple instances of the same Bevy app/example on sequential ports using an `instance_count` parameter.

## Design Principles
1. **Simplicity**: Keep the existing code structure intact - just loop the existing launch logic
2. **Clean Design**: Use array-based structure for instances (breaking change accepted)
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
  - Constants for min/max/range validation (MIN_INSTANCE_COUNT, MAX_INSTANCE_COUNT, VALID_INSTANCE_RANGE)

### 2. Add Module Registration
**File**: `mcp/src/app_tools/mod.rs`
```rust
mod instance_count;
// Note: Don't need pub use - internal only
```

### 3. Update Parameter Structs
**Files**: `brp_launch_bevy_app.rs`, `brp_launch_bevy_example.rs`
```rust
// Add import at the top of both files:
use crate::app_tools::instance_count::InstanceCount;

// Add to both LaunchBevyAppParams and LaunchBevyExampleParams:
/// Number of instances to launch (default: 1)
#[serde(default)]
pub instance_count: InstanceCount,

// Update ToLaunchParams impl to pass instance_count:
impl ToLaunchParams for LaunchBevyAppParams {
    fn to_launch_params(&self, default_profile: &str) -> LaunchParams {
        LaunchParams {
            target_name: self.app_name.clone(),
            profile: self.profile.clone().unwrap_or_else(|| default_profile.to_string()),
            path: self.path.clone(),
            port: self.port,
            instance_count: self.instance_count,
        }
    }
}
```

### 4. Update Support Structures
**File**: `support/launch_common.rs`

#### 4a. LaunchParams
```rust
use crate::app_tools::instance_count::InstanceCount;

pub struct LaunchParams {
    // ... existing fields ...
    pub instance_count: InstanceCount,
}
```

#### 4b. LaunchConfig
```rust
#[derive(Clone)]  // Add Clone derive
pub struct LaunchConfig<T> {
    // ... existing fields ...
    pub instance_count: InstanceCount,
}

// Update new() constructor to accept instance_count:
impl<T> LaunchConfig<T> {
    pub fn new(
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

// Update FromLaunchParams impls:
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
```

#### 4c. LaunchConfigTrait
```rust
trait LaunchConfigTrait: Clone {  // Add Clone bound
    // ... existing methods ...

    /// Get the instance count for launching multiple instances
    fn instance_count(&self) -> InstanceCount;

    /// Set the port (needed for multi-instance launches)
    fn set_port(&mut self, port: Port);
}

// Implement in both LaunchConfig<App> and LaunchConfig<Example>:
impl<T> LaunchConfigTrait for LaunchConfig<T> {
    // ... existing method implementations ...

    fn instance_count(&self) -> InstanceCount {
        self.instance_count
    }

    fn set_port(&mut self, port: Port) {
        self.port = port;
    }
}
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
    use crate::brp_tools::constants::MAX_VALID_PORT;

    let launch_start = Instant::now();

    // Find and validate the target
    let target = find_and_validate_target(config, search_paths)?;

    // Ensure the target is built
    config.ensure_built(&target)?;

    let instance_count = config.instance_count().0;
    let base_port = config.port().0;

    // Validate entire port range fits within valid bounds
    // MAX_VALID_PORT is imported from brp_tools::constants (65534)
    if base_port.saturating_add(instance_count as u16 - 1) > MAX_VALID_PORT {
        return Err(Error::tool_call_failed(
            format!("Port range {} to {} exceeds maximum valid port {}",
                    base_port,
                    base_port.saturating_add(instance_count as u16 - 1),
                    MAX_VALID_PORT)
        ));
    }

    // SINGLE UNIFIED LOOP - handles both single (1) and multi (N) instances
    let mut all_pids = Vec::new();
    let mut all_log_files = Vec::new();
    let mut all_ports = Vec::new();

    for i in 0..instance_count {
        let port = Port(config.port().0 + i as u16); // Now safe after validation

        // Create a modified config with the updated port for this instance
        let mut instance_config = config.clone();
        instance_config.set_port(port);

        // Prepare launch environment with the instance-specific config
        let (mut cmd, manifest_dir, log_file_path, log_file_for_redirect) =
            prepare_launch_environment(&instance_config, &target)?;

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

#### 5c. Build unified result from collected vectors
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

- **BREAKING CHANGE**: LaunchResult structure changes from single instance fields (pid, log_file) to instances array
- instance_count is optional and defaults to 1 (single instance behavior)
- Consumers need to update to access instances[0].pid instead of pid directly

## Design Review Findings

### TYPE-SYSTEM-1: Terminology inconsistency between InstanceCount type and internal constants - **Verdict**: CONFIRMED ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: Create InstanceCount Type
- **Issue**: The `InstanceCount` type uses 'SEQUENCE_COUNT' terminology throughout constants, functions, and internal structures, but 'InstanceCount' in the type name, creating pervasive inconsistent naming that violates type system design principles
- **Reasoning**: This is definitely a real issue that's actually worse than originally described. The inconsistency isn't just in the constants - it permeates the entire module. The public type is called 'InstanceCount' but internally everything uses 'sequence' terminology: constants, functions, visitor structs, error messages, and comments. This creates confusion for anyone reading or maintaining the code. When a type is called 'InstanceCount', users expect all related functionality to use 'instance' terminology consistently. This violates the principle of least surprise and makes the API harder to understand.

### Approved Change:
All "sequence" terminology has been changed to "instance" terminology throughout:
- Constants: MIN_INSTANCE_COUNT, MAX_INSTANCE_COUNT, VALID_INSTANCE_RANGE
- Functions: deserialize_instance_count
- Visitor struct: InstanceCountVisitor
- Error messages: "instance count" instead of "sequence count"

### Implementation Notes:
The instance_count.rs file has been updated to use consistent "instance" terminology throughout. The plan has been updated to reflect these changes.

### TYPE-SYSTEM-2: Missing port overflow validation in sequential port assignment - **Verdict**: CONFIRMED ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: Modify launch_target Function - UNIFIED LOOP APPROACH
- **Issue**: Plan shows `port + i as u16` without validation that the result stays within valid port range (1-65534), which could cause port overflow or invalid ports
- **Reasoning**: This finding correctly identifies a real vulnerability in the planned multi-instance launch feature. While the problematic code doesn't exist in the current implementation, the plan document shows code that would bypass Port validation by directly constructing Port(value) without checking if the calculated ports stay within the valid range. If a base port of 65530 was used with 10 instances, the final port would be 65540, exceeding the maximum valid port.

### Approved Change:
Added port range validation before the loop to ensure all calculated ports stay within valid bounds (1024-65534). Uses the existing MAX_VALID_PORT constant from brp_tools::constants.

### Implementation Notes:
The launch_target function now validates the entire port range before entering the loop, using saturating_add to prevent integer overflow during validation.

## Design Review Skip Notes

### TYPE-SYSTEM-3: Backward compatibility break in LaunchResult structure - **Verdict**: REJECTED
- **Status**: SKIPPED - Invalid finding after design clarification
- **Location**: Section: New LaunchResult Structure - Clean Instance Array
- **Issue**: Plan removes `pid: Option<u32>` field from LaunchResult and replaces with `instances: Vec<LaunchedInstance>`, breaking existing consumers despite claiming backward compatibility
- **Reasoning**: The finding was based on backward compatibility claims in the plan. After review, we've removed those claims and acknowledged this as an intentional breaking change. The clean array-based design is preferred over maintaining legacy fields, and breaking changes are acceptable for major new features.
- **Decision**: Plan updated to clearly mark this as a breaking change in Migration Notes. The clean instances array design is retained as originally planned.

### DESIGN-1: Missing instance_count field in LaunchParams struct - **Verdict**: MODIFIED ✅
- **Status**: APPROVED - To be implemented (with modifications)
- **Location**: Section: Update Support Structures
- **Issue**: Plan shows adding instance_count to LaunchParams but current struct doesn't include it, and plan doesn't show the required update
- **Reasoning**: The finding correctly identified the missing field, but the suggested implementation was wrong. Just like Port is not optional and has a default value, InstanceCount should also be non-optional with its default of 1. This maintains consistency in the API design.

### Approved Change:
InstanceCount is added as a non-optional field with #[serde(default)] attribute, exactly like Port:
- LaunchParams: `pub instance_count: InstanceCount`
- LaunchBevyAppParams/LaunchBevyExampleParams: `#[serde(default)] pub instance_count: InstanceCount`
- Uses Default implementation returning InstanceCount(1)

### Implementation Notes:
The plan has been updated to show InstanceCount as a non-optional field throughout, maintaining consistency with the Port pattern. All ToLaunchParams and FromLaunchParams implementations updated to pass instance_count.

### DESIGN-2: Missing instance_count method in LaunchConfigTrait - **Verdict**: CONFIRMED ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: LaunchConfigTrait
- **Issue**: Plan references config.instance_count() method but LaunchConfigTrait doesn't define this method, and plan doesn't show adding it
- **Reasoning**: This is a real issue where the plan document shows the LaunchConfigTrait should have an instance_count() method, but the actual trait definition is missing this method. The plan references config.instance_count() in the launch logic, but the trait doesn't define this method.

### Approved Change:
Added instance_count() method to LaunchConfigTrait with proper documentation and implementation.

### Implementation Notes:
The trait method returns InstanceCount (non-optional) and the implementation simply returns the stored instance_count field from LaunchConfig.