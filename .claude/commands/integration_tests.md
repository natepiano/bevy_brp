# BRP Test Suite Runner

## Configuration
PARALLEL_TESTS = 12  # Number of tests to run concurrently
TEST_CONFIG_FILE = .claude/config/test_config.json  # Test configuration file location

## Overview

This command runs BRP tests in two modes:
- **Without arguments**: Runs tests ${PARALLEL_TESTS} at a time with continuous execution, stops immediately on any failure
- **With argument**: Runs a single test by name

## Usage Examples
```
/test                    # Run all tests ${PARALLEL_TESTS} at a time, stop on first failure
/test extras             # Run only the extras test
/test data_operations    # Run only the data_operations test
```

## Test Configuration

**Configuration Source**: ${TEST_CONFIG_FILE} (see above)

This file contains an array of test configurations with the following structure:
- `test_name`: Identifier for the test
- `test_file`: Test file path (relative to project root)
- `app_name`: App/example to launch (or "N/A" or "various")
- `app_type`: Type of app - "example" or "app" (null for "various" or "N/A")
- `test_objective`: What the test validates

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
Tests where `app_name` is a specific app (e.g., "extras_plugin", "test_app"):
- Each test needs an app instance on a dynamically assigned port
- Main runner launches apps using instance_count for efficiency
- Main runner manages port pools and assignments

**Main runner handles:**
1. App launch on test-specific port
2. BRP connectivity verification (using brp_status)
3. App lifecycle management
4. Central cleanup

**Note**: Window titles are set by main runner after status verification

### Self-Managed Tests
- **various**: Tests handle their own app launching (path, shutdown tests)
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
   - If fails: Wait using `.claude/scripts/integration_test_launch_retry.sh [attempt_number]`
   - Attempt 2-5: Retry with increasing delays (0.5s, 1s, 2s, 4s)
   - This handles Cargo lock contention during concurrent launches
2. **Validation**: Confirm status is "running_with_brp"
3. **Error Handling**: If verification fails after all retries, stop and report
4. **Window Title**: Set title using `brp_extras_set_window_title` with format "{test_name} test - {app_name} - port {port}"
</VerifyBrpConnectivity>

<CleanupApps>
1. **For each launched app**:
   - Shutdown using `mcp__brp__brp_shutdown(app_name=app_name, port=port)`
2. **Verify shutdown completion**
3. **Clear port pools and tracking data**
</CleanupApps>

## Sub-agent Prompt Templates

### Template for Dedicated App Tests

<DedicatedAppPrompt>

You are executing BRP test: [TEST_NAME]
Configuration: Port [ASSIGNED_PORT], App [APP_NAME]

**Your Task:**
A [APP_NAME] app is running on port [ASSIGNED_PORT] with BRP enabled.
Execute test procedures from file: [TEST_FILE]

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
1. Manage your own app launches as needed
2. Execute test procedures from file: [TEST_FILE]
3. Clean up any apps you launched

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

## Execution Mode Selection

**First, check if `$ARGUMENTS` is provided:**
- If `$ARGUMENTS` exists and is not empty: Execute **Single Test Mode**
- If `$ARGUMENTS` is empty or not provided: Execute **Continuous Parallel Test Mode**

## Single Test Mode (when $ARGUMENTS provided)

### Execution Instructions

1. **Load Configuration**: Read test configuration from ${TEST_CONFIG_FILE}
2. **Find Test**: Search for test configuration where `test_name` matches `$ARGUMENTS`
3. **Validate**: If test not found, report error and list available test names
4. **Execute Test**: If found, run the single test using appropriate strategy

### Single Test Execution

**For tests where app_name is a specific app (not "various" or "N/A"):**
1. **Clean up stale processes** from previous test runs:
   ```bash
   pkill -9 extras_plugin || true
   pkill -9 no_extras_plugin || true
   pkill -9 test_app || true
   ```
2. Execute <AllocatePortFromPool/> for single port
3. Execute <LaunchDedicatedApp/> with instance_count=1
4. Execute <VerifyBrpConnectivity/> for assigned port
5. **Execute Test**: Use DedicatedAppPrompt template with assigned port
6. Execute <CleanupApps/> for single app

**For self-managed tests (app_name is "various" or "N/A"):**
1. **Execute Test**: Use SelfManagedPrompt template directly

### Error Handling

If no test configuration matches `$ARGUMENTS`:
```
# Error: Test Not Found

The test "$ARGUMENTS" was not found in ${TEST_CONFIG_FILE}.

Usage: /test <test_name>
Example: /test extras
```

## Continuous Parallel Test Mode (when no $ARGUMENTS)

### App Launch Phase

**Before running tests:**

1. **Clean up stale processes** from previous test runs:
   ```bash
   pkill -9 extras_plugin || true
   pkill -9 no_extras_plugin || true
   pkill -9 test_app || true
   ```
   Note: The `|| true` ensures the command succeeds even if no processes are found

2. **Analyze app requirements** using this command:
   ```bash
   jq '[.[] | select(.app_name == "extras_plugin" or .app_name == "test_app")] | group_by(.app_name) | map({app_name: .[0].app_name, app_type: .[0].app_type, count: length})' ${TEST_CONFIG_FILE}
   ```
   Note: Replace ${TEST_CONFIG_FILE} with the actual path from the configuration section.
   This will show you exactly how many instances of each app type you need.

3. **Launch apps using instance_count** based on the counts from step 2:
   - Execute <LaunchDedicatedApp/> with appropriate instance_count for each app type
   - Start at BASE_PORT=20100 and increment by the count for each app group

4. **Track port assignments**:
   - Execute <AllocatePortFromPool/> to manage port pools for test assignment

5. **Verify all apps**: Execute <VerifyBrpConnectivity/> on each port in PARALLEL (single message, multiple tool uses)

6. **Track launched apps** for cleanup

**If any app launch fails, STOP immediately and report failure.**

### Test Execution Phase

**Execute tests PARALLEL_TESTS at a time with continuous execution:**

**CRITICAL PARALLEL EXECUTION REQUIREMENT:**
You MUST execute tests in parallel by creating a SINGLE message with multiple Task tool invocations.
DO NOT execute tests sequentially (one Task, wait for result, then next Task).
CORRECT: One message containing 12 Task tool uses for 12 tests
INCORRECT: 12 separate messages each with one Task tool use

1. **Load Configuration**: Read ${TEST_CONFIG_FILE}

2. **Extract Test List**: Execute this EXACT command:
   ```bash
   jq -c '.[] | {test_name, test_file, app_name, app_type, test_objective}' ${TEST_CONFIG_FILE}
   ```
   This produces one JSON object per line, in config order, with all fields needed.

3. **Process Each Test in Order**: For each line of output from step 2:
   - Extract test_name field: this is the test identifier
   - Extract app_name field: this determines port assignment
   - Extract app_type field: this determines launch tool
   - Extract test_file field: this is the test specification path
   - Extract test_objective field: this describes what the test validates

4. **Assign Ports Using This Algorithm**:
   - Initialize: extras_plugin_next_port=20100, test_app_next_port=20108
   - For each test in order from step 2:
     - If app_name=="extras_plugin": assign port=extras_plugin_next_port, then extras_plugin_next_port++
     - If app_name=="test_app": assign port=test_app_next_port, then test_app_next_port++
     - If app_name=="various" or "N/A": assign port=null (self-managed, no port needed)
   - Store result: create mapping of test_name → {port, app_name, app_type, test_file, test_objective}

5. **Set Window Titles**: For each test where port is not null, execute:
   ```
   mcp__brp__brp_extras_set_window_title(
     title="{test_name} test - {app_name} - port {port}",
     port={port}
   )
   ```
   Where {test_name}, {app_name}, {port} come from the mapping in step 4.
   Use the EXACT test_name from the config - do not modify or substitute it.

6. **Create Task Prompts**: For each test in config order from step 2:
   - If test has a port (not null): use DedicatedAppPrompt template
     - Set [TEST_NAME] to test_name from config
     - Set [ASSIGNED_PORT] to port from step 4 mapping
     - Set [APP_NAME] to app_name from config
     - Set [TEST_FILE] to test_file from config
     - Set [TEST_OBJECTIVE] to test_objective from config
   - If test has no port (null): use SelfManagedPrompt template
     - Set [TEST_NAME] to test_name from config
     - Set [APP_NAME] to app_name from config
     - Set [TEST_FILE] to test_file from config
     - Set [TEST_OBJECTIVE] to test_objective from config

7. **Execute All Tests in Parallel**:
   - Build batches of tests to run in parallel using prompts from step 6
   - **CRITICAL PARALLEL EXECUTION**:
     - Create a SINGLE message with multiple Task tool invocations
     - Each Task call represents one test to run in parallel
     - Example structure for parallel execution:
       ```
       <single_message>
       Task(description="Execute test1", prompt=DedicatedAppPrompt_for_test1)
       Task(description="Execute test2", prompt=DedicatedAppPrompt_for_test2)
       Task(description="Execute test3", prompt=SelfManagedPrompt_for_test3)
       ... up to PARALLEL_TESTS tasks in ONE message
       </single_message>
       ```
   - Monitor completed results for failures
   - Return ports to pool as tests complete
   - Stop immediately on first failure detected
   - Continue batching until all tests complete or failure detected

### Cleanup Phase

**After all tests complete (success or failure):**
Execute <CleanupApps/> for all launched apps

### Error Detection and Immediate Stopping

**CRITICAL**: Monitor each completed test result for failure indicators:
- Check for `### ❌ FAILED` sections with content
- Check for `**Critical Issues**: Yes` in summary
- Check for any `CRITICAL FAILURE` mentions in results

**On Error Detection**:
1. **STOP immediately** - do not start any new tests
2. **Collect results** from any currently running tests
3. **Cleanup apps** immediately
4. **Report failure immediately** with details from failed test

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
- **Total Execution Time**: ~X minutes (continuous parallel)
- **Execution Strategy**: ${PARALLEL_TESTS} tests at a time with continuous execution

## Test Results Summary
[List each test by name with its result count, avoiding duplication]

## ⚠️ SKIPPED TESTS
[List of skipped tests with reasons]
```

**Failure Path**: When error detected:
```
# BRP Test Suite - FAILED

## ❌ CRITICAL FAILURE DETECTED

**Failed Test**: [test_name]
**Failure Type**: [Critical Issues/Failed Tests/etc.]

### Failure Details
[Include full failure details from the failed test]

### Tests Completed Before Failure
- **Completed**: X tests
- **Results**: [Brief summary of completed tests]

### Tests Not Executed
- **Remaining**: Y tests
- **Reason**: Execution stopped due to failure

### Cleanup Status
- **extras_plugin**: [Shutdown/Still Running]
- **test_extras_plugin_app**: [Shutdown/Still Running]

**Recommendation**: Fix the failure in [test_name] before running remaining tests.
```
