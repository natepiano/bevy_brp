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
TYPES_PER_SUBAGENT = 2                                  # Types each subagent tests
MAX_SUBAGENTS = 10                                      # Parallel subagents per batch
BATCH_SIZE = [TYPES_PER_SUBAGENT] * [MAX_SUBAGENTS]     # Types per batch (MAX_SUBAGENTS * TYPES_PER_SUBAGENT)
BASE_PORT = 30001                                       # Starting port for subagents
PORT_RANGE = 30001-30010                                # Each subagent gets dedicated port
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
    **STEP 7:** Execute the <FinalCleanup/> (SILENTLY if failures detected)
    **STEP 8:** Execute the <InteractiveFailureReview/> (ONLY if failures detected)
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
    **Retrieve subagent assignments for current batch:**

    ```bash
    python3 ./.claude/scripts/mutation_test_get_subagent_assignments.py \
        --batch [BATCH_NUMBER] \
        --max-subagents [MAX_SUBAGENTS] \
        --types-per-subagent [TYPES_PER_SUBAGENT]
    ```

    Returns JSON with:
    - batch_number, max_subagents, types_per_subagent, total_types
    - assignments: Array with subagent, port, and types (complete type data including spawn_format and mutation_paths)

    **CRITICAL VALIDATION**:
    1. **STOP IF** assignments array length != MAX_SUBAGENTS
       - ERROR: "Expected exactly {MAX_SUBAGENTS} assignments, got {actual_count}"
    2. **STOP IF** any port is outside range BASE_PORT through (BASE_PORT + MAX_SUBAGENTS - 1)
       - ERROR: "Invalid port {port} - must be in range {BASE_PORT}-{BASE_PORT + MAX_SUBAGENTS - 1}"
    3. **STOP IF** any assignment doesn't have exactly TYPES_PER_SUBAGENT types
       - ERROR: "Assignment {subagent} has {actual_count} types, expected {TYPES_PER_SUBAGENT}"

    **Store this output in a variable for systematic processing.**
</GetBatchAssignments>

<SetWindowTitles>
    **Set window titles for visual tracking:**

    **EXACT PROCEDURE**:
    1. Get the assignments from GetBatchAssignments (returns exactly MAX_SUBAGENTS assignments - one per subagent)
    2. For each of the MAX_SUBAGENTS subagent assignments:
       - Port = assignment.port
       - Types = assignment.types (already contains complete type data)
       - **VALIDATE** types array length == TYPES_PER_SUBAGENT
         - **STOP IF** wrong count: "Assignment {subagent} has {actual} types, expected {TYPES_PER_SUBAGENT}"
       - Title = Create comma-separated list of last segments after `::` from all type names

    **DEFENSIVE VALIDATION**:
    - Each assignment MUST contain exactly TYPES_PER_SUBAGENT types
    - FAIL FAST if any assignment has wrong number of types

    Send all window title updates in parallel.
</SetWindowTitles>

<LaunchSubagents>
    **Launch parallel subagents for batch testing:**

    **EXACT PROCEDURE**:
    1. Use the assignments from GetBatchAssignments to determine type names and counts
    2. Create exactly MAX_SUBAGENTS Task invocations - one per subagent
    3. Each subagent will fetch their own complete type data
    4. For each subagent (index 0 through MAX_SUBAGENTS-1):
       - Subagent index = loop index (0-based)
       - Port = BASE_PORT + index (where BASE_PORT = 30001)
       - Task description = "Test [TYPE_NAMES]" where TYPE_NAMES is comma-separated list of last segments after "::" from assignment data
       - Provide minimal information in prompt:
         * Batch number
         * Subagent index (0-based)
         * Port number
         * Max subagents
         * Types per subagent

    **DEFENSIVE VALIDATION**:
    - Main agent verifies assignment count before launching subagents
    - Subagents fetch their own data to prevent prompt corruption
    - Always exactly MAX_SUBAGENTS subagent assignments (one per available port)
    - Task prompts contain ONLY identification info, not type data
    - Task description should include type names for tracking
    - Subagents retrieve their exact assigned types directly from the script

    **CRITICAL TYPE ASSIGNMENT VALIDATION**:
    The main agent provides ONLY identification information to subagents.
    Subagents are responsible for:
    1. **FETCHING** their assignments directly from the script
    2. **VALIDATING** the fetched data contains expected number of types
    3. **USING** the exact type data as fetched - NO MODIFICATIONS
    4. **REMEMBER**: This prevents corruption during prompt construction

    Example validation:
    ```
    Assignment script says: "bevy_core_pipeline::tonemapping::ColorGrading"
    ✅ CORRECT: Use exactly "bevy_core_pipeline::tonemapping::ColorGrading"
    ❌ WRONG: Change to "bevy_render::view::ColorGrading" because you "know better"
    ```

    **ENFORCEMENT**: If you detect yourself trying to modify type names, STOP and report the validation failure.

    **Example for MAX_SUBAGENTS=3, TYPES_PER_SUBAGENT=1**:
    ```
    Subagent index 0: port BASE_PORT, batch 5, description "Test Bloom"
    Subagent index 1: port BASE_PORT+1, batch 5, description "Test Camera3d"
    Subagent index 2: port BASE_PORT+2, batch 5, description "Test Skybox"
    ```

    **Example for MAX_SUBAGENTS=3, TYPES_PER_SUBAGENT=2**:
    ```
    Subagent index 0: port BASE_PORT, batch 5, description "Test Bloom, BloomSettings"
    Subagent index 1: port BASE_PORT+1, batch 5, description "Test Camera3d, Camera2d"
    Subagent index 2: port BASE_PORT+2, batch 5, description "Test Skybox, Tonemapping"
    ```

    Send ALL Tasks in ONE message for parallel execution.
</LaunchSubagents>

<ProcessBatchResults>
    **Collect and merge batch results:**

    1. **Collect all subagent results** into single JSON array

    2. **CRITICAL VALIDATION** of collected results:
       - **STOP IF** number of subagent results != MAX_SUBAGENTS
         - ERROR: "Expected {MAX_SUBAGENTS} subagent results, got {actual_count}"
       - **STOP IF** total number of type results != BATCH_SIZE
         - ERROR: "Expected {BATCH_SIZE} total type results, got {actual_count}"
       - Each subagent result should contain exactly TYPES_PER_SUBAGENT type results
         - **STOP IF** any subagent has wrong count: "Subagent {N} returned {actual} type results, expected {TYPES_PER_SUBAGENT}"

    3. **Write results to temp file** using Write tool:
    ```python
    Write(
        file_path="[TEMP_DIR]/batch_results_[BATCH_NUMBER].json",
        content=[collected_results_json]
    )
    ```

    4. **Execute merge script**:
    ```bash
    ./.claude/scripts/mutation_test_merge_batch_results.sh \
        [TEMP_DIR]/batch_results_[BATCH_NUMBER].json \
        [TEMP_DIR]/all_types.json
    ```

    5. **Cleanup temp file**:
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
    1. **Store failure details** in a variable from batch results JSON
    2. Save progress for passed types (merge script handles this)
    3. Execute <FinalCleanup/> SILENTLY - no output during cleanup
    4. Execute <InteractiveFailureReview/> to present failures one by one
    5. **DO NOT CONTINUE** to next batch

    **CRITICAL**: Do NOT display failure details during this step. Store them for the interactive review after cleanup.
</CheckForFailures>

## STEP 7: FINAL CLEANUP

<FinalCleanup>
    **Shutdown all applications SILENTLY (no output):**

    ```python
    # Execute in parallel for ports 30001-30010:
    mcp__brp__brp_shutdown(app_name="extras_plugin", port=PORT)
    ```

    **CRITICAL**: Do NOT display shutdown status messages. Execute silently.
</FinalCleanup>

## STEP 8: INTERACTIVE FAILURE REVIEW (Only if failures detected)

<InteractiveFailureReview>
    **After cleanup is complete, present failures interactively:**

    1. **Display Summary First**:
    ```
    ## MUTATION TEST EXECUTION COMPLETE

    **Status**: STOPPED DUE TO FAILURES
    **Progress**: Batch [N] of [TOTAL] processed
    **Results**: [PASS_COUNT] PASSED, [FAIL_COUNT] FAILED, [MISSING_COUNT] MISSING COMPONENTS

    **Detailed failure log saved to**: [PATH]
    ```

    2. **Present Each Failure One by One**:

    For each failure, present it with this format:

    ```
    ## FAILURE [X] of [TOTAL]: `[type_name]`

    ### Overview
    - **Entity ID**: [entity_id] (successfully created and queried)
    - **Total Mutations**: [total] attempted
    - **Mutations Passed**: [count] succeeded
    - **Failed At**: [operation type or mutation path]

    ### What Succeeded Before Failure
    [List each successful operation with ✅]

    ### The Failure

    **Failed [Operation/Path]**: [specific failure point]

    **What We Sent**:
    ```json
    [formatted request]
    ```

    **Error Response**:
    ```json
    [formatted response]
    ```

    ### Analysis
    [Brief analysis of what the error means]

    ---

    **Would you like to**:
    - **Investigate** this specific failure in detail
    - **Known Issue** - mark as known and continue to next
    - **Skip** this failure and continue to the next
    - **Stop** reviewing failures and exit

    What would you prefer?
    ```

    3. **Wait for User Response** after each failure presentation

    4. **Handle User Choice**:
    - **Investigate**: Launch Task tool with specific investigation prompt
    - **Known Issue**: Add to `.claude/mutation_test_known_issues.json` with full details and continue
    - **Skip**: Continue to next failure without marking as known
    - **Stop**: Exit failure review

    **CRITICAL**: Present failures ONE AT A TIME and wait for user input between each one.
</InteractiveFailureReview>

## KEYWORD HANDLING

<KeywordHandling>
**User Response Processing**:

When presenting failures, ALWAYS use this exact format for the options:

```
**Would you like to**:
- **Investigate** this specific failure in detail
- **Known Issue** - mark as known and continue to next
- **Skip** this failure and continue to the next
- **Stop** reviewing failures and exit

What would you prefer?
```

**Keyword Actions**:
- **Investigate**: Launch detailed investigation Task for the current failure
- **Known Issue**:
  1. Add type/mutation path pair to `.claude/mutation_test_known_issues.json`
  2. Include the failure reason and error details
  3. Continue to the next failure
- **Skip**: Continue to the next failure without recording (temporary skip)
- **Stop**: Exit the failure review process immediately

**Known Issues Tracking**:
When user selects **Known Issue**, add to `.claude/mutation_test_known_issues.json`:
```json
{
  "type": "fully::qualified::type::name",
  "path": ".mutation.path",
  "issue": "Brief description of the problem"
}
```

**Future Test Behavior**:
- Check `.claude/mutation_test_known_issues.json` before presenting failures
- Automatically skip known issues without presenting them
- Summary should note: "X known issues skipped (see `.claude/mutation_test_known_issues.json`)"
- Known issues are persistent across test runs
</KeywordHandling>

## SUBAGENT PROMPT TEMPLATE

<SubagentPrompt>
**CRITICAL RESPONSE LIMIT**: Return ONLY the JSON array result. NO explanations, NO commentary, NO test steps, NO summaries.

You are subagent with index [INDEX] (0-based) assigned to port [PORT].

**YOUR ASSIGNED PORT**: [PORT]
**YOUR BATCH**: [BATCH_NUMBER]
**YOUR SUBAGENT INDEX**: [INDEX] (0-based)
**MAX SUBAGENTS**: [MAX_SUBAGENTS]
**TYPES PER SUBAGENT**: [TYPES_PER_SUBAGENT]

**DO NOT**:
- Launch any apps (use EXISTING app on your port)
- Update JSON files
- Provide explanations or commentary
- Test any type other than those provided in your assignment data
- Make up or substitute different types
- Use your Bevy knowledge to "fix" or "improve" type names
- Test related types (like bundles when given components)
- MODIFY TYPE NAMES IN ANY WAY - use the exact strings provided

**CRITICAL CONSTRAINT**: You MUST test ONLY the exact types provided in your assignment data. NEVER substitute type names even if you think they are wrong. The test system controls type names completely.

**TYPE NAME VALIDATION**: Before testing each type, validate that you are using the EXACT type_name string from your assignment data:
- ✅ CORRECT: Use exactly the type_name provided in assignment data
- ❌ WRONG: Change type names based on your Bevy knowledge
- ❌ WRONG: "Fix" type paths that you think are incorrect
- **FAIL IMMEDIATELY** if you detect yourself modifying any type name

**Fetching Your Assignment Data**:
You MUST fetch your own assignment data as your FIRST action:
```bash
python3 ./.claude/scripts/mutation_test_get_subagent_assignments.py \
    --batch [BATCH_NUMBER] \
    --max-subagents [MAX_SUBAGENTS] \
    --types-per-subagent [TYPES_PER_SUBAGENT] \
    --subagent-index [YOUR_INDEX]
```

This returns your specific assignment with complete type data.

**Testing Protocol**:
1. FIRST: Fetch your assignment using the script with --subagent-index parameter
2. VALIDATE: Ensure you received exactly [TYPES_PER_SUBAGENT] types
3. For each type in your fetched assignment:
   a. **SPAWN/INSERT TESTING**:
      - **CHECK FIRST**: If `spawn_format` is `null` OR `supported_operations` does NOT include "spawn" or "insert", SKIP spawn/insert testing entirely
      - **ONLY IF** spawn_format exists AND supported_operations includes "spawn"/"insert": attempt spawn/insert operations
      - **NEVER** attempt spawn/insert on types that don't support it - this will cause massive error responses
   b. **ENTITY QUERY**: Query for entities with component using EXACT syntax:
   ```json
   {
     "filter": {"with": ["EXACT_TYPE_NAME_FROM_GUIDE"]},
     "data": {"components": []}
   }
   ```
   CRITICAL: Use the exact `type_name` field from the guide - NEVER modify or abbreviate it
   c. **ENTITY ID SUBSTITUTION FOR MUTATIONS**:
      - **CRITICAL**: If any mutation example contains the value `8589934670`, this is a PLACEHOLDER Entity ID
      - **YOU MUST**: Replace ALL instances of `8589934670` with REAL entity IDs from the running app
      - **HOW TO GET REAL ENTITY IDs**:
        1. First query for existing entities: `bevy_query` with appropriate filter
        2. Use the entity IDs from query results
        3. If testing EntityHashMap types, use the queried entity ID as the map key
      - **EXAMPLE**: If mutation example shows `{"8589934670": [...]}` for an EntityHashMap:
        - Query for an entity with the component first
        - Replace `8589934670` with the actual entity ID from the query
        - Then perform the mutation with the real entity ID
   d. **MUTATION TESTING**: Test ALL mutable mutation paths from the mutation_paths object
      - Apply Entity ID substitution BEFORE sending any mutation request
      - If a mutation uses Entity IDs and you don't have real ones, query for them first
3. **CAPTURE ALL ERROR DETAILS**: When ANY operation fails, record the COMPLETE request and response
4. Return ONLY JSON result array for ALL tested types
5. NEVER test types not provided in your assignment data

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
