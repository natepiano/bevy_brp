# BRP Test Suite Runner

<InstallWarning>
## IMPORTANT NOTE ##
If you have recently made changes and haven't intalled it, then you need to install it according to the instructions in ./~claude/commands/build_and_install.md

You can ignore this if no changes have been made.
</InstallWarning>


## Configuration
**PARALLEL_TESTS**: 12 # Number of tests to run concurrently

## Overview

This command runs BRP tests in two modes:
- **Without arguments**: Runs tests PARALLEL_TESTS at a time with continuous execution, stops immediately on any failure
- **With argument**: Runs a single test by name

## Usage Examples
```
/test                    # Run all tests PARALLEL_TESTS at a time, stop on first failure
/test extras             # Run only the extras test
/test data_operations    # Run only the data_operations test
```

## Test Configuration

**Configuration Source**: `.claude/commands/test_config.json`

This file contains an array of test configurations with the following structure:
- `test_name`: Identifier for the test
- `test_file`: Test file name in the tests/ directory
- `app_name`: App/example to launch (or "N/A" or "various")
- `app_type`: Type of app - "example" or "app" (null for "various" or "N/A")
- `test_objective`: What the test validates

**Dynamic Port Allocation**:
- BASE_PORT = 20100
- Ports are dynamically allocated from pools based on app requirements
- Each app instance gets a sequential port starting from BASE_PORT

**IMPORTANT**: Count the number of test objects in test_config.json to determine the total number of tests. Do NOT assume it matches PARALLEL_TESTS.

**CRITICAL COUNTING INSTRUCTION**: You MUST use the following command to count tests accurately:
```bash
jq '. | length' .claude/commands/test_config.json
```
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

1. **Load Configuration**: Read `test_config.json` from `.claude/commands/test_config.json`
2. **Find Test**: Search for test configuration where `test_name` matches `$ARGUMENTS`
3. **Validate**: If test not found, report error and list available test names
4. **Execute Test**: If found, run the single test using appropriate strategy

### Single Test Execution

**For tests where app_name is a specific app (not "various" or "N/A"):**
1. **Launch App**: Use appropriate launch tool based on app_type ("example" or "app")
2. **Assign Port**: Allocate a port from BASE_PORT (20100) upward
3. **Verify Launch**: Use `brp_status` to confirm BRP connectivity on the port
4. **Execute Test**: Use DedicatedAppPrompt template with assigned port
6. **Cleanup**: Shutdown app using `mcp__brp__brp_shutdown`

**For self-managed tests (app_name is "various" or "N/A"):**
1. **Execute Test**: Use SelfManagedPrompt template directly

### Error Handling

If no test configuration matches `$ARGUMENTS`:
```
# Error: Test Not Found

The test "$ARGUMENTS" was not found in .claude/commands/test_config.json.

Usage: /test <test_name>
Example: /test extras
```

## Continuous Parallel Test Mode (when no $ARGUMENTS)

### App Launch Phase

**Before running tests:**

1. **Analyze app requirements** using this command:
   ```bash
   jq '[.[] | select(.app_name == "extras_plugin" or .app_name == "test_app")] | group_by(.app_name) | map({app_name: .[0].app_name, app_type: .[0].app_type, count: length})' .claude/commands/test_config.json
   ```
   This will show you exactly how many instances of each app type you need.

2. **Launch apps using instance_count** based on the counts from step 1:
   - For each app in the jq output, launch with the appropriate tool:
     - If app_type is "example": use `mcp__brp__brp_launch_bevy_example`
     - If app_type is "app": use `mcp__brp__brp_launch_bevy_app`
   - Start at BASE_PORT=20100 and increment by the count for each app group

3. **Track port assignments**:
   - Keep track of which ports belong to which app for later assignment to tests

4. **Verify all apps**: Use `brp_status` on each port in PARALLEL (single message, multiple tool uses)


5. **Track launched apps** for cleanup

**If any app launch fails, STOP immediately and report failure.**

### Test Execution Phase

**Execute tests PARALLEL_TESTS at a time with continuous execution:**

**CRITICAL PARALLEL EXECUTION REQUIREMENT:**
You MUST execute tests in parallel by creating a SINGLE message with multiple Task tool invocations.
DO NOT execute tests sequentially (one Task, wait for result, then next Task).
CORRECT: One message containing 12 Task tool uses for 12 tests
INCORRECT: 12 separate messages each with one Task tool use

1. **Load Configuration**: Read `test_config.json`
2. **Initialize Port Pools**: Based on launched apps
3. **Categorize Tests**:
   - **Dedicated app tests**: Need port assignment from pool
   - **Self-managed tests**: Handle their own apps ("various" or "N/A")
4. **Continuous Execution Loop**:
   - Build batches of tests to run in parallel:
     - For each test needing an app: assign port from appropriate pool
     - **Set Window Title**: Use `mcp__brp__brp_extras_set_window_title` with format "{test_name} test - {app_name} - port {port}"
     - For self-managed tests: prepare with SelfManagedPrompt
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
1. **For each launched app group:**
   - Shutdown all instances at once:
   ```python
   # For each port in use
   mcp__brp__brp_shutdown(app_name=app_name, port=port)
   ```
2. **Verify all shutdowns completed**
3. **Clear port pools and tracking data**

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
- **Total Tests**: [Count from test_config.json]
- **Executed**: X
- **Passed**: X
- **Failed**: 0 (execution stops on first failure)
- **Skipped**: Y
- **Critical Issues**: 0 (execution stops on critical issues)
- **Total Execution Time**: ~X minutes (continuous parallel)
- **Execution Strategy**: PARALLEL_TESTS tests at a time with continuous execution

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
