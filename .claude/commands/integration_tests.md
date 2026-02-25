# BRP Test Suite Runner

## Configuration
PARALLEL_TESTS = 8  # Number of tests to run concurrently
TEST_CONFIG_FILE = .claude/config/integration_tests.json  # Test configuration file location
AGENT_MODEL = sonnet  # Model for test runner agents (sonnet is concise and fast for execution tasks)

## Overview

This command runs BRP tests in three modes:
- **Without arguments**: Runs all tests ${PARALLEL_TESTS} at a time with continuous execution, stops immediately on any failure
- **With single test name**: Runs one test by name
- **With comma-delimited list**: Runs specified tests ${PARALLEL_TESTS} at a time, stops on any failure

## Usage Examples
```
/test                           # Run all tests ${PARALLEL_TESTS} at a time, stop on first failure
/test extras                    # Run only the extras test
/test extras,mouse_input        # Run extras and mouse_input tests
/test data_operations,events    # Run data_operations and events tests
```

## Test Configuration

**Configuration Source**: ${TEST_CONFIG_FILE} (see above)

This file contains an array of test configurations. Each entry has one of two formats:

**Single-app format** (most tests):
- `test_name`: Identifier for the test
- `test_file`: Test file path (relative to project root)
- `app_name`: App/example to launch (or "N/A" or "various")
- `app_type`: Type of app - "example" or "app" (null for "various" or "N/A")

**Multi-app format** (tests needing multiple pre-launched instances):
- `test_name`: Identifier for the test
- `test_file`: Test file path (relative to project root)
- `apps`: Array of app instance objects, each with:
  - `app_name`: Name of the app/example to launch
  - `app_type`: Type of app - "example" or "app"
  - `label`: Unique label for referencing this instance in the test file
  - `fixed_port` (optional): If set, this instance uses this exact port instead of a dynamically allocated one
  - `consumed_by_test` (optional): If true, the test itself shuts down this app as part of its test steps. The runner must NOT shut it down during cleanup, but must verify it is no longer running (error if still alive).

**Note**: Test objectives are extracted from each test file's `## Objective` section, not stored in this config.

**Dynamic Port Allocation**:
- BASE_PORT = 20100
- Ports are dynamically allocated from pools based on app requirements
- Each app instance gets a sequential port starting from BASE_PORT

**IMPORTANT**: Count the number of test objects in test_config.json to determine the total number of tests. Do NOT assume it matches ${PARALLEL_TESTS}.

**CRITICAL COUNTING INSTRUCTION**: You MUST use the following command to count tests accurately:
```bash
jq '. | length' ${TEST_CONFIG_FILE}
```
Note: Replace ${TEST_CONFIG_FILE} with the actual path from the configuration section above.
Use this exact count in your final summary. Do NOT manually count or assume any number.

## App Management Strategy

Tests are categorized by app requirements:

### App-Managed Tests (Main Runner Handles)
Tests where `app_name` is a specific app (e.g., "extras_plugin", "test_app", "event_test"):
- Each test needs an app instance on a dynamically assigned port
- Main runner launches apps using instance_count for efficiency
- Main runner manages port pools and assignments

**Main runner handles:**
1. App launch on test-specific port
2. BRP connectivity verification (using brp_status)
3. App lifecycle management
4. Central cleanup

**Note**: Window titles are set by main runner after status verification

### Multi-App Tests (Main Runner Handles)
Tests with an `apps` array instead of `app_name`:
- Each entry in the array defines an app instance with a unique label
- Dynamic-port instances get sequential ports from the pool
- Fixed-port instances (`fixed_port` field) use their specified port
- Main runner launches all instances, verifies connectivity, and sets window titles
- Sub-agent receives a port map keyed by label and must NOT launch or shutdown apps

### Self-Managed Tests
- **various**: Tests handle their own app launching (path tests)
- **N/A**: No app required (list test)

## Reusable Operation Sections

<LaunchDedicatedApp>
1. **Determine Launch Tool**: Based on app_type:
   - If app_type is "example": use `mcp__brp__brp_launch_bevy_example`
   - If app_type is "app": use `mcp__brp__brp_launch_bevy_app`
2. **Launch**: Execute with target_name=[APP_NAME], port=[ASSIGNED_PORT], instance_count=[COUNT]
3. **Track**: Record launched app for cleanup
</LaunchDedicatedApp>

<AllocatePortFromPool>
1. **Calculate Port**: Start from BASE_PORT (20100) + offset for app type
2. **Assign**: Take next available port from the appropriate pool
3. **Reserve**: Mark port as in-use for tracking
</AllocatePortFromPool>

<VerifyBrpConnectivity>
1. **Status Check with Retry**: For each app, retry up to 5 times with exponential backoff:
   - Attempt 1: Check `brp_status(app_name=[APP_NAME], port=[ASSIGNED_PORT])` immediately
   - If fails: Wait using `.claude/scripts/integration_tests/launch_retry.sh [attempt_number]`
   - Attempt 2-5: Retry with increasing delays (0.5s, 1s, 2s, 4s)
   - This handles Cargo lock contention during concurrent launches
2. **Validation**: Confirm status is "running_with_brp"
3. **Error Handling**: If verification fails after all retries, stop and report
4. **Window Title**: Set title using `brp_extras_set_window_title` with format "{test_name} test - {app_name} - port {port}"
</VerifyBrpConnectivity>

<PrebuildWorkspace>
**Purpose**: Pre-compile all workspace targets to eliminate Cargo lock contention when multiple tests launch apps concurrently.

Run the following command and wait for it to complete:
```bash
bash .claude/scripts/integration_tests/prebuild_workspace.sh
```

- `--workspace` builds all library and binary targets, `--examples` additionally builds all examples
- Subsequent `cargo run` calls skip compilation and launch immediately
- **CRITICAL**: Must complete before ANY app launches
- Script uses strict error handling and preserves Cargo exit status
- If the build fails, STOP and report the build error

**WASM Prebuild** (conditional):
- Check if "wasm" is in the current test list (either running all tests or explicitly specified)
- If yes, additionally run:
```bash
bash .claude/scripts/integration_tests/prebuild_workspace.sh --include-wasm
```
- This MUST complete before the wasm test starts to avoid Cargo lock contention
- If the WASM build fails, STOP and report the build error
</PrebuildWorkspace>

<CleanupApps>
Apps fall into two categories based on the `consumed_by_test` field in the test config:

**Runner-managed apps** (no `consumed_by_test` or `consumed_by_test: false`):
1. Shut down using `mcp__brp__brp_shutdown(app_name=app_name, port=port)`
2. If shutdown fails, report as error

**Test-consumed apps** (`consumed_by_test: true`):
1. Do NOT attempt shutdown — the test was responsible for shutting these down
2. Verify they are no longer running using `mcp__brp__brp_status(app_name=app_name, port=port)`
3. If still running, report as **error** — the test failed to consume the app

**Clear port pools and tracking data** after all checks complete.
</CleanupApps>

<AllocatePortsForMultiAppTest>
For tests with an `apps` array, allocate ports for each app instance:
1. **Iterate** through the `apps` array entries
2. **Fixed-port apps**: If entry has `fixed_port`, assign that exact port to the label
3. **Dynamic-port apps**: Assign the next sequential port from the pool (starting at current_port)
4. **Return**: A map of `{label → port}` for all instances
5. **Note**: Fixed-port apps do NOT consume from the dynamic pool
</AllocatePortsForMultiAppTest>

<LaunchMultiAppInstances>
For tests with an `apps` array, launch all app instances:
1. **Group** apps by `(app_name, app_type)`, separating fixed-port from dynamic-port entries
2. **Dynamic-port batch launch**: For each group of dynamic-port apps with the same `(app_name, app_type)`:
   - Execute <LaunchDedicatedApp/> with instance_count=count, starting at the first allocated port
   - This launches multiple instances in a single call for efficiency
3. **Fixed-port individual launch**: For each fixed-port app:
   - Execute <LaunchDedicatedApp/> with instance_count=1, port=fixed_port
4. **Track**: Record all launched apps (label, app_name, port) for cleanup
</LaunchMultiAppInstances>

<VerifyMultiAppConnectivity>
For tests with an `apps` array, verify all instances and set window titles:
1. **For each app** in the `apps` array:
   - Execute <VerifyBrpConnectivity/> on the assigned port (from the label→port map)
   - **Skip BRP verification** for apps that don't have BRP (e.g., no_extras_plugin) — instead just verify the process launched using `brp_status` to confirm a PID exists
2. **Set Window Titles**: For each app with BRP, set title using format: `"{test_name} test - {label} - {app_name} - port {port}"`
</VerifyMultiAppConnectivity>

## Sub-agent Prompt Templates

### Template for Dedicated App Tests

<DedicatedAppPrompt>

You are executing BRP test: [TEST_NAME]
Configuration: Port [ASSIGNED_PORT], App [APP_NAME]

**Your Task:**
A [APP_NAME] app is running on port [ASSIGNED_PORT] with BRP enabled.
Read [TEST_FILE] and execute each numbered test step exactly as written.
Use only the exact types, values, and tool parameters specified in the test file.

**CRITICAL PORT REQUIREMENT:**
- **ALL BRP operations MUST use port [ASSIGNED_PORT]**
- **DO NOT launch or shutdown the app** - it's managed externally
- **Port parameter is MANDATORY** for all BRP tool calls


**Test Context:**
- Test File: [TEST_FILE]
- Port: [ASSIGNED_PORT] (MANDATORY for all BRP operations)
- App: [APP_NAME] (already running)
- Objective: [TEST_OBJECTIVE]

**FAILURE HANDLING PROTOCOL:**
- **STOP ON FIRST FAILURE**: When ANY test step fails, IMMEDIATELY stop all testing
- **CAPTURE EVERYTHING**: Include complete tool responses for all failed operations
- **NO CONTINUATION**: Do not attempt further test steps after first failure

**CRITICAL: NO ISSUE IS MINOR - EVERY ISSUE IS A FAILURE**
- Error message quality issues are FAILURES, not minor issues
- Any deviation from expected behavior is a FAILURE
- Do NOT categorize any issue as "minor" - mark it as FAILED

**Required Response Format:**

# Test Results: [TEST_NAME]

## Configuration
- Port: [ASSIGNED_PORT]
- App: [APP_NAME] (externally managed)
- Test Status: [Completed/Failed]


## Test Results
### ✅ PASSED
- [Test description]: [Brief result]

### ❌ FAILED
- [Test description]: [Brief result]
  - **Error**: [exact error message]
  - **Expected**: [what should happen]
  - **Actual**: [what happened]
  - **Impact**: critical
  - **Component/Resource**: [fully qualified type name or N/A if not applicable]
  - **Full Tool Response**: [Complete JSON response from the failed tool call]

### ⚠️ SKIPPED
- [Test description]: [reason for skipping]

## Summary
- **Total Tests**: X
- **Passed**: Y
- **Failed**: Z
- **Critical Issues**: [Yes/No - brief description if yes]

</DedicatedAppPrompt>

### Template for Self-Managed Tests

<SelfManagedPrompt>

You are executing BRP test: [TEST_NAME]
Configuration: App [APP_NAME]

**Your Task:**
1. Launch apps as needed using MCP tools
2. Read [TEST_FILE] and execute each numbered test step exactly as written. Use only the exact types, values, and tool parameters specified in the test file.

**CRITICAL TOOL USAGE - USE MCP TOOLS DIRECTLY:**
- **Launch apps**: `mcp__brp__brp_launch_bevy_example(target_name="app_name", port=PORT, profile="debug")`
- **Check status**: `mcp__brp__brp_status(app_name="app_name", port=PORT)`
- **Shutdown apps**: `mcp__brp__brp_shutdown(app_name="app_name", port=PORT)`
- **DO NOT write bash scripts** - call MCP tools directly or invoke scripts that are already written if this is specified in the test
- **DO NOT simulate tool calls** - execute the actual MCP tools

**Port Allocation for Self-Managed Tests:**
- If using a launch command, use ports starting from 20200 to avoid conflicts with main runner
- Increment port for each additional app instance you launch

**Test Context:**
- Test File: [TEST_FILE]
- App: [APP_NAME] (self-managed)
- Objective: [TEST_OBJECTIVE]

**FAILURE HANDLING PROTOCOL:**
- **STOP ON FIRST FAILURE**: When ANY test step fails, IMMEDIATELY stop all testing
- **CAPTURE EVERYTHING**: Include complete tool responses for all failed operations
- **NO CONTINUATION**: Do not attempt further test steps after first failure

**CRITICAL: NO ISSUE IS MINOR - EVERY ISSUE IS A FAILURE**

**Required Response Format:**
[Same format as DedicatedAppPrompt]

</SelfManagedPrompt>

### Template for Multi-App Tests

<MultiAppPrompt>

You are executing BRP test: [TEST_NAME]

**Your Task:**
Multiple app instances are pre-launched and managed externally.
Read [TEST_FILE] and execute each numbered test step exactly as written.
Use only the exact types, values, and tool parameters specified in the test file.

**App Instance Configuration:**
[For each label in the apps array, list:]
- **[LABEL]**: [APP_NAME] on port [PORT] ([dynamic/fixed])

**CRITICAL REQUIREMENTS:**
- **DO NOT launch or shutdown any apps** - all instances are managed externally
- **Reference apps by label** and use the port assigned to each label
- **Port parameter is MANDATORY** for all BRP tool calls
- Where the test file says `[label port]`, substitute the actual port number from the configuration above

**Test Context:**
- Test File: [TEST_FILE]
- Objective: [TEST_OBJECTIVE]

**FAILURE HANDLING PROTOCOL:**
- **STOP ON FIRST FAILURE**: When ANY test step fails, IMMEDIATELY stop all testing
- **CAPTURE EVERYTHING**: Include complete tool responses for all failed operations
- **NO CONTINUATION**: Do not attempt further test steps after first failure

**CRITICAL: NO ISSUE IS MINOR - EVERY ISSUE IS A FAILURE**
- Error message quality issues are FAILURES, not minor issues
- Any deviation from expected behavior is a FAILURE
- Do NOT categorize any issue as "minor" - mark it as FAILED

**Required Response Format:**

# Test Results: [TEST_NAME]

## Configuration
[For each label: label → app_name on port X]
- Test Status: [Completed/Failed]

## Test Results
### ✅ PASSED
- [Test description]: [Brief result]

### ❌ FAILED
- [Test description]: [Brief result]
  - **Error**: [exact error message]
  - **Expected**: [what should happen]
  - **Actual**: [what happened]
  - **Impact**: critical
  - **Component/Resource**: [fully qualified type name or N/A if not applicable]
  - **Full Tool Response**: [Complete JSON response from the failed tool call]

### ⚠️ SKIPPED
- [Test description]: [reason for skipping]

## Summary
- **Total Tests**: X
- **Passed**: Y
- **Failed**: Z
- **Critical Issues**: [Yes/No - brief description if yes]

</MultiAppPrompt>

## Execution Mode Selection

**Parse `$ARGUMENTS` to determine mode:**

1. **Split on commas**: Parse `$ARGUMENTS` by splitting on commas
   - Use: `echo "$ARGUMENTS" | tr ',' '\n'` to get test names (one per line)
   - Trim whitespace from each name

2. **Select mode based on count**:
   - If `$ARGUMENTS` is empty or not provided: Execute **All Tests Mode**
   - If split produces 1 test name: Execute **Single Test Mode**
   - If split produces 2+ test names: Execute **Multiple Tests Mode**

## Single Test Mode (1 test name in $ARGUMENTS)

### Execution Instructions

1. **Load Configuration**: Read test configuration from ${TEST_CONFIG_FILE}
2. **Find Test**: Search for test configuration where `test_name` matches the test name
3. **Validate**: If test not found, report error and list available test names
4. **Execute Test**: If found, run the single test using appropriate strategy

### Single Test Execution

0. Execute <PrebuildWorkspace/>

**For tests with an `apps` array (multi-app tests):**
1. **Clean up stale processes** from previous test runs:
   ```bash
   bash .claude/scripts/integration_tests/cleanup_stale_test_processes.sh ${TEST_CONFIG_FILE}
   ```
2. Execute <AllocatePortsForMultiAppTest/> to build the label→port map
3. Execute <LaunchMultiAppInstances/> for all app entries
4. Execute <VerifyMultiAppConnectivity/> for all instances
5. **Execute Test**: Use MultiAppPrompt template with the label→port map, model=${AGENT_MODEL}
6. Execute <CleanupApps/> for all app instances launched for this test

**For tests where app_name is a specific app (not "various" or "N/A"):**
1. **Clean up stale processes** from previous test runs:
   ```bash
   bash .claude/scripts/integration_tests/cleanup_stale_test_processes.sh ${TEST_CONFIG_FILE}
   ```
2. Execute <AllocatePortFromPool/> for single port
3. Execute <LaunchDedicatedApp/> with instance_count=1
4. Execute <VerifyBrpConnectivity/> for assigned port
5. **Execute Test**: Use DedicatedAppPrompt template with assigned port, model=${AGENT_MODEL}
6. Execute <CleanupApps/> for single app

**For self-managed tests (app_name is "various" or "N/A"):**
1. **Execute Test**: Use SelfManagedPrompt template directly, model=${AGENT_MODEL}

### Error Handling

If no test configuration matches the test name:
```
# Error: Test Not Found

The test "{test_name}" was not found in ${TEST_CONFIG_FILE}.

Usage: /test [test_name[,test_name...]]
Examples:
  /test extras
  /test extras,mouse_input
```

## Multiple Tests Mode (2+ test names in $ARGUMENTS)

### Execution Instructions

1. **Load Configuration**: Read test configuration from ${TEST_CONFIG_FILE}
2. **Parse Test Names**: Split `$ARGUMENTS` on commas and trim whitespace from each name
3. **Validate All Tests**: For each test name, search for matching test configuration
   - If ANY test not found, report error listing all missing tests and available test names
   - If all found, continue to execution
4. **Filter Test List**: Build test list containing only the specified tests (preserve config order)
5. **Execute Tests**: Use the same batched parallel execution as "All Tests Mode" (see below), but with filtered test list

### Error Handling

If any test configuration is not found:
```
# Error: Tests Not Found

The following tests were not found in ${TEST_CONFIG_FILE}:
- {test_name1}
- {test_name2}

Available tests: {list of all available test names}

Usage: /test [test_name[,test_name...]]
Examples:
  /test extras
  /test extras,mouse_input,data_operations
```

## All Tests Mode (when no $ARGUMENTS)

### Setup Phase

**Before running tests:**

1. Execute <PrebuildWorkspace/>

2. **Clean up stale processes** from previous test runs:
   ```bash
   bash .claude/scripts/integration_tests/cleanup_stale_test_processes.sh ${TEST_CONFIG_FILE}
   ```

3. **Load Configuration**: Read ${TEST_CONFIG_FILE}

4. **Extract Test List**: Execute this EXACT command:
   ```bash
   jq -c '.[] | {test_name, test_file, app_name, app_type, apps}' ${TEST_CONFIG_FILE}
   ```
   This produces one JSON object per line, in config order. Tests with `apps` array will have `app_name: null` and `app_type: null`.

5. **Extract Objectives and Build Test List**:
   - Collect all test_file paths from step 3 into a space-separated list
   - Extract all objectives in one call: `.claude/scripts/integration_tests/extract_test_objectives.sh file1.md file2.md ...`
   - This returns one objective per line, matching the order of input files
   - Combine with test data from step 3 (line-by-line pairing)
   - Store in test list: {test_name, test_file, app_name, app_type, test_objective}

### Batched Execution with Just-In-Time App Launching

**CRITICAL PARALLEL EXECUTION REQUIREMENT:**
You MUST execute tests in parallel by creating a SINGLE message with multiple Task tool invocations.
DO NOT execute tests sequentially (one Task, wait for result, then next Task).

**For each batch of up to PARALLEL_TESTS tests:**

1. **Select Next Batch**: Take next PARALLEL_TESTS tests from the test list

2. **Analyze Batch App Requirements**:
   - For single-app tests: Identify unique app_name values in this batch (excluding "N/A" and "various"), count instances needed per app_name
   - For multi-app tests: Add each entry from the `apps` array to the requirements list
   - Example: If batch has 3 single-app tests using "extras_plugin" and 1 multi-app test with 2 extras_plugin + 1 no_extras_plugin, need extras_plugin×5 and no_extras_plugin×1

3. **Allocate Ports for Batch**:
   - Start at BASE_PORT=20100
   - For single-app tests:
     - For each unique app: assign sequential ports starting from current_port
     - Track: app_name → [port1, port2, ...]
     - Increment current_port by instance count
   - For multi-app tests: Execute <AllocatePortsForMultiAppTest/>
     - Fixed-port apps get their `fixed_port` value (do NOT consume from dynamic pool)
     - Dynamic-port apps get sequential ports from the pool
   - For self-managed tests (app_name is "N/A" or "various"): assign port=null

4. **Launch Apps for This Batch Only**:
   - For single-app tests: Group by app_name, execute <LaunchDedicatedApp/> with instance_count=count
   - For multi-app tests: Execute <LaunchMultiAppInstances/>
   - Track all launched apps for cleanup

5. **Verify App Connectivity**:
   - For single-app tests: Execute <VerifyBrpConnectivity/> on each launched port in PARALLEL
   - For multi-app tests: Execute <VerifyMultiAppConnectivity/>
   - If any verification fails, cleanup and STOP

6. **Set Window Titles**:
   - For single-app tests with assigned port, execute in PARALLEL:
     ```
     mcp__brp__brp_extras_set_window_title(
       title="{test_name} test - {app_name} - port {port}",
       port={port}
     )
     ```
   - Multi-app window titles are set in <VerifyMultiAppConnectivity/>

7. **Create Task Prompts for Batch**:
   - For each test in batch:
     - If test has `apps` array → use MultiAppPrompt with label→port map, [TEST_NAME], [TEST_FILE], [TEST_OBJECTIVE]
     - If test has specific `app_name` (not "various"/"N/A") → use DedicatedAppPrompt with [TEST_NAME], [ASSIGNED_PORT], [APP_NAME], [TEST_FILE], [TEST_OBJECTIVE]
     - If test has `app_name` of "various" or "N/A" → use SelfManagedPrompt with [TEST_NAME], [APP_NAME], [TEST_FILE], [TEST_OBJECTIVE]

8. **Execute Batch Tests in Parallel**:
   - Create SINGLE message with multiple Task invocations (one per test in batch)
   - Example:
     ```
     <single_message>
     Task(description="Execute test1", prompt=DedicatedAppPrompt_for_test1, model=AGENT_MODEL)
     Task(description="Execute test2", prompt=DedicatedAppPrompt_for_test2, model=AGENT_MODEL)
     ... up to PARALLEL_TESTS tasks
     </single_message>
     ```
   - Wait for ALL tasks in batch to complete

9. **Check Batch Results**:
   - Monitor for failure indicators in any test result
   - If ANY test failed: Execute <CleanupApps/> for batch apps and STOP
   - If all passed: Continue to step 10

10. **Cleanup Batch Apps**:
    - Execute <CleanupApps/> for all apps launched in this batch
    - Clear batch tracking data

11. **Continue or Complete**:
    - If more tests remain: Return to step 1 for next batch
    - If all tests complete: Proceed to final summary

### Error Detection and Immediate Stopping

**CRITICAL**: After each batch completes, check ALL test results for failure indicators:
- Check for `### ❌ FAILED` sections with content
- Check for `**Critical Issues**: Yes` in summary
- Check for any `CRITICAL FAILURE` mentions in results

**On Error Detection**:
1. **Cleanup batch apps** - shutdown all apps from current batch
2. **STOP immediately** - do not start any new batches
3. **Report failure** with details from failed test and batch summary

### Results Formats

**Success Path**: After all tests complete successfully:
```
# BRP Test Suite - Consolidated Results

## Overall Statistics
- **Total Tests**: [Count from ${TEST_CONFIG_FILE}]
- **Executed**: X
- **Passed**: X
- **Failed**: 0 (execution stops on first failure)
- **Skipped**: Y
- **Critical Issues**: 0 (execution stops on critical issues)
- **Total Batches**: Z
- **Total Execution Time**: ~X minutes
- **Execution Strategy**: Just-in-time batch execution (${PARALLEL_TESTS} tests per batch)

## Test Results Summary
[List each test by name with its result count, avoiding duplication]

## ⚠️ SKIPPED TESTS
[List of skipped tests with reasons]

## Execution Notes
- Apps launched just-in-time per batch for efficiency
- All batch apps cleaned up successfully after each batch
```

**Failure Path**: When error detected:
```
# BRP Test Suite - FAILED

## ❌ CRITICAL FAILURE DETECTED

**Failed Test**: [test_name]
**Failed Batch**: Batch X (tests Y-Z)
**Failure Type**: [Critical Issues/Failed Tests/etc.]

### Failure Details
[Include full failure details from the failed test]

### Batch Information
- **Batch Apps Launched**: [app_name on port X, ...]
- **Batch Apps Cleaned Up**: Yes/No
- **Tests in Failed Batch**: [test names]
- **Results**: [passed count] passed, [failed count] failed

### Tests Completed Before Failure
- **Completed Batches**: X batches
- **Completed Tests**: Y tests
- **Results**: [Brief summary of completed tests from previous batches]

### Tests Not Executed
- **Remaining Tests**: Z tests
- **Reason**: Execution stopped due to failure in batch X

**Recommendation**: Fix the failure in [test_name] before running remaining tests.
```
