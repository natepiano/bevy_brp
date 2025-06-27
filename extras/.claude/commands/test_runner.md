# BRP Test Suite Runner - Parallel Execution

## Test Configuration

**Configuration Source**: `test_config.json` (in same directory)

This file contains an array of test configurations with the following structure:
- `test_name`: Identifier for the test
- `test_file`: Test file name in the tests/ directory
- `port`: Port number for the test (or "N/A")
- `app_name`: App/example to launch (or "N/A")
- `launch_instruction`: How to launch the app
- `shutdown_instruction`: How to shutdown the app
- `test_objective`: What the test validates

**Total Tests**: 12 tests

## Sub-agent Prompt Template

<SubAgentPrompt>

You are executing BRP test: [TEST_NAME]
Configuration: Port [PORT], App [APP_NAME]

**Your Task:**
1. [LAUNCH_INSTRUCTION]
2. Execute test procedures from file: tests/[TEST_FILE]
3. [SHUTDOWN_INSTRUCTION]
4. Report results using the exact format below

**Test Context:**
- Test File: [TEST_FILE]
- Port: [PORT]
- App: [APP_NAME]
- Objective: [TEST_OBJECTIVE]

**CRITICAL ERROR HANDLING:**
- **ALWAYS use the specified port [PORT] for ALL BRP operations**
- If you encounter HTTP request failures, connection errors, or unexpected tool failures:
  1. STOP immediately
  2. Record the exact error message
  3. Note what operation was being attempted
  4. Report the failure in your test results
- Do NOT continue testing after unexpected errors
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
  - **Impact**: [critical/minor]

### ⚠️ SKIPPED
- [Test description]: [reason for skipping]

## Summary
- **Total Tests**: X
- **Passed**: Y
- **Failed**: Z
- **Critical Issues**: [Yes/No - brief description if yes]

## Cleanup Status
- **App Status**: [Shutdown Successfully/Still Running/N/A]
- **Port Status**: [Available/Still in use]

**CRITICAL ERROR HANDLING:**
  - **ALWAYS use the specified port [PORT] for ALL BRP operations**
  - If you encounter HTTP request failures, connection errors, or
  unexpected tool failures:
    1. **IMMEDIATELY return your test results with the failure
  documented**
    2. **Do not attempt any further BRP operations**
    3. **Do not relaunch the app**
    4. **Mark the test as CRITICAL FAILURE in your response**

  **When you see "MCP error -32602" or "HTTP request failed":**
  - This is a CRITICAL FAILURE
  - Stop immediately and return results
  - Do not continue testing

</SubAgentPrompt>

## Parallel Execution Instructions

**Execute ALL tests simultaneously using the Task tool:**

1. **Load Configuration**: Read `test_config.json` from the same directory
2. **For each test configuration object**:
   - Create a Task with description: "BRP [test_name] Tests"
   - Use the SubAgentPrompt template above, substituting values from `test_config.json`
     - [TEST_NAME] = `test_name` field
     - [TEST_FILE] = `test_file` field
     - [PORT] = `port` field
     - [APP_NAME] = `app_name` field
     - [LAUNCH_INSTRUCTION] = `launch_instruction` field
     - [SHUTDOWN_INSTRUCTION] = `shutdown_instruction` field
     - [TEST_OBJECTIVE] = `test_objective` field

**Example Task Invocation:**
```
Task tool with:
- Description: "BRP app_launch_status Tests"
- Prompt: [SubAgentPrompt with values substituted from the app_launch_status config object]
```

**Launch all 12 tasks simultaneously for maximum parallel execution efficiency.**

## Results Consolidation

After all sub-agents complete, collect their individual results and generate a consolidated summary:

### Final Summary Format

# BRP Test Suite - Consolidated Results

## Overall Statistics
- **Total Tests**: 12
- **Passed**: X
- **Failed**: Y
- **Skipped**: Z
- **Critical Issues**: [Count]
- **Total Execution Time**: ~X minutes (parallel)

## ✅ PASSED TESTS
[List of successful tests with brief summaries]

## ❌ FAILED TESTS
[List of failed tests with key details]

## ⚠️ SKIPPED TESTS
[List of skipped tests with reasons]

## Critical Issues Summary
[Any issues requiring immediate attention]

## Port Usage Summary
[Status of all test ports]

## Recommendations
[Based on test results]
