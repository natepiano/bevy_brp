# Format Discovery Comparison Test

## Objective
Validate our local registry + hardcoded knowledge approach against the current extras plugin by running comparison-driven development scenarios. Establish baseline visibility into extras responses without building any local representations yet.

## Prerequisites
- Launch extras_plugin example on port 15702 at the beginning
- Keep the app running throughout all test steps
- Clean shutdown at the end

## Test Steps

### 1. Setup Tracing Infrastructure
- Execute `mcp__brp__brp_set_tracing_level` with level `"trace"`
- Execute `mcp__brp__brp_get_trace_log_path` to get trace log location
- Use `rm` command to manually remove the trace log file at the returned path

### 2. Test Format Discovery with Failure Scenario
- Execute `mcp__brp__bevy_spawn` with Transform using wrong object format (will trigger format discovery):
  ```json
  {
    "bevy_transform::components::transform::Transform": {
      "translation": {"x": 5.0, "y": 10.0, "z": 15.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
      "scale": {"x": 2.0, "y": 2.0, "z": 2.0}
    }
  }
  ```
- This will trigger format discovery and correction during the failure/retry process
- Check trace log for comparison entries showing:
  - Format discovery comparison results during correction process
  - Differences between extras and local (should show all fields missing from Local)
  - Structured logging of comparison data during the discovery that occurs on failure

### 3. Analyze Comparison Results
- Execute `mcp__brp__brp_get_trace_log_path` to get trace log location
- Use `grep "PHASE_1_STATUS:" <trace_log_path> | tail -1` to find Phase 1 status
- Parse the JSON to check:
  - `success`: Should be `true` for Phase 1 (spawn formats match)
  - `spawn_formats_match`: Should be `true` 
  - `has_core_structure`: Should be `true`
- Use `grep "COMPARISON_RESULT:" <trace_log_path> | tail -1` to find detailed comparison
- Parse the JSON to verify:
  - `missing_in_local`: List of fields we haven't implemented yet
  - `missing_in_extras`: Should be empty (extras has everything)
  - `value_mismatches`: Should be empty for spawn formats
  - `spawn_formats_match`: Should be `true` for Transform
- Look for human-friendly summary with `grep "✅ Phase 1 SUCCESS" <trace_log_path>`

### 4. Cleanup
- Execute `mcp__brp__brp_extras_shutdown` with app_name from initial launch
- Verify clean shutdown response
- Confirm app process terminates gracefully

## Expected Results
- ✅ Comparison infrastructure runs in parallel without impacting existing flow
- ✅ `PHASE_1_STATUS` shows `"success": true` for Transform
- ✅ `COMPARISON_RESULT` shows `"spawn_formats_match": true` for Transform
- ✅ Missing fields only in wrapper/metadata (not in spawn format itself)
- ✅ Human-friendly log shows "✅ Phase 1 SUCCESS for bevy_transform::components::transform::Transform"
- ✅ Format discovery and correction continue to work normally
- ✅ No impact on spawn/insert operations success rates
- ✅ Structured logs include:
  - `PHASE_1_STATUS`: Phase-specific success criteria
  - `COMPARISON_RESULT`: Full comparison details with categorized differences
  - Clear JSON format parseable with `jq`
  - Both machine-readable and human-friendly output

## Failure Criteria
STOP if:
- Comparison infrastructure interferes with normal BRP operations
- Trace logs don't show comparison entries
- Spawn/insert operations fail due to comparison code
- Comparison logic crashes or causes errors
- Unable to see structured difference data in trace logs
