# Type Guide Comprehensive Validation Test

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
    ./.claude/scripts/mutation_test_renumber_batches.sh [BATCH_SIZE]
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
    python3 ./.claude/scripts/mutation_test_get_batch_assignments.py [BATCH_NUMBER]
    ```

    Returns JSON with:
    - batch_number
    - assignments: Array with subagent, port, assignment_index, and type_name (type_name for window titles only)

    **Store this output in a variable for systematic processing.**
</GetBatchAssignments>

<SetWindowTitles>
    **Set window titles for visual tracking:**

    **EXACT PROCEDURE**:
    1. Get the assignments from GetBatchAssignments
    2. For each assignment:
       - Port = assignment.port
       - Title = For single type: last segment of assignment.type_name after `::`
                 For multiple types: comma-separated list of last segments after `::`

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
       - Task description = "Test [TYPE_NAMES]" where TYPE_NAMES is:
         * For single type: last segment after "::" from assignment.type_name
         * For multiple types: comma-separated list of last segments after "::" from all assignment.type_names

    **Example for a batch with 3 assignments (single type per subagent)**:
    ```
    Assignment 1: subagent 1, port 30001, batch 5, assignment_index 0, description "Test Bloom"
    Assignment 2: subagent 2, port 30002, batch 5, assignment_index 1, description "Test Camera3d"
    Assignment 3: subagent 3, port 30003, batch 5, assignment_index 2, description "Test Skybox"
    ```

    **Example for a batch with 3 assignments (2 types per subagent)**:
    ```
    Assignment 1: subagent 1, port 30001, batch 5, assignment_index 0, description "Test Bloom, BloomSettings"
    Assignment 2: subagent 2, port 30002, batch 5, assignment_index 1, description "Test Camera3d, Camera2d"
    Assignment 3: subagent 3, port 30003, batch 5, assignment_index 2, description "Test Skybox, Tonemapping"
    ```

    **VALIDATION BEFORE LAUNCHING**:
    - Verify assignments.length <= 10 (max subagents available)
    - Each Task prompt must include ONLY batch number and assignment index
    - Task description should include type name for tracking, but NOT in the prompt content
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
    ./.claude/scripts/mutation_test_merge_batch_results.sh \
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
    - [type_name]:
      - Query Filter: [query_details.filter]
      - Query Data: [query_details.data]
      - Entities Found: [query_details.entities_found]
      - Error: [failure_details.error_message]
    ```

    When FAIL errors are detected, display:
    ```
    **Failed Types Details:**
    - [type_name]:
      - Failed Operation: [failure_details.failed_operation]
      - Operations Completed:
        - Spawn/Insert: [operations_completed.spawn_insert]
        - Entity Query: [operations_completed.entity_query]
        - Mutations Passed: [operations_completed.mutations_passed]
        - Total Mutations Attempted: [operations_completed.total_mutations_attempted]
      - Failure Information:
        - Failed Path: [failure_details.failed_mutation_path]
        - Error Message: [failure_details.error_message]
        - Request Sent: [failure_details.request_sent]
        - Response Received: [failure_details.response_received]
    ```

    Extract this detailed information from the batch results JSON before cleanup.
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

**Get Your Complete Assignment Data**:
```bash
python3 ./.claude/scripts/mutation_test_get_assignment_guide.py [BATCH_NUMBER] [ASSIGNMENT_INDEX]
```
This returns the exact type names AND complete mutation paths you must test. Use these EXACTLY as returned.

**Testing Protocol**:
1. Call the assignment guide script to get your complete type data
2. For each type in the returned guides:
   a. **SPAWN/INSERT TESTING**: Skip spawn/insert if spawn_format is null, otherwise test spawn/insert operations
   b. **ENTITY QUERY**: Query for entities with component using EXACT syntax:
   ```json
   {
     "filter": {"with": ["EXACT_TYPE_NAME_FROM_GUIDE"]},
     "data": {"components": []}
   }
   ```
   CRITICAL: Use the exact `type_name` field from the guide - NEVER modify or abbreviate it
   c. **MUTATION TESTING**: Test ALL mutable mutation paths from the mutation_paths object
3. **CAPTURE ALL ERROR DETAILS**: When ANY operation fails, record the COMPLETE request and response
4. Return ONLY JSON result array for ALL tested types
5. NEVER test types not returned by the assignment guide script

**JSON Number Rules**:
- ALL primitives (u8, u16, u32, f32, etc.) MUST be JSON numbers
- Even large numbers like 18446744073709551615 are JSON numbers
- NEVER use strings for numbers: ✗ "42" → ✓ 42

**Return EXACTLY this format (nothing else)**:
```json
[{
  "type": "[full::qualified::type::name]",
  "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
  "entity_id": 123,  // Entity ID if created, null otherwise
  "operations_completed": {
    "spawn_insert": true|false,  // Did spawn/insert succeed?
    "entity_query": true|false,   // Did query find entity?
    "mutations_passed": [".path1", ".path2"],  // Which mutations succeeded
    "total_mutations_attempted": 5  // How many mutation paths were tested
  },
  "failure_details": {
    // ONLY PRESENT IF status is FAIL or COMPONENT_NOT_FOUND
    "failed_operation": "spawn|insert|query|mutation",
    "failed_mutation_path": ".specific.path.that.failed",
    "error_message": "Complete error message from BRP",
    "request_sent": {
      // EXACT parameters sent that caused the failure
      "method": "bevy/mutate_component",
      "params": {
        "entity": 123,
        "component": "full::type::name",
        "path": ".failed.path",
        "value": {"the": "actual", "value": "attempted"}
      }
    },
    "response_received": {
      // COMPLETE response from BRP including error details
      "error": "Full error response",
      "code": -32000,
      "data": "any additional error data"
    }
  },
  "query_details": {
    // ONLY PRESENT IF status is COMPONENT_NOT_FOUND
    "filter": {"with": ["exact::type::used"]},
    "data": {"components": []},
    "entities_found": 0
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
  "entity_id": "number|null (entity ID if created)",
  "operations_completed": {
    "spawn_insert": "boolean",
    "entity_query": "boolean",
    "mutations_passed": "array of mutation paths that succeeded",
    "total_mutations_attempted": "number"
  },
  "failure_details": {
    "failed_operation": "spawn|insert|query|mutation",
    "failed_mutation_path": "string (specific path that failed)",
    "error_message": "string (complete error from BRP)",
    "request_sent": "object (exact parameters that caused failure)",
    "response_received": "object (complete error response)"
  },
  "query_details": {
    "filter": "object (query filter used)",
    "data": "object (query data requested)",
    "entities_found": "number"
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

## ERROR DIAGNOSTICS

<ErrorDiagnostics>
**Detailed Failure Logging**:
When failures occur, the system automatically:
1. Saves complete failure details to `$TMPDIR/all_types_failures_[timestamp].json`
2. Preserves the exact request/response that caused each failure
3. Records which operations succeeded before failure

**Interpreting Failure Details**:
- `failed_operation`: Identifies where in the test sequence the failure occurred
  - "spawn": Entity creation failed
  - "insert": Component insertion failed
  - "query": Entity query failed
  - "mutation": Mutation operation failed

- `operations_completed`: Shows test progress before failure
  - `spawn_insert`: Whether entity was successfully created
  - `entity_query`: Whether entity was found after creation
  - `mutations_passed`: List of mutation paths that worked
  - `total_mutations_attempted`: How many mutations were tested

- `failure_details`: Complete diagnostic information
  - `request_sent`: Exact BRP parameters that triggered the error
  - `response_received`: Full error response from BRP
  - Can be used to reproduce the exact failure

**Common Failure Patterns**:
1. "Framework error: Unable to extract parameters" - Usually indicates a type mismatch or serialization issue
2. "The enum accessed doesn't have an X field" - Mutation path doesn't match the actual type structure
3. "Component not found" - Type isn't registered or query syntax is incorrect

**Debugging Steps**:
1. Check the failure log file for complete details
2. Look at `request_sent` to see exact parameters
3. Review `mutations_passed` to identify working paths
4. Use `response_received` error message for specific issue
</ErrorDiagnostics>

## COMPLETION CRITERIA

<CompletionCriteria>
**Success**: ALL types in ALL batches pass their tests
**Failure**: ANY single type fails = IMMEDIATE STOP
**Resume**: Can be resumed after fixing issues by re-running command

**REQUIRED FAILURE REPORTING**:
- Show counts: PASS, FAIL, COMPONENT_NOT_FOUND
- For each failed type, display:
  - Which operations succeeded before failure
  - Exact mutation path that failed
  - Complete request parameters that caused the failure
  - Full error response from BRP
  - Number of mutations that passed vs. failed
- For missing components:
  - Exact query filter and data used
  - Number of entities found (should be 0)
- Provide enough detail that the exact failure can be reproduced
</CompletionCriteria>
