# Type Guide Comprehensive Validation Test

<InstallWarning>
## IMPORTANT NOTE ##
If you have recently made changes and haven't intalled it, then you need to install it according to the instructions in ./~claude/commands/build_and_install.md

You can ignore this if no changes have been made.
</InstallWarning>

**CRITICAL**: Read and execute the tagged sections below in the specified order using the <ExecutionFlow/> workflow.

<ExecutionFlow/>

<TestContext>
[COMMAND]: `/mutation_test`
[PURPOSE]: Systematically validate ALL BRP component types by testing spawn/insert and mutation operations
[PROGRESS_FILE]: `$TMPDIR/all_types.json` - Complete type guides with test status tracking
[ARCHITECTURE]: Main agent orchestrates, subagents test in parallel
</TestContext>

<TestConfiguration>
TYPES_PER_SUBAGENT = 1                    # Types each subagent tests
MAX_SUBAGENTS = 10                        # Parallel subagents per batch
BATCH_SIZE = 10                           # Types per batch (MAX_SUBAGENTS * TYPES_PER_SUBAGENT)
BASE_PORT = 30001                         # Starting port for subagents
PORT_RANGE = 30001-30010                  # Each subagent gets dedicated port
</TestConfiguration>

## MAIN WORKFLOW

<ExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Execute the <InitialSetup/>
    **STEP 2:** Execute the <BatchRenumbering/>
    **STEP 3:** Execute the <CleanupPreviousRuns/>
    **STEP 4:** Execute the <ApplicationLaunch/>
    **STEP 5:** Execute the <ApplicationVerification/>
    **STEP 6:** Execute the <BatchProcessingLoop/>
    **STEP 7:** Execute the <FinalCleanup/>
</ExecutionFlow>

## STEP 1: INITIAL SETUP

<InitialSetup>
    **Get actual temp directory path for all file operations:**

    ```bash
    echo $TMPDIR
    ```

    Store this expanded path (e.g., `/var/folders/rf/.../T/`) for use in all Write tool operations.

    **CRITICAL**: The Write tool does NOT expand environment variables. Always use the actual path.
</InitialSetup>

## STEP 2: BATCH RENUMBERING

<BatchRenumbering>
    **Clear and reassign batch numbers for untested/failed types:**

    ```bash
    ./.claude/commands/scripts/mutation_test_renumber_batches_dict.sh [BATCH_SIZE]
    ```

    This script will:
    - Clear all existing batch numbers
    - Assign new batch numbers to untested/failed types
    - Display statistics: total types, passed, failed, untested, batches to process

    **STOP CONDITION**: If no untested types remain, stop execution and report completion.
</BatchRenumbering>

## STEP 3: CLEANUP PREVIOUS RUNS

<CleanupPreviousRuns>
    **Remove leftover batch result files to prevent interference:**

    ```bash
    rm -f [TEMP_DIR]/batch_results_*.json
    ```

    Use the actual expanded temp directory path from <InitialSetup/>.
</CleanupPreviousRuns>

## STEP 4: APPLICATION LAUNCH

<ApplicationLaunch>
    **Launch 10 extras_plugin instances on sequential ports starting at 30001:**

    1. **Shutdown any existing apps** (clean slate):
    ```python
    # Execute in parallel for ports 30001-30010:
    mcp__brp__brp_shutdown(app_name="extras_plugin", port=PORT)
    ```

    2. **Launch all 10 apps with a single command**:
    ```python
    mcp__brp__brp_launch_bevy_example(
        example_name="extras_plugin",
        port=30001,
        instance_count=10
    )
    ```

    This will launch 10 instances on ports 30001-30010 automatically.
</ApplicationLaunch>

## STEP 5: APPLICATION VERIFICATION

<ApplicationVerification>
    **Verify BRP connectivity on all ports:**

    ```python
    # Execute in parallel for ports 30001-30010:
    mcp__brp__brp_status(app_name="extras_plugin", port=PORT)
    ```

    **STOP CONDITION**: If any app fails to respond, stop and report error.
</ApplicationVerification>

## STEP 6: BATCH PROCESSING LOOP

<BatchProcessingLoop>
    **Process each batch sequentially with parallel subagents:**

    For each batch N (starting from 1):

    1. Execute <GetBatchAssignments/> for batch N
    2. Execute <SetWindowTitles/> based on assignments
    3. Execute <LaunchSubagents/> with parallel Task invocations
    4. Execute <ProcessBatchResults/> after all subagents complete
    5. Execute <CheckForFailures/> and stop if any failures detected

    Continue until all batches are processed or a failure occurs.
</BatchProcessingLoop>

### BATCH PROCESSING SUBSTEPS

<GetBatchAssignments>
    **Retrieve batch assignments (assignment indices only) for current batch:**

    ```bash
    python3 ./.claude/commands/scripts/mutation_test_get_batch_assignments.py [BATCH_NUMBER]
    ```

    Returns JSON with:
    - batch_number
    - assignments: Array with subagent, port, and assignment_index (NO type names to prevent substitution)

    **Store this output in a variable for systematic processing.**
</GetBatchAssignments>

<SetWindowTitles>
    **Set window titles for visual tracking:**

    **EXACT PROCEDURE**:
    1. Get the assignments from GetBatchAssignments
    2. For each assignment:
       - Port = assignment.port
       - Title = f"Subagent {assignment.subagent} (Index {assignment.assignment_index})"

    Send all window title updates in parallel.
</SetWindowTitles>

<LaunchSubagents>
    **Launch parallel subagents for batch testing:**

    **EXACT PROCEDURE**:
    1. Use the assignments from GetBatchAssignments stored earlier
    2. Create exactly assignments.length Task invocations
    3. For each assignment:
       - Subagent number = assignment.subagent
       - Port = assignment.port
       - Batch number = batch_number (from assignments JSON)
       - Assignment index = assignment.assignment_index

    **Example for a batch with 3 assignments**:
    ```
    Assignment 1: subagent 1, port 30001, batch 5, assignment_index 0
    Assignment 2: subagent 2, port 30002, batch 5, assignment_index 1
    Assignment 3: subagent 3, port 30003, batch 5, assignment_index 2
    ```

    **VALIDATION BEFORE LAUNCHING**:
    - Verify assignments.length <= 10 (max subagents available)
    - Each Task prompt must include ONLY batch number and assignment index
    - NO TYPE NAMES in prompts to prevent agent substitution
    - Subagents will fetch their exact assigned types using the index

    Send ALL Tasks in ONE message for parallel execution.
</LaunchSubagents>

<ProcessBatchResults>
    **Collect and merge batch results:**

    1. **Collect all subagent results** into single JSON array

    2. **Write results to temp file** using Write tool:
    ```python
    Write(
        file_path="[TEMP_DIR]/batch_results_[BATCH_NUMBER].json",
        content=[collected_results_json]
    )
    ```

    3. **Execute merge script**:
    ```bash
    ./.claude/commands/scripts/mutation_test_merge_batch_results.sh \
        [TEMP_DIR]/batch_results_[BATCH_NUMBER].json \
        [TEMP_DIR]/all_types.json
    ```

    4. **Cleanup temp file**:
    ```bash
    rm -f [TEMP_DIR]/batch_results_[BATCH_NUMBER].json
    ```
</ProcessBatchResults>

<CheckForFailures>
    **Check merge script exit code and results:**

    - Exit code 0: All passed, continue to next batch
    - Exit code 2: **FAILURES DETECTED - STOP IMMEDIATELY**
    - COMPONENT_NOT_FOUND status: **STOP IMMEDIATELY**

    **FAILURE PROTOCOL**:
    1. Save progress for passed types
    2. **Display detailed failure/missing component information**
    3. Report failure details to user
    4. Execute <FinalCleanup/>
    5. **DO NOT CONTINUE** to next batch

    **REQUIRED: Display Failure Details**
    When COMPONENT_NOT_FOUND errors are detected, display:
    ```
    **Missing Components Details:**
    - [type_name]: [fail_reason]
      - Query Filter: [filter]
      - Query Data: [data]
    ```

    When FAIL errors are detected, display:
    ```
    **Failed Types Details:**
    - [type_name]: [fail_reason]
      - Failed Mutation Path: [failed_mutation_path]
    ```

    Extract this information from the batch results JSON before cleanup.
</CheckForFailures>

## STEP 7: FINAL CLEANUP

<FinalCleanup>
    **Shutdown all applications:**

    ```python
    # Execute in parallel for ports 30001-30010:
    mcp__brp__brp_shutdown(app_name="extras_plugin", port=PORT)
    ```
</FinalCleanup>

## SUBAGENT PROMPT TEMPLATE

<SubagentPrompt>
**CRITICAL RESPONSE LIMIT**: Return ONLY the JSON array result. NO explanations, NO commentary, NO test steps, NO summaries.

You are subagent [Y] assigned to port [30000+Y].

**YOUR ASSIGNED PORT**: [30000+Y]
**YOUR BATCH**: [BATCH_NUMBER]
**YOUR ASSIGNMENT INDEX**: [ASSIGNMENT_INDEX]

**DO NOT**:
- Launch any apps (use EXISTING app on your port)
- Update JSON files
- Provide explanations or commentary
- Test any type other than those returned by the index script
- Make up or substitute different types
- Use your Bevy knowledge to "fix" or "improve" type names
- Test related types (like bundles when given components)

**CRITICAL CONSTRAINT**: You MUST test ONLY the exact types returned by the index script. NEVER substitute type names even if you think they are wrong. The test system controls type names completely.

**First Step - Get Your Assigned Types**:
```bash
python3 ./.claude/commands/scripts/mutation_test_get_types_by_index.py [BATCH_NUMBER] [ASSIGNMENT_INDEX]
```
This returns the exact type names you must test. Use these EXACTLY as returned.

**Second Step - Get Mutation Paths for Your Types**:
Take the type array from step 1 and get mutation paths:
```bash
echo '[TYPE_ARRAY_FROM_STEP_1]' | python3 ./.claude/commands/scripts/mutation_test_get_type_guides.py
```
This returns complete type guides with mutation paths for your assigned types.

**Testing Protocol**:
1. Call step 1 script to get your assigned type names
2. Call step 2 script to get mutation paths for those types
3. For each type in the returned guides:
   a. Skip spawn/insert if spawn_format is null
   b. Test spawn/insert if spawn_format exists
   c. Query for entities with component using EXACT syntax:
   ```json
   {
     "filter": {"with": ["EXACT_TYPE_NAME_FROM_GUIDE"]},
     "data": {"components": []}
   }
   ```
   CRITICAL: Use the exact `type_name` field from the guide - NEVER modify or abbreviate it
   d. Test ALL mutable mutation paths
4. Return ONLY JSON result array for ALL tested types
5. NEVER test types not returned by the index script

**JSON Number Rules**:
- ALL primitives (u8, u16, u32, f32, etc.) MUST be JSON numbers
- Even large numbers like 18446744073709551615 are JSON numbers
- NEVER use strings for numbers: ✗ "42" → ✓ 42

**Return EXACTLY this format (nothing else)**:
```json
[{
  "type": "[full::qualified::type::name]",
  "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
  "fail_reason": "[error or empty]",
  "failed_mutation_path": "[mutation path that failed, only for FAIL status]",
  "query_parameters": {
    "filter": "[query filter used, only for COMPONENT_NOT_FOUND status]",
    "data": "[query data requested, only for COMPONENT_NOT_FOUND status]"
  }
}]
```

**FINAL INSTRUCTION**: Output ONLY the JSON array above. Nothing before. Nothing after.
</SubagentPrompt>

## CRITICAL RULES AND CONSTRAINTS

<CoreRules>
**Execution Rules**:
1. ALWAYS reassign batch numbers before each run
2. ALWAYS use parallel subagents (10 at once)
3. Main agent orchestrates, subagents test
4. STOP ON ANY FAILURE - no exceptions
5. Simple pass/fail per type

**Failure Handling**:
- ANY failure = IMMEDIATE STOP
- Save progress for passed types
- Report failure details
- DO NOT continue to next batch
</CoreRules>

<NumberHandling>
**JSON Number Requirements**:
- ALL numeric primitives MUST be JSON numbers
- This includes: u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64
- Large numbers like 18446744073709551615 are STILL JSON numbers
- "invalid type: string" = serialization error, fix and retry
</NumberHandling>

<PathHandling>
**File Path Requirements**:
- NEVER use $TMPDIR in Write tool paths
- ALWAYS use expanded path from InitialSetup
- Example: `/var/folders/rf/.../T/` not `$TMPDIR`
</PathHandling>

<ParallelExecution>
**Parallel Execution Requirements**:
- ALL app launches in ONE message
- ALL app verifications in ONE message
- ALL subagent Tasks in ONE message per batch
- NEVER send tools one at a time
</ParallelExecution>

## RESULT SCHEMAS

<ResultFormat>
**Subagent Result Schema**:
```json
{
  "type": "string (full type name)",
  "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
  "fail_reason": "string (error message or empty)",
  "failed_mutation_path": "string (mutation path that failed, only for FAIL status)",
  "query_parameters": {
    "filter": "string (query filter used, only for COMPONENT_NOT_FOUND status)",
    "data": "string (query data requested, only for COMPONENT_NOT_FOUND status)"
  }
}
```

**Progress File Schema** (`all_types.json`):
```json
{
  "type_guide": [{
    "type_name": "string",
    "spawn_format": "object|null",
    "mutation_paths": "object",
    "test_status": "untested|passed|failed",
    "batch_number": "number|null",
    "fail_reason": "string"
  }]
}
```
</ResultFormat>

## COMPLETION CRITERIA

<CompletionCriteria>
**Success**: ALL types in ALL batches pass their tests
**Failure**: ANY single type fails = IMMEDIATE STOP
**Resume**: Can be resumed after fixing issues by re-running command

**REQUIRED FAILURE REPORTING**:
- Show counts: PASS, FAIL, COMPONENT_NOT_FOUND
- List each missing component with query parameters used
- List each failed type with the specific mutation path that failed and the reason/error that it failed with
- Include filter and data parameters for debugging query issues
</CompletionCriteria>
