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

## MAIN WORKFLOW

<ExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Execute the <InitialSetup/>
    **STEP 2:** Execute the <CleanupPreviousRuns/>
    **STEP 3:** Execute the <ApplicationLaunch/>
    **STEP 4:** Execute the <ApplicationVerification/>
    **STEP 5:** Execute the <BatchProcessingLoop/>
    **STEP 6:** Execute the <FinalCleanup/> (SILENTLY if failures detected)
    **STEP 7:** Execute the <InteractiveFailureReview/> (ONLY if NEW failures detected)
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

## STEP 2: CLEANUP PREVIOUS RUNS

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

## STEP 3: APPLICATION LAUNCH

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

## STEP 4: APPLICATION VERIFICATION

<ApplicationVerification>
    **Verify BRP connectivity on all ports:**

    Execute <ParallelPortOperation/> with:
    - Operation: mcp__brp__brp_status
    - Parameters: app_name="extras_plugin"

    **STOP CONDITION**: If any app fails to respond, stop and report error.
</ApplicationVerification>

## STEP 5: BATCH PROCESSING LOOP

<BatchProcessingLoop>
    **Process each batch sequentially with parallel subagents:**

    For each batch N (starting from 1):

    1. **REPORT PROGRESS**: Display "Processing batch N of [TOTAL_BATCHES] - Testing [TYPES_IN_BATCH] types ([REMAINING_TYPES] remaining)"
    2. Execute <GetBatchAssignments/> for batch N
    3. Execute <SetWindowTitles/> based on assignments
    4. Execute <LaunchSubagents/> with parallel Task invocations
       - MUST be exactly ${MAX_SUBAGENTS} Task invocations
       - NEVER combine or skip Task invocations
    5. Execute <ProcessBatchResults/> after all subagents complete
    6. Execute <CheckForFailures/> which will:
       - Continue to next batch if all pass
       - Stop if any failures are detected

    Continue until all batches are processed or failures occur.
</BatchProcessingLoop>

### BATCH PROCESSING SUBSTEPS

<GetBatchAssignments>
    **Single call to prepare batch and generate assignments:**

    ```bash
    python3 ./.claude/scripts/mutation_test_prepare.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
    ```

    **This single call:**
    - **When batch == 1**: Automatically renumbers batches (resets failed to untested, assigns batch numbers)
    - **For all batches**: Generates ALL test plan files (one per subagent)
    - Returns complete assignment data with window_description, task_description, and test_plan_file paths

    **Extract from the JSON output:**
    - `assignments.length` - number of subagents to launch
    - `total_types` - number of types in the batch
    - `assignments[i].window_description` - pre-formatted window title
    - `assignments[i].task_description` - pre-formatted task description
    - `assignments[i].test_plan_file` - path to pass to subagent
    - `assignments[i].port` - port number for this subagent

    **Store the entire assignments array for use in SetWindowTitles and LaunchSubagents.**

    **NOTE**: Batch renumbering statistics will be displayed only when batch == 1.
</GetBatchAssignments>

<SetWindowTitles>
    **Set window titles using the cached assignments array from GetBatchAssignments:**

    **For each assignment in assignments array:**

    1. **Extract pre-formatted data from assignment**:
       - `window_description` - pre-formatted window title
       - `port` - port number

    2. **Set window title**:
       Use the pre-formatted window_description directly

    **EXECUTE ALL WINDOW TITLE UPDATES IN PARALLEL:**
    Make all mcp__brp__brp_extras_set_window_title calls in parallel.

    **Example**:
    ```
    For assignment:
      {
        "port": 30001,
        "window_description": "Subagent 1: CascadeShadowConfig (C), AmbientLight (R)"
      }
      Tool call: mcp__brp__brp_extras_set_window_title(
        port=30001,
        title="Subagent 1: CascadeShadowConfig (C), AmbientLight (R)"
      )
    ```
</SetWindowTitles>

<LaunchSubagents>
    **Launch parallel subagents using cached assignments array from GetBatchAssignments:**

    **STEP 1: Open test plan files in Zed**
    - For each assignment in assignments array:
      - Execute: `/Applications/Zed.app/Contents/MacOS/cli [test_plan_file]`
    - Execute all Zed CLI commands in parallel

    **STEP 2: Launch all subagents in parallel**

    **For each assignment in assignments array:**

    1. **Extract pre-formatted data from assignment**:
       - `task_description` - pre-formatted task description
       - `test_plan_file` - path to test plan file
       - `port` - port number

    2. **Create Task** with:
       - description: Use `task_description` directly from assignment
       - subagent_type: "general-purpose"
       - prompt: See template below

    **TASK PROMPT TEMPLATE**:
    ```
    EXECUTE the mutation test workflow defined in @.claude/instructions/mutation_test_subagent.md

    Your configuration:
    - TEST_PLAN_FILE = [test_plan_file from assignment]
    - PORT = [port from assignment]

    The test plan file contains ALL the operations you need to execute. Read it and execute each operation in sequence.

    CRITICAL: After completing all operations, just finish. The test plan file will contain all results. Do not return any JSON output.
    ```

    **Send ALL Tasks in ONE message for parallel execution.**
</LaunchSubagents>

<ProcessBatchResults>
    **Process test results from subagent test plans:**

    Execute result processing script:
    ```bash
    python3 ./.claude/scripts/mutation_test_process_results.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
    ```

    The script automatically:
    1. Reads test plan files from $TMPDIR (mutation_test_subagent_[PORT]_plan.json)
    2. Converts test operations to result format
    3. Merges results into `.claude/transient/all_types.json`
    4. Reports statistics (passed/failed/missing)
    5. Saves failure details to timestamped log file if failures exist
    6. Cleans up temporary batch results file

    **Exit codes**:
    - 0: Success (all passed)
    - 2: Failures detected (stop and review)
    - 1: Processing error

    **Note**: Script output includes statistics and failure detection handled by <CheckForFailures/>
</ProcessBatchResults>

<CheckForFailures>
    **Check result processing script exit code:**

    **Exit code handling:**
    - Exit code 0: All passed → **CONTINUE TO NEXT BATCH**
    - Exit code 2: **FAILURES DETECTED** → Stop and review
    - Exit code 1: Processing error → Stop

    **FAILURE PROTOCOL** (exit code 2):
    1. Failure details already saved by script to timestamped log
    2. Execute <FinalCleanup/> SILENTLY - no output during cleanup
    3. Execute <InteractiveFailureReview/> to review failures
    4. **DO NOT CONTINUE** to next batch

    **SUCCESS PROTOCOL** (exit code 0):
    - Continue directly to next batch

    **ERROR PROTOCOL** (exit code 1):
    - Report processing error
    - Execute <FinalCleanup/> SILENTLY
    - Stop execution
</CheckForFailures>

## STEP 6: FINAL CLEANUP

<FinalCleanup>
    **Shutdown all applications SILENTLY (no output):**

    Execute <ParallelPortOperation/> with:
    - Operation: mcp__brp__brp_shutdown
    - Parameters: app_name="extras_plugin"
    - Mode: SILENT (no status messages)

    **CRITICAL**: Do NOT display shutdown status messages. Execute silently.
</FinalCleanup>

## STEP 7: INTERACTIVE FAILURE REVIEW (Only if NEW failures detected)

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
    - **skip** - Skip this failure and continue to the next
    - **stop** - Stop reviewing failures and exit

    Please select one of the keywords above.
    ```

    3. **MANDATORY: Always investigate first**
    - Execute <InvestigateFailure/> IMMEDIATELY after presenting each failure
    - Present investigation findings to user
    - Only proceed to next failure if user explicitly selects a keyword after investigation

    4. **Wait for User Response** after investigation findings

    5. **Handle User Choice**:
    - **Investigate**: Already completed - confirm findings and wait for next keyword
    - **Skip**: Continue to next failure
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
- **skip** - Skip this failure and continue to the next
- **stop** - Stop reviewing failures and exit

Please select one of the keywords above.
```

**CRITICAL AGENT BEHAVIOR**:
- **ALWAYS** execute <InvestigateFailure/> first for every failure
- **ALWAYS** attempt to identify root cause and propose fixes

**Keyword Actions**:
- **Investigate**: Already executed automatically - present findings
- **Skip**: Continue to the next failure (temporary skip)
- **Stop**: Exit the failure review process immediately
</KeywordHandling>

<InvestigateFailure>
**Investigate the current failure using the type guide:**

1. Run: `.claude/scripts/get_type_guide.sh <failed_type_name> --file .claude/transient/all_types.json`
2. Examine the returned type guide focusing on the failed mutation path
3. Check `path_info` for the failed path (look for `applicable_variants`, `root_example`, `mutability`)
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

**Subagent non-response**: Check app logs at `/var/folders/.../bevy_brp_mcp_extras_plugin_port[PORT]_[timestamp].log` for assigned types' spawn crashes.
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
