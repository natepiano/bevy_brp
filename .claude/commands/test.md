# BRP Test Suite Runner

## Configuration
**PARALLEL_TESTS**: 7 # Number of tests to run concurrently

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
- `port`: Port number for the test (or "N/A")
- `app_name`: App/example to launch (or "N/A" or "various")
- `log_file_prefix`: Expected log file prefix for verification (or "N/A")
- `launch_instruction`: How to launch the app (used by main runner)
- `shutdown_instruction`: How to shutdown the app (used by main runner)
- `test_objective`: What the test validates

**IMPORTANT**: Count the number of test objects in test_config.json to determine the total number of tests. Do NOT assume it matches PARALLEL_TESTS.

**CRITICAL COUNTING INSTRUCTION**: You MUST use the following command to count tests accurately:
```bash
jq '. | length' .claude/commands/test_config.json
```
Use this exact count in your final summary. Do NOT manually count or assume any number.

## App Management Strategy

Tests are categorized by app requirements:

### App-Managed Tests (Main Runner Handles)
Tests where `app_name` is a specific app (e.g., "extras_plugin", "test_extras_plugin_app"):
- Each test needs its own app instance on its specified port
- Main runner launches the app before running the test
- Main runner shuts down the app after test completion

**Main runner handles:**
1. App launch on test-specific port
2. BRP connectivity verification (using brp_status)
3. Window title setting
4. App lifecycle management
5. Central cleanup

### Self-Managed Tests
- **various**: Tests handle their own app launching (path, shutdown tests)
- **N/A**: No app required (list test)

## Sub-agent Prompt Templates

### Template for Dedicated App Tests

<DedicatedAppPrompt>

You are executing BRP test: [TEST_NAME]
Configuration: Port [PORT], App [APP_NAME]

**Your Task:**
A [APP_NAME] app is running on port [PORT] with BRP enabled and window title already set.

Execute test procedures from file: [TEST_FILE]

**CRITICAL PORT REQUIREMENT:**
- **ALL BRP operations MUST use port [PORT]**
- **DO NOT launch or shutdown the app** - it's managed externally
- **DO NOT set window titles** - already handled
- **Port parameter is MANDATORY** for all BRP tool calls

**Test Context:**
- Test File: [TEST_FILE]
- Port: [PORT] (MANDATORY for all BRP operations)
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
- Port: [PORT]
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
Configuration: Port [PORT], App [APP_NAME]

**Your Task:**
1. [LAUNCH_INSTRUCTION]
2. Execute test procedures from file: [TEST_FILE]
3. [SHUTDOWN_INSTRUCTION]

**Test Context:**
- Test File: [TEST_FILE]
- Port: [PORT]
- App: [APP_NAME]
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
1. **Launch App**: Use launch_instruction to start the app on the test's specified port
2. **Verify Launch**: Use `brp_status` to confirm BRP connectivity on the port
3. **Set Window Title**: Set title to "[TEST_NAME] test - port [PORT]"
4. **Execute Test**: Use DedicatedAppPrompt template
5. **Cleanup**: Shutdown app using shutdown_instruction

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

**Before running each batch of tests:**

1. **Read test_config.json** to identify which tests are in the current batch
2. **For each test in the batch:**
   - If `app_name` is "various" or "N/A": Skip (test manages its own apps)
   - Otherwise: Note the app_name and port for launching
3. **Launch required apps for the batch:**
   - For each unique app/port combination needed:
     - Use appropriate launch tool (check if it's an example or app)
     - Verify BRP connectivity using `brp_status` on the specified port
     - Set window title to "[TEST_NAME] test - port [PORT]" (using the test name that needs this app)
4. **Track launched apps** for cleanup after batch completion

**If any app launch fails, STOP immediately and report failure.**

### Test Execution Phase

**Execute tests PARALLEL_TESTS at a time with continuous execution:**

1. **Load Configuration**: Read `test_config.json`
2. **Categorize Tests**:
   - **Dedicated app tests**: Use DedicatedAppPrompt (no app management)
   - **Self-managed tests**: Use SelfManagedPrompt (handle own apps)
3. **Continuous Execution Loop**:
   - Launch PARALLEL_TESTS tests from queue using appropriate templates
   - Monitor for failures and stop immediately on first failure
   - Continue until all tests complete or failure detected

### Cleanup Phase

**After each batch of tests completes (success or failure):**
1. **For each app launched in the batch:**
   - Use `mcp__brp__brp_shutdown` with the app_name and port
   - Verify shutdown completed
2. **Clear the tracked apps list** for the next batch

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