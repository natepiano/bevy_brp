# Type Guide Comprehensive Validation Test

**CRITICAL**: Read and execute the tagged sections below in the specified order using the <ExecutionFlow/> workflow.

<ExecutionFlow/>

<TestContext>
[COMMAND]: `/mutation_test`
[PURPOSE]: Systematically validate ALL BRP component types by testing spawn/insert and mutation operations
[PROGRESS_FILE]: `.claude/transient/all_types.json` - Complete type guides with test status tracking
[ARCHITECTURE]: Main agent orchestrates, subagents test in parallel
</TestContext>

<TestConfiguration>
TYPES_PER_SUBAGENT = 1                                  # Types each subagent tests
MAX_SUBAGENTS = 10                                      # Parallel subagents per batch
BATCH_SIZE = ${TYPES_PER_SUBAGENT * MAX_SUBAGENTS}      # Types per batch
BASE_PORT = 30001                                       # Starting port for subagents
MAX_PORT = ${BASE_PORT + MAX_SUBAGENTS - 1}             # Last port in range
PORT_RANGE = ${BASE_PORT}-${MAX_PORT}                   # Port range for subagents
</TestConfiguration>

<NoOptimizationAllowed>
**CRITICAL CONSTRAINTS**:
- Use commands EXACTLY as specified - no modifications
- Execute each batch independently with full procedures
- Work with command output directly even if truncated
- No custom scripts, intermediate files, or automation shortcuts
- No combining or skipping steps regardless of previous success
</NoOptimizationAllowed>

## MAIN WORKFLOW

<ExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 0:** Execute the <NoOptimizationAllowed/> - Read and internalize
    **STEP 1:** Execute the <InitialSetup/>
    **STEP 2:** Execute the <BatchRenumbering/>
    **STEP 3:** Execute the <CleanupPreviousRuns/>
    **STEP 4:** Execute the <ApplicationLaunch/>
    **STEP 5:** Execute the <ApplicationVerification/>
    **STEP 6:** Execute the <BatchProcessingLoop/>
    **STEP 7:** Execute the <FinalCleanup/> (SILENTLY if failures detected)
    **STEP 8:** Execute the <InteractiveFailureReview/> (ONLY if NEW failures detected)
</ExecutionFlow>

## STEP 1: INITIAL SETUP

<InitialSetup>
    **Ensure types directory exists for all file operations:**

    ```bash
    mkdir -p .claude/transient
    echo "Using .claude/transient/ for persistent storage"
    ```

    All mutation test files will be stored in `.claude/transient/` for persistence across reboots.

    **CRITICAL**: Use `.claude/transient/` prefix for all file paths in Write tool operations.
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

## VALIDATION SECTIONS

<ValidationErrorFormat>
    **Standard validation error format:**
    ```
    ERROR: [Brief description]
    Expected: [What should have happened]
    Actual: [What was found instead]
    ```

    **Apply this format consistently across all validation errors.**
</ValidationErrorFormat>

<ValidateAssignmentsStructure>
    **Core assignments array validation:**
    - **STOP IF** JSON parsing fails:
      - ERROR: Cannot parse GetBatchAssignments JSON output
    - **STOP IF** assignments array missing:
      - ERROR: No 'assignments' array found in JSON
    - **STOP IF** assignments.length < 1 or > ${MAX_SUBAGENTS}:
      - ERROR: Invalid assignment count
      - Expected: 1-${MAX_SUBAGENTS} assignments
      - Actual: {actual_count} assignments
</ValidateAssignmentsStructure>

<ValidateAssignmentFields>
    **Individual assignment field validation:**
    - **FOR** each assignment in assignments array:
        - **STOP IF** port missing:
          - ERROR: Assignment {index} missing port field
        - **STOP IF** types missing:
          - ERROR: Assignment {index} missing types array
        - **STOP IF** types.length < 1 or > ${TYPES_PER_SUBAGENT}:
          - ERROR: Invalid types count for assignment {index}
          - Expected: 1-${TYPES_PER_SUBAGENT} types
          - Actual: {actual} types
        - **STOP IF** port outside range ${BASE_PORT}-${MAX_PORT}:
          - ERROR: Invalid port for assignment {index}
          - Expected: Port in range ${BASE_PORT}-${MAX_PORT}
          - Actual: Port {port}
</ValidateAssignmentFields>

## STEP 3: CLEANUP PREVIOUS RUNS

<CleanupPreviousRuns>
    **Remove leftover files from previous runs to prevent interference:**

    ```bash
    rm -f .claude/transient/batch_results_*.json
    rm -f .claude/transient/all_types_failures_*.json
    ```

    Clean up:
    - Batch result files from previous runs
    - Old failure log files to prevent confusion with new failures
</CleanupPreviousRuns>

## STEP 4: APPLICATION LAUNCH

<ApplicationLaunch>
    **Launch ${MAX_SUBAGENTS} extras_plugin instances on sequential ports starting at ${BASE_PORT}:**

    1. **Shutdown any existing apps** (clean slate):
    Execute <ParallelPortOperation/> with:
    - Operation: mcp__brp__brp_shutdown
    - Parameters: app_name="extras_plugin"

    2. **Launch all ${MAX_SUBAGENTS} apps with a single command**:
    ```python
    mcp__brp__brp_launch_bevy_example(
        example_name="extras_plugin",
        port=${BASE_PORT},
        instance_count=${MAX_SUBAGENTS}
    )
    ```

    This will launch ${MAX_SUBAGENTS} instances on ports ${BASE_PORT}-${MAX_PORT} automatically.
</ApplicationLaunch>

## STEP 5: APPLICATION VERIFICATION

<ApplicationVerification>
    **Verify BRP connectivity on all ports:**

    Execute <ParallelPortOperation/> with:
    - Operation: mcp__brp__brp_status
    - Parameters: app_name="extras_plugin"

    **STOP CONDITION**: If any app fails to respond, stop and report error.
</ApplicationVerification>

## STEP 6: BATCH PROCESSING LOOP

<BatchProcessingLoop>
    **Process each batch sequentially with parallel subagents:**

    For each batch N (starting from 1):

    0. Re-read <NoOptimizationAllowed/> before processing this batch
    1. **REPORT PROGRESS**: Display "Processing batch N of [TOTAL_BATCHES] - Testing [TYPES_IN_BATCH] types ([REMAINING_TYPES] remaining)"
    2. Execute <GetBatchAssignments/> for batch N
    3. Execute <SetWindowTitles/> based on assignments
    4. Execute <LaunchSubagents/> with parallel Task invocations
       - MUST be exactly ${MAX_SUBAGENTS} Task invocations
       - NEVER combine or skip Task invocations
    5. Execute <ProcessBatchResults/> after all subagents complete
    6. Execute <CheckForFailures/> which will:
       - Continue to next batch if all pass OR only known issues found
       - Stop only if NEW (non-known) failures are detected

    Continue until all batches are processed or NEW failures occur.
</BatchProcessingLoop>

### BATCH PROCESSING SUBSTEPS

<GetBatchAssignments>
    **Retrieve subagent assignments for current batch:**

    **MANDATORY EXACT COMMAND - DO NOT MODIFY**:
    ```bash
    python3 ./.claude/scripts/mutation_test_get_subagent_assignments.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
    ```

    **CRITICAL INSTRUCTION**: Use the command EXACTLY as specified above. DO NOT:
    - Create intermediate files
    - Pipe output to files
    - Add custom Python processing
    - Modify the command in any way
    - Work around truncated output by creating files

    **If output appears truncated**: Work with the available data directly from the command output.

    Returns JSON with:
    - batch_number, max_subagents, types_per_subagent, total_types
    - assignments: Array with subagent, port, and types (complete type data including spawn_format and mutation_paths)

    **CRITICAL VALIDATION**:
    Execute <ValidateAssignmentsStructure/> followed by <ValidateAssignmentFields/>

    **Extract essential information directly from the command output for the next steps.**
</GetBatchAssignments>

<SetWindowTitles>
    **Set window titles for visual tracking using GetBatchAssignments JSON output:**

    **STEP 1: Parse and validate assignments data from previous command output:**
    - Locate the "assignments" array in the GetBatchAssignments JSON response
    - Execute <ValidateAssignmentsStructure/>

    **STEP 2: Validate individual assignments:**
    - Execute <ValidateAssignmentFields/>
    - Extract required fields: `assignment.port`, `assignment.types`

    **STEP 3: Create window titles and execute in parallel:**
    - For each assignment.types[].type_name, extract short name (text after last "::")
    - Join short names with commas: "ShortName1, ShortName2, ShortName3"
    - Create title: "Subagent {assignment.subagent}: {joined_short_names}"
    - Use mcp__brp__brp_extras_set_window_title tool with assignment.port and title

    **EXECUTE ALL WINDOW TITLE UPDATES IN PARALLEL:**
    ```python
    # For each assignment, execute in ONE message:
    mcp__brp__brp_extras_set_window_title(port=assignment1.port, title="Subagent 1: Type1, Type2")
    mcp__brp__brp_extras_set_window_title(port=assignment2.port, title="Subagent 2: Type3, Type4")
    # ... continue for all assignments
    ```

    **Example data transformation:**
    ```
    Input: assignment.types = [{"type_name": "bevy_pbr::light::CascadeShadowConfig"}, {"type_name": "bevy_pbr::light::AmbientLight"}]
    Output: title = "Subagent 1: CascadeShadowConfig, AmbientLight"
    Tool call: mcp__brp__brp_extras_set_window_title(port=${BASE_PORT}, title="Subagent 1: CascadeShadowConfig, AmbientLight")
    ```
</SetWindowTitles>

<LaunchSubagents>
    **Launch parallel subagents for batch testing:**

    **EXACT PROCEDURE**:
    1. Use the assignments from GetBatchAssignments to determine type names and counts
    2. Create exactly assignments.length Task invocations - one per actual assignment
    3. Each subagent will fetch their own complete type data
    4. For each subagent (index 0 through assignments.length-1):
       - Subagent index = loop index (0-based)
       - Port = ${BASE_PORT} + index
       - Task description = "Test [TYPE_NAMES] ([INDEX+1] of [ACTUAL_SUBAGENTS])" where TYPE_NAMES is comma-separated list of last segments after "::" from assignment data and INDEX is 0-based
       - Task prompt = Reference to instructions file with variable substitution

    **TASK PROMPT TEMPLATE**:
    ```
    EXECUTE the mutation test workflow defined in @.claude/instructions/mutation_test_subagent.md

    Your configuration:
    - SUBAGENT_INDEX = [index]
    - PORT = [port]
    - BATCH_NUMBER = [batch]
    - MAX_SUBAGENTS = ${MAX_SUBAGENTS}
    - TYPES_PER_SUBAGENT = ${TYPES_PER_SUBAGENT}

    CRITICAL: You MUST return ONLY the JSON array result. NO explanations, NO commentary, NO test steps.
    ```

    **DEFENSIVE VALIDATION**:
    - Main agent verifies assignment count before launching subagents
    - Subagents fetch their own data to prevent prompt corruption
    - Use actual assignment count (may be less than MAX_SUBAGENTS for partial batches)
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

    **Example for MAX_SUBAGENTS=3, TYPES_PER_SUBAGENT=1**:
    ```
    Subagent index 0: port BASE_PORT, batch 5, description "Test Bloom (1 of 3)"
    Subagent index 1: port BASE_PORT+1, batch 5, description "Test Camera3d (2 of 3)"
    Subagent index 2: port BASE_PORT+2, batch 5, description "Test Skybox (3 of 3)"
    ```

    **Example for MAX_SUBAGENTS=3, TYPES_PER_SUBAGENT=2**:
    ```
    Subagent index 0: port BASE_PORT, batch 5, description "Test Bloom, BloomSettings (1 of 3)"
    Subagent index 1: port BASE_PORT+1, batch 5, description "Test Camera3d, Camera2d (2 of 3)"
    Subagent index 2: port BASE_PORT+2, batch 5, description "Test Skybox, Tonemapping (3 of 3)"
    ```

    Send ALL Tasks in ONE message for parallel execution.
</LaunchSubagents>

<ProcessBatchResults>
    **Collect and merge batch results:**

    1. **Collect all subagent results** into single JSON array

    2. **Write results to temp file** using Write tool (save partial progress even if validation fails):
    ```python
    Write(
        file_path=".claude/transient/batch_results_[BATCH_NUMBER].json",
        content=[collected_results_json]
    )
    ```

    3. **Execute merge script** to save progress:
    ```bash
    ./.claude/scripts/mutation_test_merge_batch_results.sh \
        .claude/transient/batch_results_[BATCH_NUMBER].json \
        .claude/transient/all_types.json
    ```

    4. **Cleanup temp file**:
    ```bash
    rm -f .claude/transient/batch_results_[BATCH_NUMBER].json
    ```

    5. **CRITICAL VALIDATION** of collected results:
       - **STOP IF** number of subagent results != actual_assignments_count
         - ERROR: "Expected {actual_assignments_count} subagent results, got {actual_count}"
       - **STOP IF** total number of type results != total_types_in_batch
         - ERROR: "Expected {total_types_in_batch} total type results, got {actual_count}"
       - Each subagent result should contain 1-${TYPES_PER_SUBAGENT} type results
         - **STOP IF** any subagent has wrong count: "Subagent {N} returned {actual} type results, expected {expected_for_this_subagent}"
       - **HALLUCINATION CHECK**: For each type result, validate `type` == `tested_type`
         - **STOP IF** any mismatch found:
           - ERROR: "Subagent hallucinated type name"
           - Show: "Assigned: {type}, Tested: {tested_type}"
           - This indicates the subagent modified/invented the type name instead of using the exact assignment
</ProcessBatchResults>

<CheckForFailures>
    **Check merge script exit code and results:**

    **The merge script NOW handles known issue filtering automatically:**
    - Exit code 0: All passed OR only known issues found → **CONTINUE TO NEXT BATCH**
    - Exit code 2: **NEW FAILURES DETECTED** → Stop and review

    **MERGE SCRIPT BEHAVIOR**:
    - Automatically loads `.claude/config/mutation_test_known_issues.json` if it exists
    - Filters out known issues from the failure count
    - Returns exit code 0 if only known issues were found
    - Returns exit code 2 only if NEW (non-known) failures exist
    - Displays appropriate messages for each case

    **FAILURE PROTOCOL** (only if exit code 2):
    1. Failure details are already saved by merge script to timestamped log
    2. Execute <FinalCleanup/> SILENTLY - no output during cleanup
    3. Execute <InteractiveFailureReview/> to review NEW failures
    4. **DO NOT CONTINUE** to next batch

    **SUCCESS PROTOCOL** (exit code 0):
    - Continue directly to next batch
    - No manual filtering needed - script already handled it
</CheckForFailures>

## STEP 7: FINAL CLEANUP

<FinalCleanup>
    **Shutdown all applications SILENTLY (no output):**

    Execute <ParallelPortOperation/> with:
    - Operation: mcp__brp__brp_shutdown
    - Parameters: app_name="extras_plugin"
    - Mode: SILENT (no status messages)

    **CRITICAL**: Do NOT display shutdown status messages. Execute silently.
</FinalCleanup>

## STEP 8: INTERACTIVE FAILURE REVIEW (Only if NEW failures detected)

<InteractiveFailureReview>
    **MANDATORY: Create todos before any user interaction:**
    **CRITICAL**: All interactive commands MUST use TodoWrite. Create todos for:
    - "Display failure summary and initialize review process"
    - "Review failure [X] of [TOTAL]" (create one for each failure found)
    - "Process user response for failure [X]" (track each user decision)

    **Update todos as you progress:**
    - Mark summary todo as completed after showing the initial summary
    - Mark each failure review todo as in_progress when presenting it
    - Mark as completed after user responds with a keyword

    **After cleanup is complete, present failures interactively:**

    1. **Display Summary First**:
    ```
    ## MUTATION TEST EXECUTION COMPLETE

    - **Status**: STOPPED DUE TO FAILURES
    - **Progress**: Batch [N] of [TOTAL] processed
    - **Results**: [PASS_COUNT] PASSED, [FAIL_COUNT] FAILED, [MISSING_COUNT] MISSING COMPONENTS

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

    ## Available Actions
    - **investigate** - Investigate this specific failure in detail (DEFAULT - agent will always investigate unless told otherwise)
    - **known** - Mark as known issue and continue to next (USER DECISION ONLY - agent never suggests this)
    - **skip** - Skip this failure and continue to the next
    - **stop** - Stop reviewing failures and exit

    Please select one of the keywords above.
    ```

    3. **MANDATORY: Always investigate first**
    - Execute <InvestigateFailure/> IMMEDIATELY after presenting each failure
    - Present investigation findings to user
    - NEVER suggest marking as "known" - this is user's decision only
    - Only proceed to next failure if user explicitly selects a keyword after investigation

    4. **Wait for User Response** after investigation findings

    5. **Handle User Choice**:
    - **Investigate**: Already completed - confirm findings and wait for next keyword
    - **Known Issue**: Add to `.claude/config/mutation_test_known_issues.json` with full details and continue
    - **Skip**: Continue to next failure without marking as known
    - **Stop**: Exit failure review

    **CRITICAL**: Present failures ONE AT A TIME, investigate each one, and wait for user input between each one.
</InteractiveFailureReview>

## KEYWORD HANDLING

<KeywordHandling>
**User Response Processing**:

When presenting failures, ALWAYS use this exact format for the options:

```
## Available Actions
- **investigate** - Investigate this specific failure in detail (DEFAULT - agent will always investigate unless told otherwise)
- **known** - Mark as known issue and continue to next (USER DECISION ONLY - agent never suggests this)
- **skip** - Skip this failure and continue to the next
- **stop** - Stop reviewing failures and exit

Please select one of the keywords above.
```

**CRITICAL AGENT BEHAVIOR**:
- **ALWAYS** execute <InvestigateFailure/> first for every failure
- **NEVER** suggest marking as "known issue" in your analysis
- **ALWAYS** attempt to identify root cause and propose fixes
- "known" keyword exists ONLY for user to override investigation - not for agent to suggest

**Keyword Actions**:
- **Investigate**: Already executed automatically - present findings
- **Known Issue** (USER ONLY):
  1. Add type to `.claude/config/mutation_test_known_issues.json`
  2. Include a brief description of the issue
  3. Continue to the next failure
- **Skip**: Continue to the next failure without recording (temporary skip)
- **Stop**: Exit the failure review process immediately

**Known Issues Tracking** (USER-INITIATED ONLY):
When user selects **known**, add to `.claude/config/mutation_test_known_issues.json`:
```json
{
  "type": "fully::qualified::type::name",
  "issue": "Brief description of the problem"
}
```

**Note**: See `.claude/config/mutation_test_known_issues.json.example` for format reference.
The merge script automatically loads `.claude/config/mutation_test_known_issues.json` if it exists to filter known issues.

**Future Test Behavior**:
- Check `.claude/config/mutation_test_known_issues.json` before presenting failures
- Automatically skip known issues without presenting them
- Summary should note: "X known issues skipped (see `.claude/config/mutation_test_known_issues.json`)"
- Known issues are persistent across test runs
</KeywordHandling>

<InvestigateFailure>
**Investigate the current failure using the type guide:**

1. Run: `.claude/scripts/get_type_guide.sh <failed_type_name> --file .claude/transient/all_types.json`
2. Examine the returned type guide focusing on the failed mutation path
3. Check `path_info` for the failed path (look for `applicable_variants`, `root_example`, `mutation_status`)
4. Present findings to user with specific recommendations
5. Do NOT launch Task agents - handle investigation directly
</InvestigateFailure>

## JSON PRIMITIVE RULES

<JsonPrimitiveRules>
**CRITICAL JSON PRIMITIVE REQUIREMENTS**:
- ALL numeric values MUST be JSON numbers, NOT strings
- NEVER quote numbers: ❌ "3.1415927410125732" → ✅ 3.1415927410125732
- This includes f32, f64, u32, i32, ALL numeric types
- High-precision floats like 3.1415927410125732 are STILL JSON numbers
- ALL boolean values MUST be JSON booleans, NOT strings
- NEVER quote booleans: ❌ "true" → ✅ true, ❌ "false" → ✅ false
- Numbers: ✅ 3.14, ✅ 42, ✅ 3.1415927410125732
- Booleans: ✅ true, ✅ false
- NEVER: ❌ "3.14", ❌ "42", ❌ "true", ❌ "false"
- If you get "invalid type: string" error, you quoted a number or boolean

**COMMON MISTAKES THAT CAUSE STRING CONVERSION**:
❌ Converting example to string: `str(example)` or `f"{example}"`
❌ String interpolation in values: treating numbers as text
❌ Copy-pasting example values as strings instead of raw values
❌ Using string formatting functions on numeric values

✅ CORRECT: Use the example value DIRECTLY from the type guide without any string conversion
✅ When constructing mutation params: assign the value AS-IS from the example
✅ Keep numeric types as numbers, boolean types as booleans throughout your code

**MANDATORY PRE-SEND VERIFICATION**:
Before EVERY mutation request with a numeric or boolean value:
1. **CHECK**: Look at the value you're about to send in `params["value"]`
2. **VERIFY**: If it's a number like `42`, ensure you're sending the NUMBER 42, not the STRING "42"
3. **TEST**: In your JSON structure, it should appear as `"value": 42` NOT `"value": "42"`
4. **CONFIRM**: No quotes around numbers or booleans in the actual value field

**VERIFICATION EXAMPLES**:
- ❌ WRONG: `{"value": "42"}` - This is a STRING "42"
- ✅ CORRECT: `{"value": 42}` - This is a NUMBER 42
- ❌ WRONG: `{"value": "true"}` - This is a STRING "true"
- ✅ CORRECT: `{"value": true}` - This is a BOOLEAN true
- ❌ WRONG: `{"value": "3.14"}` - This is a STRING "3.14"
- ✅ CORRECT: `{"value": 3.14}` - This is a NUMBER 3.14

**ERROR RECOVERY PROTOCOL**:
If you receive error: `invalid type: string "X", expected [numeric/boolean type]`:
1. **RECOGNIZE**: This means you DEFINITELY sent "X" as a quoted string
2. **DO NOT** report this as a test failure - this is YOUR bug, not a BRP bug
3. **FIX IMMEDIATELY**: Retry the SAME mutation with the value as an unquoted primitive
4. **VERIFY**: Before retry, confirm your value is a number/boolean, NOT a string
5. **ONLY FAIL**: If the retry also fails with a DIFFERENT error message

**VALIDATION**: Before sending ANY mutation, verify primitives are unquoted
</JsonPrimitiveRules>

## SUBAGENT INSTRUCTIONS

Subagent instructions have been moved to `.claude/instructions/mutation_test_subagent.md` for performance optimization.

The main agent references this file when launching subagents (see <LaunchSubagents/> section).

## CRITICAL RULES AND CONSTRAINTS

<CoreRules>
**Execution Rules**:
1. ALWAYS reassign batch numbers before each run
2. ALWAYS use parallel subagents (${MAX_SUBAGENTS} at once)
3. Main agent orchestrates, subagents test
4. STOP ON ANY FAILURE - no exceptions
5. Simple pass/fail per type

**Failure Handling**:
- ANY failure = IMMEDIATE STOP
- Save progress for passed types
- Report failure details
- DO NOT continue to next batch
</CoreRules>

<PrimitiveHandling>
**JSON Primitive Requirements**: Follow <JsonPrimitiveRules/> for all JSON values.
- This includes: u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64
- Large numbers like 18446744073709551615 are STILL JSON numbers
- "invalid type: string" = primitive serialization error - you sent a quoted value
- **ERROR RECOVERY**: If you get this error, follow the ERROR RECOVERY PROTOCOL in <JsonPrimitiveRules/>
- Retry immediately with unquoted value, only report failure if retry also fails
</PrimitiveHandling>

<PathHandling>
**File Path Requirements**:
- ALWAYS use `.claude/transient/` for persistent file storage
- NEVER use $TMPDIR for mutation test files
- Example: `.claude/transient/all_types.json` not `$TMPDIR/all_types.json`
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
  "type": "string (type_name from assignment - authoritative)",
  "tested_type": "string (actual type used in queries - must match 'type')",
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
1. Saves complete failure details to `.claude/transient/all_types_failures_[timestamp].json`
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

## REUSABLE PATTERNS

<ParallelPortOperation>
    **Execute operations across all configured ports in parallel:**

    ```python
    # Execute in parallel for ports ${BASE_PORT}-${MAX_PORT}:
    [Operation](app_name=[Parameters.app_name], port=PORT)
    ```

    - **Operation**: The BRP operation to execute (e.g., mcp__brp__brp_shutdown, mcp__brp__brp_status)
    - **Parameters**: Operation-specific parameters (e.g., app_name)

    This pattern ensures consistent parallel execution across the port range.
</ParallelPortOperation>

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
