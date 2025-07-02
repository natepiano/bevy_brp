# BRP Test Suite Runner

## Configuration
**PARALLEL_TESTS**: 12  # Number of tests to run concurrently

## Overview

This command runs BRP tests in two modes:
- **Without arguments**: Runs tests PARALLEL_TESTS at a time with continuous execution, stops immediately on any failure
- **With argument**: Runs a single test by name

## Usage Examples
```
/test_runner                    # Run all tests PARALLEL_TESTS at a time, stop on first failure
/test_runner debug_mode         # Run only the debug_mode test
/test_runner data_operations    # Run only the data_operations test
```

## Test Configuration

**Configuration Source**: `.claude/commands/test_config.json`

This file contains an array of test configurations with the following structure:
- `test_name`: Identifier for the test
- `test_file`: Test file name in the tests/ directory
- `port`: Port number for the test (or "N/A")
- `app_name`: App/example to launch (or "N/A")
- `launch_instruction`: How to launch the app
- `shutdown_instruction`: How to shutdown the app
- `test_objective`: What the test validates
- `expected_shutdown_method`: Expected shutdown behavior (`clean_shutdown`, `process_kill`, or `N/A`)

**Total Tests**: 12 tests

## Shutdown Validation

All tests that launch apps must validate the shutdown method matches the expected behavior:

### Expected Shutdown Methods
- **clean_shutdown**: Apps with `BrpExtrasPlugin` (extras_plugin, test_extras_plugin_app)
  - Message: "Successfully initiated graceful shutdown for '...' via bevy_brp_extras on port ..."
  - Method field: `"clean_shutdown"`
- **process_kill**: Apps without `BrpExtrasPlugin` (no_extras_plugin)
  - Message: "Terminated process '...' (PID: ...) using kill. Consider adding bevy_brp_extras for clean shutdown."
  - Method field: `"process_kill"`
- **N/A**: Tests with no app launch (discovery)

### Validation Rules
- Parse shutdown response `data.method` field
- Compare against `expected_shutdown_method` from test config
- Report mismatch as FAILED test with detailed explanation
- Include both expected and actual methods in test results

## Sub-agent Prompt Template

<SubAgentPrompt>

You are executing BRP test: [TEST_NAME]
Configuration: Port [PORT], App [APP_NAME]

**Your Task:**
1. [LAUNCH_INSTRUCTION]
2. Execute test procedures from file: [TEST_FILE]
3. [SHUTDOWN_INSTRUCTION]
4. Report results using the exact format below

**FAILURE HANDLING PROTOCOL:**
- **STOP ON FIRST FAILURE**: When ANY test step fails, IMMEDIATELY stop all testing. Failures include:
  - Tool returns an error or exception
  - Tool succeeds but response doesn't match test expectations
  - Behavior doesn't match expected test outcomes
- **CAPTURE EVERYTHING**: For every failed test step, include the complete tool response in your results
- **NO CONTINUATION**: Do not attempt any further test steps after the first failure

**CRITICAL: NO ISSUE IS MINOR - EVERY ISSUE IS A FAILURE**
- Error message quality issues are FAILURES, not minor issues
- Half of our codebase exists to construct proper error messages
- Any deviation from expected behavior is a FAILURE
- Do NOT categorize any issue as "minor" - mark it as FAILED

**Test Context:**
- Test File: [TEST_FILE]
- Port: [PORT]
- App: [APP_NAME]
- Objective: [TEST_OBJECTIVE]
- Expected Shutdown Method: [EXPECTED_SHUTDOWN_METHOD]

**CRITICAL ERROR HANDLING:**
- **ALWAYS use the specified port [PORT] for ALL BRP operations**
- **STOP ON FIRST FAILURE**: When ANY test step fails, IMMEDIATELY stop testing and report results
- **CAPTURE FULL RESPONSES**: For every failed test step, include the complete tool response in your results
- **TEST FAILURES INCLUDE**: 
  - HTTP request failures, connection errors, or tool exceptions
  - Tool succeeds but response data doesn't match test expectations
  - Unexpected behavior or state changes
- **FAILURE RESPONSE PROTOCOL**:
  1. STOP immediately - do not continue with remaining test steps
  2. Record the exact error message or expectation mismatch
  3. Note what operation was being attempted and what was expected
  4. **Include the full JSON response** from the tool call (successful or failed)
  5. Report the failure in your test results with complete context
- Do NOT continue testing after ANY failure (error OR expectation mismatch)
- Do NOT retry failed operations - report them as failures

**Required Response Format:**

# Test Results: [TEST_NAME]

## Configuration
- Port: [PORT]
- App: [APP_NAME]
- Launch Status: [Launched Successfully/Failed to Launch/N/A]

## Test Results
### ✅ PASSED
- [Test description]: [Brief result]
- [Test description]: [Brief result]

### ❌ FAILED
- [Test description]: [Brief result]
  - **Error**: [exact error message]
  - **Expected**: [what should happen]
  - **Actual**: [what happened]
  - **Impact**: critical (ALL ISSUES ARE CRITICAL - NO MINOR ISSUES ALLOWED)
  - **Component/Resource**: [fully qualified type name or N/A if not applicable]
  - **Full Tool Response**: [Complete JSON response from the failed tool call]

### ⚠️ SKIPPED
- [Test description]: [reason for skipping]

## Summary
- **Total Tests**: X
- **Passed**: Y
- **Failed**: Z
- **Critical Issues**: [Yes/No - brief description if yes]

## Cleanup Status
- **App Status**: [Shutdown Successfully/Still Running/N/A]
- **Shutdown Method**: [clean_shutdown/process_kill/N/A]
- **Expected Method**: [EXPECTED_SHUTDOWN_METHOD]
- **Shutdown Validation**: [PASSED/FAILED - explanation if failed]
- **Port Status**: [Available/Still in use]

**CRITICAL ERROR HANDLING:**
  - **ALWAYS use the specified port [PORT] for ALL BRP operations**
  - **STOP ON FIRST ERROR**: When ANY test step fails, IMMEDIATELY stop and return results
  - **CAPTURE FULL RESPONSES**: Include complete tool responses for ALL failed operations
  - If you encounter HTTP request failures, connection errors, or
  unexpected tool failures:
    1. **IMMEDIATELY return your test results with the failure documented**
    2. **Include the full JSON response from the failed tool**
    3. **Do not attempt any further BRP operations or test steps**
    4. **Do not relaunch the app**
    5. **Mark the test as CRITICAL FAILURE in your response**

  **When you see "MCP error -32602" or "HTTP request failed":**
  - This is a CRITICAL FAILURE
  - **Capture the complete tool response**
  - Stop immediately and return results
  - Do not continue testing

**SHUTDOWN VALIDATION REQUIREMENTS:**
  - **CRITICAL**: After shutdown, verify the shutdown response `method` field matches [EXPECTED_SHUTDOWN_METHOD]
  - If shutdown method doesn't match expected:
    1. **Mark as FAILED test** in your results
    2. Report actual vs expected shutdown method
    3. Include full shutdown response in error details
  - Expected behaviors:
    - `clean_shutdown`: Apps with BrpExtrasPlugin (extras_plugin, test_extras_plugin_app)
    - `process_kill`: Apps without BrpExtrasPlugin (no_extras_plugin)
    - `N/A`: Tests with no app launch (discovery)
  - Parse the shutdown response `data.method` field to get actual method
  - **FAILURE EXAMPLE**: If expected `clean_shutdown` but got `process_kill`, this indicates BrpExtrasPlugin configuration issue

</SubAgentPrompt>

## Execution Mode Selection

**First, check if `$ARGUMENTS` is provided:**
- If `$ARGUMENTS` exists and is not empty: Execute **Single Test Mode**
- If `$ARGUMENTS` is empty or not provided: Execute **Continuous Parallel Test Mode**

## Single Test Mode (when $ARGUMENTS provided)

### Execution Instructions

1. **Load Configuration**: Read `test_config.json` from `.claude/commands/test_config.json`
2. **Find Test**: Search for test configuration where `test_name` matches `$ARGUMENTS`
3. **Validate**: If test not found, report error and list available test names
4. **Execute Test**: If found, run the single test using the Task tool

### Single Test Execution

**For the test configuration matching `$ARGUMENTS`**:
- Create a Task with description: "BRP [test_name] Test"
- Use the SubAgentPrompt template above, substituting values from the matched test configuration:
  - [TEST_NAME] = `test_name` field
  - [TEST_FILE] = `test_file` field
  - [PORT] = `port` field
  - [APP_NAME] = `app_name` field
  - [LAUNCH_INSTRUCTION] = `launch_instruction` field
  - [SHUTDOWN_INSTRUCTION] = `shutdown_instruction` field
  - [TEST_OBJECTIVE] = `test_objective` field
  - [EXPECTED_SHUTDOWN_METHOD] = `expected_shutdown_method` field

**Example Task Invocation:**
```
Task tool with:
- Description: "BRP debug_mode Test"
- Prompt: [SubAgentPrompt with values substituted from the debug_mode config object]
```

### Error Handling

If no test configuration matches `$ARGUMENTS`:
```
# Error: Test Not Found

The test "$ARGUMENTS" was not found in .claude/commands/test_config.json.

Available tests:
- app_launch_status
- brp_extras_methods
- data_operations
- debug_mode
- discovery
- format_discovery_with_plugin
- introspection
- large_response
- no_plugin_tests
- registry_discovery
- watch_commands
- workspace_disambiguation

Usage: /test_runner <test_name>
Example: /test_runner debug_mode
```

### Final Output for Single Test

After the Task completes, simply present the test results as returned by the sub-agent. No consolidation or summary needed since it's a single test.

## Continuous Parallel Test Mode (when no $ARGUMENTS)

### Continuous Parallel Execution Instructions

**Execute tests PARALLEL_TESTS at a time with continuous execution:**

**IMPORTANT**: When you see "PARALLEL_TESTS" in this document, substitute the value from the Configuration section at the top. This means launching PARALLEL_TESTS tests concurrently.

**CRITICAL PARALLEL EXECUTION REQUIREMENT**: When executing multiple tests "at a time", you MUST invoke multiple Task tools in a SINGLE message. Sequential execution (one Task per message) is NOT parallel execution.

1. **Load Configuration**: Read `test_config.json` from `.claude/commands/test_config.json`
2. **Initialize Execution State**:
   - Create queue of all test configurations
   - Track running tests (max PARALLEL_TESTS at a time)
   - Track completed tests and their results
   - Track failed tests for immediate stopping
3. **Continuous Execution Loop**:
   - **Start Phase**: Launch first PARALLEL_TESTS tests from queue by invoking PARALLEL_TESTS Task tools IN ONE MESSAGE
   - **Monitor Phase**: Wait for any test to complete
   - **Result Phase**: Collect completed test results and check for failures
   - **Error Handling**: If any test reports failures, STOP immediately and report
   - **Continue Phase**: If no failures, start next test from queue (maintaining PARALLEL_TESTS running)
   - **Repeat**: Continue until all tests complete or failure detected

### Error Detection and Immediate Stopping

**CRITICAL**: Monitor each completed test result for failure indicators:
- Check for `### ❌ FAILED` sections with content
- Check for `**Critical Issues**: Yes` in summary
- Check for `**Shutdown Validation**: FAILED` in cleanup status
- Check for any `CRITICAL FAILURE` mentions in results

**On Error Detection**:
1. **STOP immediately** - do not start any new tests
2. **Collect results** from any currently running tests
3. **Report failure immediately** with details from failed test
4. **Skip consolidation** and provide immediate failure report

### Test Configuration and Execution

**For each test in the execution queue**:
- Create a Task with description: "BRP [test_name] Test"
- Use the SubAgentPrompt template above, substituting values from `test_config.json`
  - [TEST_NAME] = `test_name` field
  - [TEST_FILE] = `test_file` field
  - [PORT] = `port` field
  - [APP_NAME] = `app_name` field
  - [LAUNCH_INSTRUCTION] = `launch_instruction` field
  - [SHUTDOWN_INSTRUCTION] = `shutdown_instruction` field
  - [TEST_OBJECTIVE] = `test_objective` field
  - [EXPECTED_SHUTDOWN_METHOD] = `expected_shutdown_method` field

**Critical Implementation Note:**
The key requirement is that when launching PARALLEL_TESTS tests, ALL Task tool invocations MUST be in a SINGLE message. This ensures true parallel execution.

- ✅ **CORRECT**: One message containing PARALLEL_TESTS Task tool invocations
- ❌ **WRONG**: Multiple separate messages, each with one Task tool invocation

The execution maintains exactly PARALLEL_TESTS running tests at all times. When any test completes, immediately start the next test from the queue (if any remain) to maintain the parallel count.

### Results Consolidation

**Success Path**: After all tests complete successfully, generate consolidated summary
**Failure Path**: If any test fails, provide immediate failure report instead

### Immediate Failure Report (when error detected)

# BRP Test Suite - FAILED

## ❌ CRITICAL FAILURE DETECTED

**Failed Test**: [test_name]
**Failure Type**: [Critical Issues/Failed Tests/Shutdown Validation/etc.]

### Failure Details
[Include full failure details from the failed test]

### Tests Completed Before Failure
- **Completed**: X tests
- **Results**: [Brief summary of completed tests]

### Tests Not Executed
- **Remaining**: Y tests
- **Reason**: Execution stopped due to failure

**Recommendation**: Fix the failure in [test_name] before running remaining tests.

### Final Summary Format for All Tests (Success Path Only)

# BRP Test Suite - Consolidated Results

## Overall Statistics
- **Total Tests**: 12
- **Passed**: X
- **Failed**: 0 (execution stops on first failure)
- **Skipped**: Y
- **Critical Issues**: 0 (execution stops on critical issues)
- **Total Execution Time**: ~X minutes (continuous parallel)
- **Execution Strategy**: PARALLEL_TESTS tests at a time with continuous execution

## ✅ PASSED TESTS
[List of successful tests with brief summaries, in execution order]

## ⚠️ SKIPPED TESTS
[List of skipped tests with reasons]

## Execution Flow Summary
[Brief summary of test execution order and timing]

## Port Usage Summary
[Status of all test ports after completion]

## Recommendations
[Based on test results - all passed scenario]
