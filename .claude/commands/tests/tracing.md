# Tracing and Debug Modes Comprehensive Test

## Objective
Validate the complete tracing and debug mode functionality including:
1. "Do no harm" behavior (no log files created by default)
2. MCP tracing level system works correctly
3. Extras debug mode works independently
4. Integration between MCP tracing and extras debug
5. Verify clippy fixes didn't break functionality

## Test Steps

### Pre-Test Setup
1. **Clean State**: Verify no trace log file exists
   - Use `mcp__brp__brp_get_trace_log_path` to get log file location
   - Verify `exists: false` (file should not exist on fresh start)
   - If `exists: true`, this indicates a violation of "do no harm" - FAIL the test
2. **Launch App**: Start fresh MCP server and launch extras_plugin on assigned port

### Part A: "Do No Harm" Validation (Critical Test)

#### A1. Startup Behavior Test
- **Execute**: Basic operations after server startup
  - `mcp__brp__brp_list_brp_apps`
  - `mcp__brp__brp_list_bevy_examples`
  - `mcp__brp__bevy_list` (basic BRP call)
- **Verify**: Use `mcp__brp__brp_get_trace_log_path` to confirm `exists: false`
- **Expected Result**: No log file created (WARN default level prevents INFO logging)
- **Failure Criteria**: If log file exists, STOP - "do no harm" violated

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
- **Count**: Should see ~21+ DEBUG entries for comprehensive operations

#### B4. TRACE Level Behavior
- **Execute**: `mcp__brp__brp_set_tracing_level` with `level: "trace"`
- **Execute**: Operations that trigger TRACE logs
  - `mcp__brp__brp_extras_discover_format` with type array
  - Complex BRP operations that trigger format discovery
- **Verify**: Maximum verbosity with all trace messages
- **Count**: Should see ~27+ TRACE entries for format discovery details

#### B5. ERROR Level Behavior
- **Execute**: `mcp__brp__brp_set_tracing_level` with `level: "error"`
- **Execute**: Normal operations
- **Verify**: Minimal logging (level changes + actual errors only)
- **Expected Result**: Very quiet output for normal operations

#### B6. WARN Level Behavior
- **Execute**: `mcp__brp__brp_set_tracing_level` with `level: "warn"`
- **Execute**: Operations that might generate warnings
- **Verify**: Only ERROR and WARN messages appear
- **Expected Result**: Level change message + any warnings/errors

### Part C: Extras Debug Mode Tests (Independent)

#### C1. Enable Extras Debug Mode
- **Execute**: `mcp__brp__brp_extras_set_debug_mode` with `enabled: true`
- **Verify**: Success response with debug_enabled: true

#### C2. Extras Debug Output Test
- **Execute**: `mcp__brp__brp_extras_discover_format` with types array
- **Verify**: Response contains `brp_extras_debug_info` field
- **Check**: Debug info includes discovery details, type analysis
- **Verify**: MCP tracing level doesn't affect this response structure

#### C3. Disable Extras Debug Mode
- **Execute**: `mcp__brp__brp_extras_set_debug_mode` with `enabled: false`
- **Execute**: Same discover_format operation
- **Verify**: NO `brp_extras_debug_info` field in response

#### C4. Independence Test
- **Setup**: MCP tracing at DEBUG level, extras debug OFF
- **Execute**: BRP operations
- **Verify**: MCP file logging works, but NO extras debug in responses
- **Expected Result**: Two systems operate independently

### Part D: Integration Tests

#### D1. Both Systems Active
- **Setup**: MCP tracing at DEBUG + extras debug enabled
- **Execute**: `mcp__brp__brp_extras_discover_format`
- **Verify**:
  - MCP trace log contains DEBUG/TRACE entries
  - Response contains `brp_extras_debug_info`
  - Both systems provide complementary information

#### D2. Backwards Compatibility Test
- **Execute**: `mcp__brp__brp_set_tracing_level` with `level: "debug"`
- **Verify**: Extras debug mode automatically enabled (backwards compatibility)
- **Execute**: Operations and check both file logging and response debug info

#### D3. Mixed Level Test
- **Setup**: MCP tracing at TRACE, extras debug OFF
- **Execute**: Complex discovery operations
- **Verify**: Comprehensive MCP logging but clean response format

### Part E: New Tool Validation

#### E1. brp_get_trace_log_path Tool Test
- **Execute**: `mcp__brp__brp_get_trace_log_path` (no parameters)
- **Verify**: Returns JSON with:
  - `log_path`: Valid file path to trace log
  - `exists`: Boolean matching actual file existence
  - `file_size_bytes`: Accurate file size when exists
- **Expected Result**: Tool provides reliable trace log information

#### E2. Dynamic Level Changes
- **Execute**: Switch between all levels multiple times
  - error → warn → info → debug → trace → info
- **Use**: `mcp__brp__brp_get_trace_log_path` to monitor file size changes
- **Verify**: Each change logged and takes immediate effect
- **Check**: No interference between level changes

#### E3. Persistence Test
- **Execute**: Operations at each level to verify filtering
- **Use**: `mcp__brp__brp_get_trace_log_path` to track file growth
- **Verify**: Previous log entries preserved when changing levels
- **Check**: No retroactive filtering of existing entries

## Expected Results Summary

### Critical Success Criteria
- ✅ **"Do No Harm"**: No log file created on startup or normal operations (WARN default)
- ✅ **MCP Tracing**: All levels filter correctly with immediate effect
- ✅ **Extras Debug**: Independent operation, unchanged functionality
- ✅ **Integration**: Backwards compatibility and proper system interaction
- ✅ **Regression**: No functionality lost from clippy fixes
- ✅ **New Tool**: `brp_get_trace_log_path` provides accurate log file information

### Log File Content Expectations
- **Default (WARN)**: 0 entries until user activates tracing
- **ERROR**: Level change messages + actual errors only
- **WARN**: Level changes + warnings + errors
- **INFO**: All above + startup messages + info logs (~4+ entries)
- **DEBUG**: All above + parameter logging (~21+ additional entries)
- **TRACE**: All above + format discovery details (~27+ additional entries)

### Response Format Expectations
- **Extras Debug OFF**: Clean responses without debug fields
- **Extras Debug ON**: Responses include `brp_extras_debug_info`
- **Unchanged**: All existing response formats preserved post-refactoring

## Failure Criteria
**STOP if**:
- Log file created without explicit user request
- Tracing levels don't filter correctly
- Extras debug mode doesn't work independently
- Discovery operations broken by refactoring
- Response formats changed unexpectedly
- Integration between systems fails
