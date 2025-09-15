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

use std::ops::RangeInclusive;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use crate::brp_tools::Port;

/// Minimum number of ports (1)
pub const MIN_PORTS_PER_REQUEST: usize = 1;
/// Maximum number of ports (100)
pub const MAX_PORTS_PER_REQUEST: usize = 100;
/// Valid range for port count
pub const VALID_PORT_COUNT_RANGE: RangeInclusive<usize> = MIN_PORTS_PER_REQUEST..=MAX_PORTS_PER_REQUEST;

/// A port parameter that accepts either a single port or an array of ports
///
/// This type enables backward compatibility by accepting:
/// - Single port: `{"port": 15702}`
/// - Multiple ports: `{"port": [15702, 15703, 15704]}`
/// - Validates port count (1-100 ports)
#[derive(Debug, Clone, JsonSchema)]
#[serde(untagged)]
pub enum PortOrPorts {
    Single(Port),
    #[serde(deserialize_with = "deserialize_validated_ports")]
    Multiple(Vec<Port>),
}

fn deserialize_validated_ports<'de, D>(deserializer: D) -> Result<Vec<Port>, D::Error>
where
    D: Deserializer<'de>,
{
    let ports = Vec::<Port>::deserialize(deserializer)?;

    if ports.is_empty() {
        return Err(serde::de::Error::custom("Port array cannot be empty"));
    }

    if ports.len() > MAX_PORTS_PER_REQUEST {
        return Err(serde::de::Error::custom(format!(
            "Too many ports: {} (max: {})",
            ports.len(), MAX_PORTS_PER_REQUEST
        )));
    }

    Ok(ports)
}

impl PortOrPorts {
    /// Convert to a vector of ports regardless of input format
    pub fn to_ports(self) -> Vec<Port> {
        match self {
            PortOrPorts::Single(port) => vec![port],
            PortOrPorts::Multiple(ports) => ports, // Already validated during deserialization
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

### 2.1 Create Multi-Port Result Structure
**File**: `mcp/src/app_tools/brp_status.rs` (modify existing)

```rust
// Multi-port result preserving individual port results
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct MultiPortStatusResult {
    #[to_metadata]
    app_name: String,

    // Array of individual port results - each exactly like single-port today
    #[to_result]
    ports: Vec<PortStatusResult>,

    // Simple summary counts
    #[to_metadata]
    summary: StatusSummary,

    #[to_message]
    message_template: String,
}

// Individual port result - either success or our structured error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStatusResult {
    pub port: u16,

    // Success case: full StatusResult with pid, message, etc.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub success: Option<StatusResult>,

    // Error case: Our structured error types (ProcessNotFoundError, BrpNotRespondingError, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<dyn ResultStruct>>,
}

// Simple summary without complex enums
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSummary {
    pub total_ports: usize,
    pub successful: usize,
    pub app_running_no_brp: usize,
    pub app_not_found: usize,
}
```

### 2.2 Implementation Preserving Current Single-Port Logic
```rust
impl MultiPortStatusResult {
    pub fn from_port_checks(app_name: String, port_results: Vec<(u16, Result<StatusResult, Error>)>) -> Self {
        // Convert each port result to PortStatusResult - preserving all original info
        let ports: Vec<PortStatusResult> = port_results.into_iter()
            .map(|(port, result)| {
                match result {
                    Ok(status_result) => PortStatusResult {
                        port, // Use actual port from the operation context
                        success: Some(status_result),
                        error: None,
                    },
                    Err(error) => PortStatusResult {
                        port, // Use actual port from the operation context - never use 0
                        success: None,
                        error: error.downcast::<Box<dyn ResultStruct>>().ok(),
                    },
                }
            })
            .collect();

        let summary = StatusSummary::from_ports(&ports);
        let message_template = Self::generate_message(&app_name, &summary);

        Self {
            app_name,
            ports,
            summary,
            message_template,
        }
    }

    // Simple message generation based on counts
    fn generate_message(app_name: &str, summary: &StatusSummary) -> String {
        match (summary.successful, summary.total_ports) {
            (s, t) if s == t && t == 1 => {
                format!("Process '{app_name}' is running with BRP enabled")
            },
            (s, t) if s == t => {
                format!("Process '{app_name}' is running with BRP enabled on all {t} ports")
            },
            (s, t) if s > 0 => {
                format!("Process '{app_name}' status: {s}/{t} ports with BRP enabled")
            },
            (0, _) => {
                format!("Process '{app_name}' not responding on any checked ports")
            },
        }
    }
}

impl StatusSummary {
    fn from_ports(ports: &[PortStatusResult]) -> Self {
        let total_ports = ports.len();
        let successful = ports.iter().filter(|p| p.success.is_some()).count();
        let (app_running_no_brp, app_not_found) = ports.iter()
            .filter(|p| p.error.is_some())
            .fold((0, 0), |(brp_count, not_found_count), port| {
                if let Some(ref error) = port.error {
                    // Check the actual error type using our structured error system
                    if error.downcast_ref::<BrpNotRespondingError>().is_some() {
                        (brp_count + 1, not_found_count)
                    } else {
                        (brp_count, not_found_count + 1)
                    }
                } else {
                    (brp_count, not_found_count)
                }
            });

        Self {
            total_ports,
            successful,
            app_running_no_brp,
            app_not_found,
        }
    }
}
```

## 3. Implementation Logic Changes

### 3.1 Update Main Handler Function
**File**: `mcp/src/app_tools/brp_status.rs`

```rust
async fn handle_impl(params: StatusParams) -> Result<MultiPortStatusResult> {
    let ports = params.port.to_ports();
    let app_name = &params.app_name;

    // Check all ports in parallel for efficiency
    let port_checks: Vec<_> = ports
        .into_iter()
        .map(|port| async move {
            let result = check_brp_for_app_on_single_port(app_name, port).await;
            (port.0, result)
        })
        .collect();

    let port_results = futures::future::join_all(port_checks).await;

    Ok(MultiPortStatusResult::from_port_checks(app_name.to_string(), port_results))
}

// Keep existing single-port function unchanged
async fn check_brp_for_app_on_single_port(app_name: &str, port: Port) -> Result<StatusResult> {
    // Current implementation logic unchanged - returns existing StatusResult or structured errors
    // ... existing logic ...
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

Response structure:
- ports: Array of individual port results (each preserves full success/error information)
- summary: Simple counts for quick assessment

Each port result contains either:
- success: Full StatusResult with pid, port, app_name, message_template (same as single-port today)
- error: Structured error information with suggestions and context

Summary fields:
- total_ports: Total number of ports checked
- successful: Number of ports with working BRP
- app_running_no_brp: Number of ports with app but no BRP
- app_not_found: Number of ports where app is not running
- app_name: Checked app name

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

**Single Port (New Structure)**:
```json
{
  "status": "success",
  "message": "Process 'extras_plugin' is running with BRP enabled",
  "call_info": { "mcp_tool": "brp_status" },
  "metadata": {
    "app_name": "extras_plugin",
    "summary": {
      "total_ports": 1,
      "successful": 1,
      "app_running_no_brp": 0,
      "app_not_found": 0
    }
  },
  "parameters": {
    "app_name": "extras_plugin",
    "port": 15702
  },
  "result": [
    {
      "port": 15702,
      "success": {
        "app_name": "extras_plugin",
        "pid": 22687,
        "port": 15702,
        "message_template": "Process 'extras_plugin' (PID: 22687) is running with BRP enabled on port 15702"
      }
    }
  ]
}
```

**Multi-Port Example**:
```json
{
  "status": "success",
  "message": "Process 'extras_plugin' is running with BRP enabled on all 3 ports",
  "call_info": { "mcp_tool": "brp_status" },
  "metadata": {
    "app_name": "extras_plugin",
    "summary": {
      "total_ports": 3,
      "successful": 3,
      "app_running_no_brp": 0,
      "app_not_found": 0
    }
  },
  "parameters": {
    "app_name": "extras_plugin",
    "port": [15702, 15703, 15704]
  },
  "result": [
    {
      "port": 15702,
      "success": {
        "app_name": "extras_plugin",
        "pid": 22687,
        "port": 15702,
        "message_template": "Process 'extras_plugin' (PID: 22687) is running with BRP enabled on port 15702"
      }
    },
    {
      "port": 15703,
      "success": {
        "app_name": "extras_plugin",
        "pid": 22688,
        "port": 15703,
        "message_template": "Process 'extras_plugin' (PID: 22688) is running with BRP enabled on port 15703"
      }
    },
    {
      "port": 15704,
      "error": {
        "app_name": "extras_plugin",
        "similar_processes": ["bevy_example", "bevy_test"],
        "brp_responding_on_port": false,
        "port": 15704,
        "message_template": "Process 'extras_plugin' not found. Similar processes found: bevy_example, bevy_test. Check if the app is running or verify the app name is correct."
      }
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
- ✅ `result` array with detailed per-port status (preserving individual StatusResult objects)
- ✅ `metadata.summary` with simple count fields for status overview
- ✅ Individual port results maintain all original success/error information

**Testing After Implementation**:
1. Launch same `extras_plugin` example
2. Call `brp_status` with single port `{"app_name": "extras_plugin", "port": 15702}`
3. Verify all critical fields are preserved
4. Verify new fields provide enhanced functionality
5. Test multi-port scenario with `{"app_name": "extras_plugin", "port": [15702, 15703, 15704]}`

---

## Design Review Skip Notes

## DESIGN-1: Response structure breaking change without proper backward compatibility - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Response Structure Design
- **Issue**: The plan changes the response from individual metadata fields (pid, port) to an array-based structure (ports array). This breaks existing tooling that expects metadata.pid and metadata.port fields.
- **Reasoning**: No existing tooling depends on the current response structure. This is new functionality being added, so there are no backward compatibility concerns. The new multi-port structure is appropriate for the feature being implemented.
- **Decision**: User elected to skip this recommendation

---

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