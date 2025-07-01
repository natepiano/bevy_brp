# Tracing and Debug Modes Comprehensive Test

## Objective
Validate the complete tracing and debug mode functionality including:

## Test Steps

### Pre-Test Setup
1. **Clean State**: Delete any existing trace log file
   - Use `mcp__brp__brp_get_trace_log_path` to get log file location
   - If `exists: true`, delete the file using the returned `log_path`
   - Verify `exists: false` after deletion to ensure clean test start

### Part A: "Do No Harm" Validation (Critical Test)

#### A1. Startup Behavior Test
- **Execute**: Basic operations after server startup
  - `mcp__brp__brp_list_brp_apps`
  - `mcp__brp__brp_list_bevy_examples`
  - `mcp__brp__bevy_list` (basic BRP call)
- **Verify**: Use `mcp__brp__brp_get_trace_log_path` to confirm `exists: false`
- **Expected Result**: No log file created (WARN default level prevents logging)
- **Failure Criteria**: If log file exists, normal operations triggered unwanted logging

#### A2. Extended Operations Test
- **Execute**: More BRP operations to ensure no logging occurs
  - `mcp__brp__bevy_get_resource` for any resource
  - `mcp__brp__bevy_query` with basic filter
- **Verify**: Use `mcp__brp__brp_get_trace_log_path` to confirm `exists: false`
- **Expected Result**: Normal operations don't trigger file creation

### Part B: MCP Tracing Level System Tests

#### B1. First Tracing Activation
- **Execute**: `mcp__brp__brp_set_tracing_level` with `level: "info"`
- **Verify**:
  - Use `mcp__brp__brp_get_trace_log_path` to confirm `exists: true`
  - Check returned `file_size_bytes` > 0
  - Read log file content to verify "Tracing level set to: info" message
  - No retroactive logging of previous operations
- **Expected Result**: Log file created only when explicitly requested

#### B2. INFO Level Behavior
- **Execute**: Various operations to generate INFO logs
  - `mcp__brp__brp_set_tracing_level` with `level: "debug"` (triggers info log)
  - Additional BRP operations
- **Verify**: Log contains INFO, WARN, ERROR level messages (no DEBUG/TRACE)

#### B3. DEBUG Level Behavior
- **Execute**: Operations that trigger DEBUG logs
  - `mcp__brp__bevy_spawn` with components (parameter logging)
  - `mcp__brp__bevy_get` operations
  - `mcp__brp__bevy_mutate_component` operations
- **Verify**: Log contains detailed parameter extraction/logging
- **Count**: Should see multiple DEBUG entries showing parameter details

### Part C: New Tool Validation

#### C1. brp_get_trace_log_path Tool Test
- **Execute**: `mcp__brp__brp_get_trace_log_path` (no parameters)
- **Verify**: Returns JSON with:
  - `log_path`: Valid file path to trace log
  - `exists`: Boolean matching actual file existence
  - `file_size_bytes`: Accurate file size when exists
- **Expected Result**: Tool provides reliable trace log information

## Expected Results Summary

### Critical Success Criteria
- ✅ **"Do No Harm"**: No log file created on startup or normal operations (WARN default)
- ✅ **MCP Tracing**: All levels filter correctly with immediate effect
- ✅ **New Tool**: `brp_get_trace_log_path` provides accurate log file information

### Log File Content Expectations
- **Default (WARN)**: 0 entries until user activates tracing
- **INFO**: Level change message + info logs from operations
- **DEBUG**: All above + detailed parameter logging

### Response Format Expectations
- **MCP Tracing**: Does not affect response structure, only creates log files

## Failure Criteria
**STOP if**:
- Log file created without explicit user request
- Tracing levels don't filter correctly (INFO/DEBUG)
- Discovery operations broken by refactoring
- Response formats changed unexpectedly
