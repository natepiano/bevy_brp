# Type Schema Comprehensive Validation Test

## Objective
Systematically validate ALL BRP component types by testing spawn/insert and mutation operations using individual type schema files. This test tracks progress in `test-app/examples/type_validation.json` to avoid retesting passed types.

**CRITICAL EXECUTION REQUIREMENTS**:
1. **ALWAYS reassign batch numbers** - Even if batch numbers exist, clear and reassign them EVERY time the test runs
2. **ALWAYS use parallel subagents** - Launch 10 parallel subagents per batch, one subagent per type
3. **Main agent orchestrates, subagents test** - The main agent NEVER tests types directly; it only manages batches and launches subagents
4. **Test ALL types without stopping** - Continue through all batches unless there's an actual failure

**EXECUTION ARCHITECTURE**:
- **Main Agent (You)**: Clears/assigns batch numbers, launches subagents, monitors results, manages app restarts if needed
- **Subagents**: Each tests exactly ONE type - gets schema, tests spawn, tests all mutations, updates JSON
- **Parallelism**: 10 subagents run simultaneously per batch (all launched in one message)

**BATCH STRUCTURE**:
- Batch 1: Types 0-9 (up to 10 types)
- Batch 2: Types 10-19 (up to 10 types)  
- Batch 3: Types 20-29 (up to 10 types)
- And so on...

**NOTE**: The extras_plugin app is already running on the specified port - subagents will connect to it for testing.

**COMPONENT NOT FOUND HANDLING** (Main Agent Only):
When ANY subagent returns "COMPONENT_NOT_FOUND":
1. Stop all testing immediately
2. Identify which component(s) need to be added from subagent reports
3. Modify `test-app/examples/extras_plugin.rs` to add the missing component(s)
4. Shutdown the app: `mcp__brp__brp_shutdown(app_name="extras_plugin", port=20116)`
5. Relaunch with auto-build: `mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=20116)`
6. Reset window title: `mcp__brp__brp_extras_set_window_title(port=20116, title="type_validation test - port 20116")`
7. Retry the SAME batch (do not reassign batch numbers)

## Schema Source
- **Type schemas**: Retrieved dynamically via `mcp__brp__brp_type_schema` tool
- **Progress tracking**: `test-app/examples/type_validation.json` (single file tracking all types)

## Progress Tracking

The test uses a single JSON file to track testing progress:
- `test-app/examples/type_validation.json`: Contains all types with their current test status

Each type entry has the following structure:

```json
{
  "type": "bevy_transform::components::transform::Transform",
  "spawn_test": "untested",  // "untested", "passed", "failed", "skipped" (if no Serialize/Deserialize)
  "mutation_tests": "untested",  // "untested", "passed", "failed", "n/a" (if no mutations)
  "mutation_paths": {},  // Object with path as key, status as value (e.g., {".translation.x": "passed", ".rotation": "failed"})
  "notes": ""  // Additional notes about failures or special cases
}
```

**IMPORTANT**: When testing mutations, track each individual path result in the `mutation_paths` object. The overall `mutation_tests` field should reflect the aggregate status (all passed = "passed", any failed = "failed", no mutations = "n/a").

When a type is tested, update its status in place:
- Only test types where `spawn_test` is "untested" - change to "passed" or "failed" 
- Update each path in `mutation_paths` from "untested" to "passed" or "failed" as you test them
- Only test types where `mutation_tests` is "untested" - change to "passed" or "failed"
- Add any relevant notes about failures or special handling

## Test Strategy

1. **Load progress**: Read `test-app/examples/type_validation.json` to see which types have been tested
2. **Skip passed types**: Don't retest types where both spawn_test and mutation_tests are "passed"
3. **Build todo list**: Create tasks only for untested or failed types
4. **Test each type**: Load individual schema file and test operations
5. **Update progress**: Update type status in `test-app/examples/type_validation.json`
6. **STOP ON FIRST FAILURE** - Continue testing all types unless an actual failure occurs. Do not stop for successful completions, progress updates, or user checkpoints. The user will manually stop if needed.

## CRITICAL EXECUTION REQUIREMENTS

**PROGRESS UPDATE ENFORCEMENT**:
1. You MUST update progress files immediately after completing each type
2. You MUST NOT proceed to the next type until progress files are updated
3. You MUST NOT batch progress updates for multiple types
4. Failure to follow this will be considered a critical test execution error

**EXECUTION ORDER FOR EACH TYPE**:
1. Get type schema
2. Test spawn operations (if supported)
3. Test all mutation paths (if supported)
4. **IMMEDIATELY update type status in test-app/examples/type_validation.json and continue to next type**
5. **This is a single continuous action - do not pause between steps**

## CRITICAL TEST EXECUTION ERRORS

The following actions constitute test execution failures:
- Stopping to provide progress summaries
- Explaining what you're doing or why
- Demonstrating the testing approach
- Pausing between types for any reason
- Any form of user communication except failure reporting

## Test Steps

### 0. Test Execution Model

**EXECUTION MODEL**: The main agent (you) ALWAYS orchestrates the test by launching parallel subagents. This is NOT optional - parallel subagents are REQUIRED for testing individual types.

1. Main agent: Manages batches, launches subagents, monitors results, handles app restarts
2. Subagents: Test individual types (one subagent per type) and update JSON directly
3. Parallelism: Up to 10 subagents run simultaneously per batch

Do NOT display statistics, counts, or summaries.

**NOTE**: Use Read/Write/Edit tools for progress updates during testing. Use bash commands only for initial batch number management as specified below.

### 1. Load Progress and Reassign Batch Numbers (MANDATORY EVERY RUN)

**CRITICAL**: This step is MANDATORY even if batch numbers already exist. We ALWAYS clear and reassign to ensure correct batching.

1. Use Read tool to load `test-app/examples/type_validation.json`
2. **MANDATORY: Clear all batch numbers**: Use bash command to set ALL batch_number fields to null
3. **MANDATORY: Assign new batch numbers**: Assign batch numbers ONLY to untested types in groups of 10
4. Identify untested types:
   - Types where spawn_test is "untested" OR
   - Types where mutation_tests is "untested" OR
   - Types that failed previously (not "passed"/"skipped" for spawn, not "passed"/"n/a" for mutations)
5. Skip types that are fully tested (spawn is "passed"/"skipped" AND mutations are "passed"/"n/a")
6. The resulting batches will be used for parallel subagent testing

#### Batch Number Management

**Step 1: Clear all batch numbers (in place)**
Use Bash tool to run this command (requires permission):
```bash
jq 'map(.batch_number = null)' test-app/examples/type_validation.json > /tmp/type_validation_temp.json && mv /tmp/type_validation_temp.json test-app/examples/type_validation.json
```

**Step 2: Assign batch numbers to untested types only (in place)**
Use Bash tool to run this command (requires permission):
```bash
jq '
  # First, number only untested types sequentially
  [.[] | select(.spawn_test == "untested" or .mutation_tests == "untested")] as $untested |
  
  # Create a lookup map of untested types with their batch numbers
  ($untested | to_entries | map({key: .value.type, value: ((.key / 10) | floor + 1)}) | from_entries) as $batch_map |
  
  # Update each type in place
  map(
    if (.spawn_test == "untested" or .mutation_tests == "untested") then
      .batch_number = $batch_map[.type]
    else
      .batch_number = null
    end
  )
' test-app/examples/type_validation.json > /tmp/type_validation_temp.json && mv /tmp/type_validation_temp.json test-app/examples/type_validation.json
```

This ensures that:
- All previously tested types have null batch_number
- Only untested types get assigned numeric batch numbers (1, 2, 3, etc.)
- Batch numbers are assigned in groups of 10 (0-9 → batch 1, 10-19 → batch 2, etc.)

### 2. Test Types Using Parallel Subagents (10 Types Per Batch)

**EXECUTION MODEL**: The main agent orchestrates; subagents do the actual testing.

1. **Process batches sequentially**: Test batch 1, wait for completion, then batch 2, etc.
2. **For EACH batch, launch up to 10 parallel subagents**: 
   - One subagent per type in the batch
   - All subagents in a batch run simultaneously
   - Example: Batch 1 with 10 types = 10 parallel subagents running at once
3. **Wait for ALL subagents in the batch to complete** before proceeding
4. **Stop on any failure**: If ANY subagent reports FAIL, stop the entire test immediately

#### Batch Processing Logic

For each batch (1, 2, 3, etc.):

1. **Collect batch types**: Get all types with the current batch_number
2. **Launch parallel subagents**: Create one Task tool call per type in the batch
3. **Wait for all subagents to complete**: Gather results from all subagents in the batch
4. **Check for failures**: If any subagent reports failure, stop testing
5. **Continue to next batch**: If all passed, process the next batch_number

#### Parallel Subagent Execution

**CRITICAL**: Launch ALL subagents for a batch in a SINGLE message with multiple Task tool calls.

**Template for EACH subagent task**:
```python
Task(
    description="Test [short_name]",
    subagent_type="general-purpose", 
    prompt="""Test the type: [full::qualified::type::name]

[Copy the ENTIRE <TestInstructions> section content here - all 6 steps and critical rules]

Return only "PASS" or "FAIL" as your final response."""
)
```

**Execution Pattern**:
1. Identify all types in current batch (up to 10)
2. Create one Task call for EACH type using the template above
3. Send ALL Task calls in a SINGLE message to run in parallel
4. Wait for ALL subagents to complete
5. Check results:
   - If ANY returns "COMPONENT_NOT_FOUND": 
     - Stop all testing
     - Add the missing component to extras_plugin.rs
     - Restart the app (shutdown, relaunch with auto-build)
     - Retry the SAME batch (do not reassign batch numbers)
   - If ANY returns "FAIL" (including "FAIL - JSON_UPDATE_FAILED"): 
     - Stop testing immediately
     - Report the specific failure reason to the user
     - For JSON_UPDATE_FAILED: Discuss the issue before proceeding
   - If ALL return "PASS": Continue to next batch

#### Individual Type Testing Instructions (For Subagents)

Each subagent receives these instructions for testing a single assigned type:

<TestInstructions>
**Your Task**: Test ONLY the assigned component type - both spawn operations and all mutation paths.

**Port**: Use port 20116 for ALL BRP operations.

**Sequential Steps to Execute**:

1. **Get Type Schema**
   - Call `mcp__brp__brp_type_schema` with your assigned type name and port 20116
   - Extract `supported_operations`, `mutation_paths`, and `spawn_format` from the result
   - If the type is not in registry, mark as failed and STOP

2. **Test Spawn/Insert Operations**
   - Check if 'spawn' or 'insert' is in `supported_operations`
   - If YES:
     - Get the `spawn_format` from the schema
     - Call `mcp__brp__bevy_spawn` with the type and spawn_format
     - If successful: Mark `spawn_test` as "passed"
     - If fails: Mark as "failed" with error details
   - If NO spawn/insert support: Mark `spawn_test` as "skipped" with note "No spawn/insert support"

3. **Prepare for Mutation Testing**
   - Check if 'mutate' is in `supported_operations`
   - If NO: Mark `mutation_tests` as "n/a" and skip to step 5
   - If YES: Use `mcp__brp__bevy_query` to find an entity with this component
   - If no entity exists (entity_count == 0):
     - Update JSON file with note: "COMPONENT_NOT_FOUND - needs extras_plugin.rs update"
     - Return "COMPONENT_NOT_FOUND" (special status for main agent to handle)

4. **Test ALL Mutation Paths**
   - For EACH path in `mutation_paths` (no exceptions, test them ALL):
     - Determine test value from the path_info (use example value provided)
     - Call `mcp__brp__bevy_mutate_component` with entity, component, path, value, port 20116
     - CRITICAL: For empty paths use `""` not `"\"\""`
     - If successful: Mark this path as "passed" in mutation_paths object
     - If failed: Mark this path as "failed" and continue to next path
   - After testing ALL paths:
     - If all passed: Set `mutation_tests` to "passed"
     - If any failed: Set `mutation_tests` to "failed"
     - If no paths exist: Set `mutation_tests` to "n/a"

5. **Update Progress File (MANDATORY - FAILURE TO UPDATE IS A TEST FAILURE)**
   - **CRITICAL**: You MUST update the JSON file. Not updating is a FAIL condition.
   - Use Read tool to load `test-app/examples/type_validation.json`
   - Find your type's entry and update:
     - `spawn_test`: "passed", "failed", or "skipped"
     - `mutation_tests`: "passed", "failed", or "n/a"
     - `mutation_paths`: Object with each path's result
     - `notes`: Any relevant failure details
   - Use Write tool to save the updated JSON
   - If Write fails due to conflict: Wait 2 seconds, re-read the file, and try update again (max 3 retries)
   - **VERIFICATION**: After Write, use Read tool again to verify your updates were saved
   - If updates were NOT saved after all retries, this is a CRITICAL FAILURE

6. **Verify Update Success (MANDATORY)**
   - Use Read tool to load `test-app/examples/type_validation.json` one final time
   - Verify that your type's entry has been updated with your test results
   - If the type still shows "untested" or doesn't reflect your changes:
     - Return "FAIL - JSON_UPDATE_FAILED: Failed to update type_validation.json after [number] attempts. Reason: [specific error or issue encountered]"
   - Only proceed to step 7 if the update was successful

7. **Return Result**
   - Return "PASS" if all applicable tests passed AND JSON was successfully updated
   - Return "FAIL" if any test failed
   - Return "FAIL - JSON_UPDATE_FAILED: [reason]" if JSON update failed
   - Return "COMPONENT_NOT_FOUND" if component doesn't exist in app (step 3)

**CRITICAL RULES**:
- Test ONLY your assigned type - do not test any other types
- Test EVERY mutation path - no shortcuts or sampling
- **MANDATORY**: Update the JSON file before returning - failure to update = test FAIL
- **MANDATORY**: Verify the JSON update was successful - unverified update = test FAIL
- Use port 20116 for all BRP operations
- If component doesn't exist in app, that's a FAILURE not a skip
- If JSON update fails, return "FAIL - JSON_UPDATE_FAILED" with specific reason
</TestInstructions>



### 3. Batch Completion and Progress Updates

**CRITICAL**: After each batch completes, check subagent results and update progress.

After each batch:
1. **Collect subagent results**: Each subagent reports PASS/FAIL for their assigned type
2. **Check for failures**: If any subagent reports FAIL, stop testing immediately
3. **Verify progress updates**: Each subagent should have updated test-app/examples/type_validation.json
4. **Continue to next batch**: If all subagents report PASS, process the next batch_number

**Subagent Responsibility**: Each subagent MUST update the JSON file with their type's test results before returning PASS/FAIL.

**FAILURE TO UPDATE PROGRESS BY SUBAGENTS WILL BE CONSIDERED A TEST EXECUTION ERROR**

**IMPORTANT**:
- Do NOT create backup files (.bak or similar) when updating these JSON files. The files are already under source control (git), which provides version history and backup functionality.
- Do NOT use bash commands like jq with redirects/pipes to edit JSON files - use Read/Write or Edit tools instead
- Always use proper file editing tools (Read/Write, Edit, MultiEdit) to update JSON files

### Progress Update Implementation

**CRITICAL**: Use the Read and Write tools, NOT bash commands, to update the JSON files. This ensures the test can run unattended without requiring user approval for file modifications.

For updating type status after testing:
1. Use Read tool to load the current JSON file
2. Parse the JSON data and find the entry to update
3. Modify the entry with new status/paths/notes
4. Use Write tool to save the updated JSON back to the file

Example workflow for updating a type's status:
1. Use Read tool to get the current content of `test-app/examples/type_validation.json`
2. Parse the JSON data internally
3. Find the entry matching the type name and update:
   - spawn_test: "passed", "failed", or "skipped"
   - mutation_tests: "passed", "failed", or "n/a"
   - mutation_paths: Object with path statuses
   - notes: Any relevant notes
4. Use Write tool to save the complete updated JSON back to the file

**NEVER use bash commands like:**
- `jq 'map(...)' file.json > /tmp/updated.json && mv /tmp/updated.json file.json`
- `jq ... file.json | tee file.json`
- Any command involving `>`, `>>`, or file redirection that requires approval

**ALWAYS use file editing tools that don't require approval:**
- Read tool to get file contents
- Write tool to save complete updated contents
- Edit tool for specific string replacements
- MultiEdit for multiple changes in one operation

### 4. Progress File Updates vs Progress Reporting

**MANDATORY PROGRESS FILE UPDATES**: After each type is completed, you MUST immediately update the JSON file:
- Update the type's status in `test-app/examples/type_validation.json`

**ABSOLUTE PROHIBITION ON USER COMMUNICATION**: Do NOT stop, pause, or communicate ANYTHING to the user including summaries, progress reports, status updates, explanations, demonstrations, or any other form of communication. Silent execution only. ANY communication is a critical test execution error.

**DISTINCTION**:
- JSON file updates = REQUIRED as part of continuous flow to next type
- User progress reports = FORBIDDEN
- Pausing between types = FORBIDDEN


## Success Criteria

✅ Test passes when:
- All batches with untested types are processed in parallel
- All subagents in each batch report PASS
- Spawn/insert operations work for supported types  
- All mutation paths work for supported types
- Progress is saved by each subagent for their assigned type

**IMPORTANT**: The test executor should process ALL batches sequentially, with parallel subagent testing within each batch, without stopping unless there's an actual failure. User interruption is their choice, not the executor's responsibility.

## Failure Handling

**On failure**:

### Standard Failure (bugs, missing components, etc.)
1. Mark type as "failed" in test-app/examples/type_validation.json
2. Record failure details:
   - Operation that failed (spawn/insert/mutate)
   - Error message
   - Path (for mutations)
3. **STOP TESTING** - Only stop when there's an actual failure
4. Save progress so test can resume later if stopped due to failure

### Special Case: Application Crash During Testing

When the application crashes or becomes unresponsive during testing (HTTP request failures, connection errors):

1. **SAFETY CLEANUP**: Run `mcp__brp__brp_shutdown` as safety cleanup (even if it fails due to crash)
2. **RESTART APPLICATION**:
   - Launch extras_plugin again: `mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=20116)`
   - Verify BRP connectivity: `mcp__brp__brp_status`
   - Set window title: `mcp__brp__brp_extras_set_window_title`
3. **MARK CRASH TYPE**: Update the current type's progress with:
   - spawn_test or mutation_tests: "failed"
   - notes: "App crashed during [operation] - [specific mutation path if applicable]"
4. **CONTINUE TESTING**: Resume with the next type in the sequence
5. **FAILURE THRESHOLD**: If the same type crashes the app 2+ times, STOP testing and report the issue


## CRITICAL: No Component Exceptions

**There are NO exceptions for any component types.** If a component:
1. Supports mutations (`'mutate' in supported_ops`)
2. Has no existing entities (`entity_count == 0`)

Then it MUST be added to extras_plugin.rs regardless of assumptions about:
- Whether it's "computed"
- Whether it's "system-managed"
- Whether it "should" be manually added
- Any other reasoning

**The protocol is absolute: Test ALL mutation-capable components.**

## Known Issues - STOPPING CONDITIONS

**CRITICAL**: These issues require IMMEDIATE test stopping:

1. **Schema format mismatch**: When BRP rejects a format that the schema tool generated - mark as FAILED
2. **Component not found**: When a mutation-only component doesn't exist - handled by main agent restart
3. **Any test failure**: Stop entire test suite on first failure

## Resume Capability

The test can be resumed at any time:
1. Previously passed types are skipped
2. Failed types can be retried
3. Untested types are processed in order

**IMPORTANT**: Resume capability exists for when tests are stopped due to failures or manual user intervention. The test executor should NOT proactively stop for checkpoints, progress reports, or successful completions. Process all types continuously unless an actual failure occurs.

This allows incremental testing and debugging of specific type issues when failures occur.
