# Tracing and Debug Modes Comprehensive Test

## Objective
Validate the complete tracing and debug mode functionality including:

## Test Steps

### Pre-Test Setup
1. **Baseline Measurement**: Record initial trace log state
   - Use `mcp__brp__brp_get_trace_log_path` to get log file info
   - Record initial state: either `exists: false` or if `exists: true`, record the `file_size_bytes`
   - This establishes baseline for detecting NEW logging activity

### Part A: "Do No Harm" Validation (Critical Test)

#### A1. Default Logging Behavior Test
- **Execute**: Basic operations after server startup
  - `mcp__brp__brp_list_brp_apps`
  - `mcp__brp__brp_list_bevy_examples`
  - `mcp__brp__bevy_list` (basic BRP call)
- **Verify**: Use `mcp__brp__brp_get_trace_log_path` to check for changes
  - If file didn't exist: confirm it still doesn't exist
  - If file existed: confirm `file_size_bytes` hasn't increased
- **Expected Result**: No new logging activity (WARN default level prevents routine logging)
- **Failure Criteria**: File created or size increased, indicating unwanted logging

#### A2. Extended Operations Test
- **Execute**: More BRP operations to ensure no logging occurs
  - `mcp__brp__bevy_get_resource` for any resource
  - `mcp__brp__bevy_query` with basic filter
- **Verify**: Use `mcp__brp__brp_get_trace_log_path` to check for changes
  - Compare against baseline: no file creation or size increase
- **Expected Result**: Normal operations don't trigger new log entries

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
- ✅ **"Do No Harm"**: No NEW log entries created on startup or normal operations (WARN default)
- ✅ **MCP Tracing**: All levels filter correctly with immediate effect
- ✅ **New Tool**: `brp_get_trace_log_path` provides accurate log file information

### Log File Content Expectations
- **Default (WARN)**: No new entries until user activates tracing (file may exist from prior runs)
- **INFO**: Level change message + info logs from operations
- **DEBUG**: All above + detailed parameter logging

### Response Format Expectations
- **MCP Tracing**: Does not affect response structure, only creates log files

## Failure Criteria
**STOP if**:
- New log entries created without explicit user request (file size increases from baseline)
- Tracing levels don't filter correctly (INFO/DEBUG)
- Discovery operations broken by refactoring
- Response formats changed unexpectedly
