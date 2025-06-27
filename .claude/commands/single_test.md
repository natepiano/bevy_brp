# BRP Single Test Runner

## Test Configuration

**Configuration Source**: `.claude/commands/test_config.json`
**Test Name**: `$ARGUMENTS` (provided as command parameter)

This command runs a single test from the test suite by name.

## Usage Examples
```
/single_test debug_mode
/single_test data_operations
/single_test brp_extras_methods
```

## Execution Instructions

1. **Load Configuration**: Read `test_config.json` from `.claude/commands/test_config.json`
2. **Find Test**: Search for test configuration where `test_name` matches `$ARGUMENTS`
3. **Validate**: If test not found, report error and list available test names
4. **Execute Test**: If found, run the single test using the Task tool

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

## Single Test Execution

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

**Example Task Invocation:**
```
Task tool with:
- Description: "BRP debug_mode Test"
- Prompt: [SubAgentPrompt with values substituted from the debug_mode config object]
```

## Error Handling

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

Usage: /single_test <test_name>
Example: /single_test debug_mode
```

## Final Output

After the Task completes, simply present the test results as returned by the sub-agent. No consolidation or summary needed since it's a single test.