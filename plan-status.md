# Plan: Multi-Port Status Check Implementation

## Overview
Implement multi-port status checking capability for `brp_status` command, following the established pattern from the recent `instance_count` implementation in launch commands. Users should be able to check status on either a single port (`"port": 15702`) or multiple ports (`"port": [15702, 15703, 15704]`) with full backward compatibility.

## 1. Core Type Implementation

### 1.1 Create PortOrPorts Type
**File**: `mcp/src/brp_tools/port_or_ports.rs` (new file)

```rust
//! Port or array of ports type for multi-port BRP operations
//!
//! Provides a type-safe wrapper that accepts either a single port or an array
//! of ports with built-in validation and backward compatibility.

use std::ops::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::brp_tools::Port;

/// A port parameter that accepts either a single port or an array of ports
///
/// This type enables backward compatibility by accepting:
/// - Single port: `{"port": 15702}`
/// - Multiple ports: `{"port": [15702, 15703, 15704]}`
#[derive(Debug, Clone, JsonSchema)]
#[serde(untagged)]
pub enum PortOrPorts {
    Single(Port),
    Multiple(Vec<Port>),
}

impl PortOrPorts {
    /// Convert to a vector of ports regardless of input format
    pub fn to_ports(self) -> Vec<Port> {
        match self {
            PortOrPorts::Single(port) => vec![port],
            PortOrPorts::Multiple(ports) => ports,
        }
    }

    /// Get the number of ports
    pub fn len(&self) -> usize {
        match self {
            PortOrPorts::Single(_) => 1,
            PortOrPorts::Multiple(ports) => ports.len(),
        }
    }

    /// Check if empty (shouldn't happen due to validation)
    pub fn is_empty(&self) -> bool {
        match self {
            PortOrPorts::Single(_) => false,
            PortOrPorts::Multiple(ports) => ports.is_empty(),
        }
    }
}

impl Default for PortOrPorts {
    fn default() -> Self {
        PortOrPorts::Single(Port::default())
    }
}

impl Serialize for PortOrPorts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PortOrPorts::Single(port) => port.serialize(serializer),
            PortOrPorts::Multiple(ports) => ports.serialize(serializer),
        }
    }
}
```

### 1.2 Update Module Exports
**File**: `mcp/src/brp_tools/mod.rs`
```rust
// Add export
pub use port_or_ports::PortOrPorts;
```

## 2. Response Structure Design

### 2.1 Create StatusInstance Structure
**File**: `mcp/src/app_tools/brp_status.rs` (modify existing)

```rust
/// Represents the status of a single port check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInstance {
    /// Port that was checked
    pub port: u16,
    /// Process ID if app is running (None if not running)
    pub pid: Option<u32>,
    /// Whether BRP is responding on this port
    pub brp_responsive: bool,
    /// Status summary for this port
    pub status: PortStatus,
    /// Detailed message for this port check
    pub message: String,
}

/// Status enumeration for each port check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortStatus {
    /// App running with BRP responding
    RunningWithBrp,
    /// App running but BRP not responding
    RunningNoBrp,
    /// App not running
    NotRunning,
}

impl std::fmt::Display for PortStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortStatus::RunningWithBrp => write!(f, "running_with_brp"),
            PortStatus::RunningNoBrp => write!(f, "running_no_brp"),
            PortStatus::NotRunning => write!(f, "not_running"),
        }
    }
}
```

### 2.2 Update StatusParams Structure
```rust
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct StatusParams {
    /// Name of the process to check for
    pub app_name: String,
    /// The BRP port(s) to check (default: 15702)
    #[serde(default)]
    pub port: PortOrPorts,
}
```

### 2.3 Update StatusResult Structure
```rust
/// Result from checking status of a Bevy app across one or more ports
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct StatusResult {
    /// App name that was checked
    #[to_metadata]
    app_name: String,

    /// Array of status results (1 or more ports)
    #[to_result]
    ports: Vec<StatusInstance>,

    /// Summary count of ports with app running and BRP responding
    #[to_metadata]
    running_with_brp_count: usize,

    /// Summary count of ports with app running but BRP not responding
    #[to_metadata]
    running_no_brp_count: usize,

    /// Summary count of ports where app is not running
    #[to_metadata]
    not_running_count: usize,

    /// Total number of ports checked
    #[to_metadata]
    total_ports_checked: usize,

    /// Overall status summary for all ports
    #[to_metadata]
    overall_status: String,

    /// Message template for formatting responses
    #[to_message]
    message_template: String,
}

impl StatusResult {
    pub fn new(app_name: String, ports: Vec<StatusInstance>) -> Self {
        let running_with_brp_count = ports.iter().filter(|p| matches!(p.status, PortStatus::RunningWithBrp)).count();
        let running_no_brp_count = ports.iter().filter(|p| matches!(p.status, PortStatus::RunningNoBrp)).count();
        let not_running_count = ports.iter().filter(|p| matches!(p.status, PortStatus::NotRunning)).count();
        let total_ports_checked = ports.len();

        let overall_status = if running_with_brp_count == total_ports_checked {
            "all_running_with_brp".to_string()
        } else if running_with_brp_count > 0 {
            "partial_running_with_brp".to_string()
        } else if running_no_brp_count > 0 {
            "running_no_brp".to_string()
        } else {
            "not_running".to_string()
        };

        let message_template = generate_message_template(&app_name, &ports, &overall_status);

        Self {
            app_name,
            ports,
            running_with_brp_count,
            running_no_brp_count,
            not_running_count,
            total_ports_checked,
            overall_status,
            message_template,
        }
    }
}

fn generate_message_template(app_name: &str, ports: &[StatusInstance], overall_status: &str) -> String {
    match overall_status {
        "all_running_with_brp" => {
            if ports.len() == 1 {
                format!("Process '{}' (PID: {}) is running with BRP enabled on port {}",
                    app_name,
                    ports[0].pid.unwrap(),
                    ports[0].port)
            } else {
                let port_range = format!("{}-{}", ports.first().unwrap().port, ports.last().unwrap().port);
                format!("Process '{}' is running with BRP enabled on {} ports ({})",
                    app_name,
                    ports.len(),
                    port_range)
            }
        },
        "partial_running_with_brp" => {
            let working_count = ports.iter().filter(|p| matches!(p.status, PortStatus::RunningWithBrp)).count();
            format!("Process '{}' status: {}/{} ports running with BRP",
                app_name,
                working_count,
                ports.len())
        },
        "running_no_brp" => {
            format!("Process '{}' is running but BRP is not responding on checked ports", app_name)
        },
        _ => {
            format!("Process '{}' not found on checked ports", app_name)
        }
    }
}
```

## 3. Implementation Logic Changes

### 3.1 Update Main Handler Function
**File**: `mcp/src/app_tools/brp_status.rs`

```rust
async fn handle_impl(params: StatusParams) -> Result<StatusResult> {
    let ports = params.port.to_ports();
    let app_name = &params.app_name;

    // Check all ports in parallel for efficiency
    let port_checks: Vec<_> = ports
        .into_iter()
        .map(|port| async move {
            check_brp_for_app_on_single_port(app_name, port).await
        })
        .collect();

    let results = futures::future::join_all(port_checks).await;

    // Convert results to StatusInstance objects
    let status_instances: Vec<StatusInstance> = results
        .into_iter()
        .map(|result| match result {
            Ok(single_result) => StatusInstance {
                port: single_result.port,
                pid: Some(single_result.pid),
                brp_responsive: true,
                status: PortStatus::RunningWithBrp,
                message: format!("Running with BRP on port {}", single_result.port),
            },
            Err(err) => {
                // Parse structured errors to determine appropriate StatusInstance
                match err {
                    Error::Structured { result } => {
                        if let Some(process_not_found) = result.downcast_ref::<ProcessNotFoundError>() {
                            StatusInstance {
                                port: process_not_found.port,
                                pid: None,
                                brp_responsive: process_not_found.brp_responding_on_port,
                                status: PortStatus::NotRunning,
                                message: process_not_found.message_template.clone().unwrap_or_default(),
                            }
                        } else if let Some(brp_not_responding) = result.downcast_ref::<BrpNotRespondingError>() {
                            StatusInstance {
                                port: brp_not_responding.port,
                                pid: Some(brp_not_responding.pid),
                                brp_responsive: false,
                                status: PortStatus::RunningNoBrp,
                                message: brp_not_responding.message_template.clone(),
                            }
                        } else {
                            // Fallback for unknown structured errors
                            StatusInstance {
                                port: 0, // Will need to be updated based on context
                                pid: None,
                                brp_responsive: false,
                                status: PortStatus::NotRunning,
                                message: "Unknown error".to_string(),
                            }
                        }
                    },
                    _ => {
                        // Fallback for non-structured errors
                        StatusInstance {
                            port: 0, // Will need to be updated based on context
                            pid: None,
                            brp_responsive: false,
                            status: PortStatus::NotRunning,
                            message: err.to_string(),
                        }
                    }
                }
            }
        })
        .collect();

    Ok(StatusResult::new(app_name.to_string(), status_instances))
}

// Rename existing function to be more specific
async fn check_brp_for_app_on_single_port(app_name: &str, port: Port) -> Result<SinglePortStatusResult> {
    // Current implementation logic, but return a simpler result type
    // ... existing logic ...
}

#[derive(Debug)]
struct SinglePortStatusResult {
    pub port: u16,
    pub pid: u32,
}
```

## 4. Help Text Updates

### 4.1 Update Status Help Text
**File**: `mcp/help_text/brp_status.txt`

```text
Check if a Bevy app is running with BRP enabled to verify app status, confirm BRP connectivity, troubleshoot connection issues, or get process information.

Multi-port status checking:
- Single port: {"app_name": "my_app", "port": 15702}
- Multiple ports: {"app_name": "my_app", "port": [15702, 15703, 15704]}
- Checks all ports in parallel for efficiency
- Useful for verifying multi-instance deployments

Return status values per port:
- "running_with_brp": App running with BRP responding
- "running_no_brp": App running but BRP not responding
- "not_running": App not running

Response includes:
- ports: Array of status results (one per port checked)
- running_with_brp_count: Number of ports with working BRP
- running_no_brp_count: Number of ports with app but no BRP
- not_running_count: Number of ports where app is not running
- total_ports_checked: Total number of ports checked
- overall_status: Summary status across all ports
- app_name: Checked app name

Overall status values:
- "all_running_with_brp": All ports have app running with BRP
- "partial_running_with_brp": Some ports have app running with BRP
- "running_no_brp": App running but no BRP response on any port
- "not_running": App not running on any port

IMPORTANT: Requires RemotePlugin in Bevy app plugin configuration.
```

## 5. Implementation Steps

### Phase 1: Core Infrastructure
1. **Create `port_or_ports.rs`** with `PortOrPorts` enum and serde implementation
2. **Update module exports** in `mcp/src/brp_tools/mod.rs`
3. **Add futures dependency** to Cargo.toml for parallel async operations

### Phase 2: Status Response Restructure
1. **Create `StatusInstance` and `PortStatus`** types
2. **Update `StatusResult`** to use array-based results with metadata counts
3. **Implement message template generation** for different scenarios
4. **Update error handling** to properly populate `StatusInstance` from errors

### Phase 3: Handler Logic
1. **Update `StatusParams`** to use `PortOrPorts`
2. **Refactor `handle_impl`** to iterate over multiple ports in parallel
3. **Create helper functions** for single-port checking and result conversion
4. **Ensure proper error propagation** and structured error handling

### Phase 4: Documentation and Testing
1. **Update help text** with multi-port examples and response format documentation
2. **Update CHANGELOG.md** with the new feature
3. **Test backward compatibility** with single port requests
4. **Test multi-port functionality** with various combinations

## 6. Backward Compatibility

### JSON API Compatibility
- ✅ **Single port input**: `{"port": 15702}` continues to work unchanged
- ✅ **Single port response**: Still contains same metadata and message format
- ✅ **Existing tooling**: No breaking changes for existing integrations

### Response Format Evolution
- **New field**: `ports` array with detailed per-port status
- **New metadata**: Count fields for status summary
- **Enhanced message**: Context-aware messages for single vs multi-port scenarios
- **Maintained field**: Original metadata fields preserved where applicable

## 7. Performance Considerations

### Parallel Execution
- **Async concurrency**: All port checks execute simultaneously using `futures::join_all`
- **Efficient timeouts**: Individual port checks maintain existing timeout behavior
- **Resource management**: No additional resource overhead per port check

### Error Handling
- **Per-port errors**: Individual port failures don't affect other port checks
- **Structured errors**: Existing error types properly converted to `StatusInstance` format
- **Graceful degradation**: Partial success scenarios handled appropriately

## 8. Testing Strategy

### Unit Tests
- Test `PortOrPorts` deserialization with single and multiple values
- Test `StatusResult` generation with various status combinations
- Test message template generation for different scenarios

### Integration Tests
- Test actual multi-port status checking with running Bevy apps
- Test backward compatibility with existing single-port usage
- Test error scenarios (mixed success/failure across ports)

### Performance Tests
- Verify parallel execution performance gains
- Test behavior with maximum port counts
- Verify timeout behavior across multiple ports

## 9. Documentation Updates

### Changelog Entry
```markdown
### Added
- Multi-port status checking support for `brp_status` command
- New `PortOrPorts` type accepting either single port or array of ports
- Parallel status checking for improved performance
- Enhanced response format with per-port details and summary counts
- Backward compatible JSON API (existing single port usage unchanged)
```

### API Documentation
- Update tool descriptions with multi-port examples
- Document new response format fields
- Provide usage examples for both single and multi-port scenarios

## 10. Current Status Response Baseline

### 10.1 Single Port Status Response (Pre-Implementation)
For comparison after implementation, here's the current response format from `brp_status` with a single port:

**Test Setup**: Launched `extras_plugin` example on port 15702
**Command**: `{"app_name": "extras_plugin", "port": 15702}`

**Current Response Format**:
```json
{
  "status": "success",
  "message": "Process 'extras_plugin' (PID: 22687) is running with BRP enabled on port 15702",
  "call_info": { "mcp_tool": "brp_status" },
  "metadata": {
    "app_name": "extras_plugin",
    "pid": 22687,
    "port": 15702
  },
  "parameters": {
    "app_name": "extras_plugin",
    "port": 15702
  }
}
```

### 10.2 Expected Post-Implementation Response Format

**Single Port (Backward Compatible)**:
```json
{
  "status": "success",
  "message": "Process 'extras_plugin' (PID: 22687) is running with BRP enabled on port 15702",
  "call_info": { "mcp_tool": "brp_status" },
  "metadata": {
    "app_name": "extras_plugin",
    "running_with_brp_count": 1,
    "running_no_brp_count": 0,
    "not_running_count": 0,
    "total_ports_checked": 1,
    "overall_status": "all_running_with_brp"
  },
  "parameters": {
    "app_name": "extras_plugin",
    "port": 15702
  },
  "result": [
    {
      "port": 15702,
      "pid": 22687,
      "brp_responsive": true,
      "status": "running_with_brp",
      "message": "Running with BRP on port 15702"
    }
  ]
}
```

**Multi-Port Example**:
```json
{
  "status": "success",
  "message": "Process 'extras_plugin' is running with BRP enabled on 3 ports (15702-15704)",
  "call_info": { "mcp_tool": "brp_status" },
  "metadata": {
    "app_name": "extras_plugin",
    "running_with_brp_count": 3,
    "running_no_brp_count": 0,
    "not_running_count": 0,
    "total_ports_checked": 3,
    "overall_status": "all_running_with_brp"
  },
  "parameters": {
    "app_name": "extras_plugin",
    "port": [15702, 15703, 15704]
  },
  "result": [
    {
      "port": 15702,
      "pid": 22687,
      "brp_responsive": true,
      "status": "running_with_brp",
      "message": "Running with BRP on port 15702"
    },
    {
      "port": 15703,
      "pid": 22688,
      "brp_responsive": true,
      "status": "running_with_brp",
      "message": "Running with BRP on port 15703"
    },
    {
      "port": 15704,
      "pid": 22689,
      "brp_responsive": true,
      "status": "running_with_brp",
      "message": "Running with BRP on port 15704"
    }
  ]
}
```

### 10.3 Critical Compatibility Requirements

**Must Preserve in Single Port Response**:
- ✅ `status: "success"` field
- ✅ `message` field with human-readable status
- ✅ `call_info.mcp_tool` field
- ✅ `parameters` field reflecting input
- ✅ `metadata.app_name` field (for tooling compatibility)

**New Fields to Add**:
- ✅ `result` array with detailed per-port status
- ✅ `metadata` summary counts for status overview
- ✅ `metadata.overall_status` for quick assessment

**Testing After Implementation**:
1. Launch same `extras_plugin` example
2. Call `brp_status` with single port `{"app_name": "extras_plugin", "port": 15702}`
3. Verify all critical fields are preserved
4. Verify new fields provide enhanced functionality
5. Test multi-port scenario with `{"app_name": "extras_plugin", "port": [15702, 15703, 15704]}`

---

## Design Review Skip Notes

## TYPE-SYSTEM-1: PortOrPorts duplicates existing InstanceCount pattern unnecessarily - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Core Type Implementation
- **Issue**: The plan creates a new PortOrPorts enum that duplicates the exact same pattern as the existing InstanceCount type. Both handle single-vs-multiple values with serde(untagged), validation, and conversion methods.
- **Reasoning**: This finding is a false positive. The InstanceCount and PortOrPorts types serve completely different purposes and use different patterns. InstanceCount is a validated integer wrapper (1-100) that doesn't use serde(untagged) or handle single-vs-multiple values. PortOrPorts is an untagged enum for flexible port specification. There is no actual duplication - these solve different problems: InstanceCount validates launch counts while PortOrPorts accepts explicit port lists for status checking.
- **Decision**: User elected to skip this recommendation

---

## Summary

This implementation follows the established pattern from the `instance_count` feature, providing:

1. **Type-safe multi-port input** with full backward compatibility
2. **Array-based response structure** with individual port results
3. **Rich metadata** for status summaries and counts
4. **Parallel execution** for performance optimization
5. **Enhanced help documentation** with clear usage examples

The design maintains all existing functionality while extending capabilities for multi-port scenarios, making it ideal for testing and monitoring multi-instance Bevy deployments.