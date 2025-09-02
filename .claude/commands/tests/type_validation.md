# Type Schema Comprehensive Validation Test

## Objective
Systematically validate ALL BRP component types by testing spawn/insert and mutation operations using individual type schema files. This test tracks progress in `test-app/examples/type_validation.json` to avoid retesting passed types.

**CRITICAL EXECUTION REQUIREMENTS**:
1. **ALWAYS reassign batch numbers** - Even if batch numbers exist, clear and reassign them EVERY time the test runs
2. **ALWAYS use parallel subagents** - Launch 10 parallel subagents per batch, one subagent per type
3. **Main agent orchestrates, subagents test** - The main agent NEVER tests types directly; it only manages batches and launches subagents
4. **Test ALL types without stopping** - Continue through all batches unless there's an actual failure

**EXECUTION ARCHITECTURE**:
- **Main Agent (You)**: Clears/assigns batch numbers, launches subagents, collects structured results, performs single atomic JSON update, manages app restarts if needed
- **Subagents**: Each tests exactly ONE type - gets schema, tests spawn, tests all mutations, returns structured results (NO JSON updates)
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

1. **Load progress**: Read `test-app/examples/type_validation.json` to identify untested types
2. **Assign batches**: Group untested types into batches of 10 for parallel processing  
3. **Launch subagents**: Process each batch with parallel subagents, one per type
4. **Collect results**: Gather structured results from all subagents in the batch
5. **Update progress**: Main agent performs atomic JSON updates with all batch results
6. **Continue or stop**: Process next batch if all passed, stop on any failure

## CRITICAL EXECUTION REQUIREMENTS

**BATCH PROCESSING MODEL**:
1. Process types in batches of up to 10 parallel subagents
2. Main agent collects all subagent results before updating JSON
3. Atomic JSON updates after each complete batch
4. Stop testing immediately on any failure or COMPONENT_NOT_FOUND

**EXECUTION FLOW**:
1. Clear and reassign batch numbers for untested types
2. Launch parallel subagents for each batch
3. Collect structured results from all subagents
4. Update JSON file atomically with all batch results  
5. Continue to next batch or stop on failure

## Test Steps

### 0. Test Execution Model

**EXECUTION MODEL**: The main agent (you) ALWAYS orchestrates the test by launching parallel subagents. This is NOT optional - parallel subagents are REQUIRED for testing individual types.

1. Main agent: Manages batches, launches subagents, collects structured results, performs atomic JSON updates, handles app restarts
2. Subagents: Test individual types (one subagent per type) and return structured results (NO JSON updates)
3. Parallelism: Up to 10 subagents run simultaneously per batch

Do NOT display statistics, counts, or summaries.

**NOTE**: Main agent uses Read/Write/Edit tools for batch progress updates. Use bash commands only for initial batch number management.

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
    description="Test [short_name] (batch [X], test [Y]/[BatchSize])",
    subagent_type="general-purpose", 
    prompt="""Test the type: [full::qualified::type::name]

[Copy the ENTIRE <TestInstructions> section content here - all testing steps and critical rules]

Return structured JSON result ONLY as your final response."""
)
```

Where:
- `[X]` = batch number (1, 2, 3, etc.)
- `[Y]` = position within batch (1, 2, 3, etc.)
- `[BatchSize]` = actual number of types in this batch (typically 10, but may be less for final batch)

**Execution Pattern**:
1. Identify all types in current batch (up to 10)
2. Create one Task call for EACH type using the template above
3. Send ALL Task calls in a SINGLE message to run in parallel
4. Wait for ALL subagents to complete
5. **Process Results and Update JSON**:
   - **Collect all structured results** from subagents
   - **Single atomic JSON update** by main agent with all batch results
   - **Check for failures** in collected results:
     - If ANY result has `status: "COMPONENT_NOT_FOUND"`: 
       - Stop all testing
       - Add the missing component to extras_plugin.rs
       - Restart the app (shutdown, relaunch with auto-build)
       - Retry the SAME batch (do not reassign batch numbers)
     - If ANY result has `status: "FAIL"`: 
       - Stop testing immediately
       - Report the specific failure reason to the user
     - If ALL results have `status: "PASS"`: Continue to next batch

#### Individual Type Testing Instructions (For Subagents)

Each subagent receives these instructions for testing a single assigned type:

<TestInstructions>
**Your Task**: Test ONLY the assigned component type - both spawn operations and all mutation paths. Return structured results to main agent.

**Port**: Use port 20116 for ALL BRP operations.

**CRITICAL**: You MUST NOT update any JSON files. Return structured results only.

**Sequential Steps to Execute**:

1. **Get Type Schema**
   - Call `mcp__brp__brp_type_schema` with your assigned type name and port 20116
   - Extract `supported_operations`, `mutation_paths`, and `spawn_format` from the result
   - If the type is not in registry, return failure result and STOP

2. **Test Spawn/Insert Operations** (Only if supported)
   - Check if 'spawn' or 'insert' is in `supported_operations`
   - If YES:
     - Get the `spawn_format` from the schema
     - Call `mcp__brp__bevy_spawn` with the type and spawn_format
     - Record result as "passed" or "failed"
   - If NO spawn/insert support: Skip spawn testing

3. **Prepare for Mutation Testing** 
   - Check if 'mutate' is in `supported_operations`
   - If NO: Skip mutation testing
   - If YES: Use `mcp__brp__bevy_query` to find an entity with this component
   - If no entity exists (entity_count == 0): Return COMPONENT_NOT_FOUND result

4. **Test ALL Mutation Paths** (Only if mutations supported and component exists)
   - For EACH path in `mutation_paths` (no exceptions, test them ALL):
     - Determine test value from the path_info (use example value provided)
     - Call `mcp__brp__bevy_mutate_component` with entity, component, path, value, port 20116
     - CRITICAL: For empty paths use `""` not `"\"\""`
     - Record result as "passed" or "failed" for each path
   - Continue testing ALL paths even if some fail

5. **Return Structured Result**
   Return ONLY this JSON structure as your final response:
   ```json
   {
     "type": "[full::qualified::type::name]",
     "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
     "spawn_test_attempted": true|false,
     "spawn_test": "passed|failed",
     "mutation_tests_attempted": true|false, 
     "mutation_tests": "passed|failed",
     "mutation_paths": {
       ".path1": "passed|failed",
       ".path2": "passed|failed"
     },
     "notes": "Any error details or explanations",
     "error_details": "Specific failure information if status is FAIL"
   }
   ```

**Result Status Rules**:
- `status: "PASS"` - All attempted tests passed
- `status: "FAIL"` - Any test failed  
- `status: "COMPONENT_NOT_FOUND"` - Component doesn't exist in app for mutation testing

**Field Rules**:
- Only include `spawn_test` if `spawn_test_attempted: true`
- Only include `mutation_tests` if `mutation_tests_attempted: true`
- `mutation_paths` should contain results for ALL paths tested
- `mutation_tests` is "passed" only if ALL mutation paths passed
- `mutation_tests` is "failed" if ANY mutation path failed

**CRITICAL RULES**:
- Test ONLY your assigned type - do not test any other types
- Test EVERY mutation path - no shortcuts or sampling
- **DO NOT update any JSON files** - return results only
- Use port 20116 for all BRP operations
- Return the JSON structure above as your ONLY response
</TestInstructions>



### 3. Batch Completion and Progress Updates

**CRITICAL**: After each batch completes, main agent processes all results and updates JSON atomically.

After each batch:
1. **Collect structured results**: Parse JSON responses from all subagents in the batch
2. **Validate results**: Ensure all subagents returned valid JSON with required fields
3. **Process results by status**:
   - `COMPONENT_NOT_FOUND`: Stop testing, handle missing component, restart app, retry batch
   - `FAIL`: Stop testing immediately, report failure details  
   - `PASS`: Add to successful results for JSON update
4. **Single atomic JSON update**: Main agent updates `test-app/examples/type_validation.json` with all batch results
5. **Continue or stop**: If all results are PASS, continue to next batch; otherwise stop

**Main Agent JSON Update Responsibility**: 
- Convert subagent structured results to JSON file format
- Update spawn_test, mutation_tests, mutation_paths for each type
- Set appropriate notes based on subagent error_details
- Verify JSON update succeeded before continuing

**NO CONCURRENT FILE ACCESS**: Only the main agent touches the JSON file, eliminating race conditions.

**JSON Update Implementation**:
- Use Read/Write/Edit tools exclusively for JSON file updates
- Do NOT create backup files (.bak) - files are under git source control
- Do NOT use bash commands like jq with redirects/pipes to edit JSON files
- Main agent performs single atomic update after collecting all batch results
- Verify JSON update succeeded before continuing to next batch

### Main Agent JSON Update Implementation

**CRITICAL**: Main agent performs single atomic JSON updates using Read/Write tools after collecting all subagent results.

**Batch Result Processing Workflow**:
1. **Collect Results**: Parse all subagent JSON responses into structured data
2. **Load Current State**: Use Read tool to load `test-app/examples/type_validation.json`
3. **Update Entries**: For each subagent result, find matching type entry and update:
   - `spawn_test`: Set to "passed"/"failed" if `spawn_test_attempted: true`, otherwise leave as "skipped" 
   - `mutation_tests`: Set to "passed"/"failed" if `mutation_tests_attempted: true`, otherwise leave as existing value
   - `mutation_paths`: Update with all path results from subagent
   - `notes`: Set based on subagent `error_details` if status is FAIL
4. **Atomic Update**: Use Write tool to save complete updated JSON back to file
5. **Verification**: Confirm all updates were saved correctly

**Subagent Result to JSON Mapping**:
```
subagent.spawn_test_attempted = true → json.spawn_test = subagent.spawn_test
subagent.spawn_test_attempted = false → json.spawn_test = "skipped" (or leave existing)
subagent.mutation_tests_attempted = true → json.mutation_tests = subagent.mutation_tests  
subagent.mutation_tests_attempted = false → json.mutation_tests = unchanged
subagent.mutation_paths → json.mutation_paths (direct copy)
subagent.error_details → json.notes (if status is FAIL)
```

**Error Handling**:
- If Write operation fails, retry once after 2 second delay
- If JSON parsing fails, report error and stop testing
- If subagent result validation fails, treat as FAIL status

### 4. Execution Flow and Communication

**BATCH-LEVEL PROGRESS UPDATES**: After each complete batch, main agent updates JSON file atomically with all batch results.

**USER COMMUNICATION RULES**: 
- Report batch completion and any critical failures
- Do NOT provide detailed progress summaries during batch execution
- Do NOT pause between individual type tests within a batch
- Do NOT stop for progress reports - continue through all batches unless failure occurs

**EXECUTION FLOW**:
- Batch processing = REQUIRED with parallel subagents  
- JSON file updates = REQUIRED after each complete batch by main agent only
- Stopping criteria = Only on actual test failures or COMPONENT_NOT_FOUND


## Success Criteria

✅ Test passes when:
- All batches with untested types are processed in parallel
- All subagents in each batch report PASS
- Spawn/insert operations work for supported types  
- All mutation paths work for supported types
- Progress is saved by main agent after each batch completes

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
