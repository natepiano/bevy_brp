# Create Mutation Test JSON File

**CRITICAL** before doing anything else, read the tagged sections below and use them where referenced.

**DIRECTORY VALIDATION**: Ensure you are in the correct working directory before starting:
```bash
.claude/scripts/create_mutation_test_json/validate_directory.sh
```

## Command Arguments

This command supports an optional argument to control test result handling:

**Default (no arguments)**: `/create_mutation_test_json`
- **Preserves test results** (batch_number, test_status, fail_reason) from existing all_types.json
- New types get initialized with default test metadata ("untested" or "passed")
- Removed types are automatically cleaned up

**Initialize mode**: `/create_mutation_test_json init` or `/create_mutation_test_json initialize`
- Replaces all_types.json with fresh BRP data (no test metadata)
- Test metadata will be auto-initialized by prepare.py when needed
- Use this when you want to clear all test history and start over

<ExecutionSteps/>

<CreateContext>
TARGET_FILE = .claude/transient/all_types.json
PURPOSE = Creates the mutation test tracking file by discovering all registered component types via BRP and systematically determining spawn support and mutation paths for ALL types.
APP_PORT = 22222
APP_NAME = extras_plugin
MODE = ${ARGUMENTS:-preserve}
</CreateContext>

<CreateKeywords>
**Main Decision Keywords:**
- **promote** - Mark this version as the new good baseline
- **skip** - Keep existing baseline, don't promote this version
- **investigate** - Launch deeper investigation of the differences
- **comparison_review** - Review change patterns with examples
- **check_type** - Check mutation paths for a specific type
- **summarize** - Summarize test results from a JSON file
</CreateKeywords>

<ComparisonReviewKeywords>
**Pattern Review Keywords:**
- **continue** - Move to next pattern without any changes
- **stop** - End the review now
</ComparisonReviewKeywords>

<KeywordExecution>
    **CRITICAL**: Follow tagged procedures for all execution steps.

    **promote**: Mark version as baseline:
    ```bash
    .claude/scripts/create_mutation_test_json/promote_baseline.sh
    ```
    **skip**: Keep existing baseline, document decision, continue
    **investigate**: Ask user "What specific aspect would you like me to investigate?", then launch Task tool with their focus
    **comparison_review**: Execute <ComparisonReviewWorkflow/>
    **check_type**: Ask user "Which type would you like me to check?", then use:
    ```bash
    python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural
    python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural_next
    ```
    to explore type+path combinations for that type in the structural review.
</KeywordExecution>

## MAIN WORKFLOW

**MANDATORY**: Create TodoWrite to track command progress through all steps and decision points.

Create a todo list with the following 5 items:
1. "Execute app launch" (pending → in_progress when starting STEP 1)
2. "Discover all registered types using brp_all_type_guides" (pending → in_progress when starting STEP 2)
3. "Save BRP response, merge test metadata, and run comparison" (pending → in_progress when starting STEP 3)
4. "Clean shutdown of test application" (pending → in_progress when starting STEP 4)
5. "Present results and get user decision on baseline promotion" (pending → in_progress when starting STEP 5)

Mark each todo as "in_progress" when beginning that step, and "completed" when the step finishes successfully.

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Execute the <AppLaunch/>
    **STEP 2:** Execute the <TypeDiscovery/>
    **STEP 3:** Execute the <FileSaveAndComparison/> (save raw BRP response and run comparison)
    **STEP 4:** Execute the <AppCleanup/>
    **STEP 5:** Execute the <UserValidation/> → **STOP** and present final summary
</ExecutionSteps>

## STEP 1: APP LAUNCH

<AppLaunch>
    Launch the extras_plugin app on the designated port:

    1. **Launch Example**:
    ```bash
    mcp__brp__brp_launch_bevy_example(
        example_name="${APP_NAME}",
        port=${APP_PORT}
    )
    ```
</AppLaunch>

## STEP 2: TYPE DISCOVERY

<TypeDiscovery>
    Get all type guides using the comprehensive discovery tool:

    Call `brp_all_type_guides` to get type guides for all registered types in one operation:
    ```bash
    mcp__brp__brp_all_type_guides(port=${APP_PORT})
    ```

    This automatically discovers all registered types and returns their type guides. The tool will save its result to a file and return the filepath (e.g., `/var/folders/.../mcp_response_brp_all_type_guides_12345.json`).

    **VALIDATION**: Confirm the tool returned a valid filepath and type count.

    **CRITICAL**: Note the returned filepath for use in Step 3.

</TypeDiscovery>

## STEP 3: FILE SAVE AND METADATA MERGE

<FileSaveAndComparison>
    Process the BRP response based on MODE:

    **If MODE is "preserve" (default)**:
    Merge new BRP data with existing test metadata:
    ```bash
    .claude/scripts/create_mutation_test_json/preserve_test_metadata.sh [FILEPATH] ${TARGET_FILE}
    ```

    This preserves test results (batch_number, test_status, fail_reason) from existing types, initializes new types with "untested" or auto-pass status, and automatically removes types no longer in BRP.

    **If MODE is "init" or "initialize"**:
    Save raw BRP response (without test metadata):
    ```bash
    cp [FILEPATH] ${TARGET_FILE}
    ```

    The file will contain only the raw BRP data. Test metadata fields (batch_number, test_status, fail_reason) will be automatically initialized when prepare.py is run.

    Replace `[FILEPATH]` with the actual path from Step 2 and `${TARGET_FILE}` with the target location.

    **After processing, run comparison**:
    ```bash
    python3 .claude/scripts/create_mutation_test_json/compare.py .claude/transient/all_types_baseline.json ${TARGET_FILE}
    ```

    The file contains the COMPLETE BRP response with test metadata:
    - spawn_format with examples
    - mutation_paths with examples for each path
    - supported_operations
    - reflection_traits
    - schema_info
    - **PLUS test metadata** (batch_number, test_status, fail_reason)

    **Comparison Output**: The compare.py script displays:
    - Current file statistics (total types, spawn-supported types, types with mutations, total mutation paths)
    - Comparison results (total changes, types modified/added/removed)
    - `✅ No changes detected!` or `⚠️ CHANGES DETECTED: X changes`

    **VALIDATION**: Confirm the file was saved, metadata was added/preserved, and comparison completed successfully.

</FileSaveAndComparison>

## STEP 4: APP CLEANUP

<AppCleanup>
    Shutdown the application:

    ```bash
    mcp__brp__brp_shutdown(
        app_name="${APP_NAME}",
        port=${APP_PORT}
    )
    ```

    **VALIDATION**: Confirm the app has been cleanly shutdown before proceeding.

</AppCleanup>

## STEP 5: USER VALIDATION

<UserValidation>
    Present the final summary using the exact template below:

## Mutation Test File Generation Complete
- **File created**: ${TARGET_FILE}
- **Types registered in Bevy**: [extract from compare.py "Total types" output]
- **Spawn-supported types**: [extract from compare.py "Spawn-supported types" output]
- **Types with mutations**: [extract from compare.py "Types with mutations" output]
- **Total mutation paths**: [extract from compare.py "Total mutation paths" output]

### Comparison with Baseline:
[Present the comparison summary from compare.py output:
 - If "No changes detected": Simple confirmation
 - If "Changes detected": Report the total changes, types modified, types added, types removed]

    **PRESENT DECISION PROMPT**

### Baseline Promotion Decision
Based on the comparison results above, should I mark this version as the new good baseline?

## Available Actions
- **promote** - Mark this version as the new good baseline
- **skip** - Keep existing baseline, don't promote this version
- **investigate** - Launch deeper investigation of the differences
- **comparison_review** - Show actual JSON examples for each change pattern, one type at a time, for user examination and testing decisions

    **CRITICAL**: STOP and wait for user's keyword response before proceeding.

    **Note**: Use the keyword from Available Actions - do not continue with detailed analysis unless user specifically requests **comparison_review** or **investigate**.
</UserValidation>

## SHARED DEFINITIONS

<FormatComparison>
**Present differences with full context:**

## Mutation Path Comparison

**Type**: `[TYPE_NAME]`
**Path**: `[MUTATION_PATH]`
**Change**: [Brief description of what changed, e.g., "examples array → example field"]

```json
// BASELINE
[Paste baseline JSON from script output after "=== BASELINE ==="]
```

```json
// CURRENT
[Paste current JSON from script output after "=== CURRENT ==="]
```

**CRITICAL FOR comparison_review**:
- ALWAYS include Type, Path, and Change summary headers
- Show the COMPLETE JSON comparison data from the structural review
- Focus on the actual nested field changes identified
</FormatComparison>

<NoIntermediateFiles>
**NO intermediate files** - Do NOT create Python scripts, temp files, or any other files beyond the target file
</NoIntermediateFiles>

<DirectToolsOnly>
**Direct tool usage only** - Use only MCP tools and the provided shell scripts
</DirectToolsOnly>

<SingleOutputFile>
**Single output file** - Only create/modify the target file specified in <CreateContext/>
</SingleOutputFile>

<UseActualBrpResponses>
**Use actual BRP responses** - Base spawn support and mutation paths on actual BRP discovery, not assumptions
</UseActualBrpResponses>

<ExecuteShellScripts>
**Execute shell scripts** - Use the provided transformation and statistics scripts as specified
</ExecuteShellScripts>

<MutationPathsExplanation>
**Understanding Mutation Paths vs JSON Paths**

Mutation paths are string keys in `mutation_paths` dict, NOT JSON navigation:
- **Mutation path key**: `.image_mode.0.center_scale_mode`
- **Access**: `type_guide['TypeName']['mutation_paths']['.image_mode.0.center_scale_mode']`
- **In output**: `mutation_paths..image_mode.0` (double dot = parent + '.' + key)

Patterns: `.field.0` (variant), `.field[0]` (array), `.field.0.nested` (nested in variant)
</MutationPathsExplanation>

<ComparisonReviewWorkflow>
    **STRUCTURAL REVIEW APPROACH**:

    **FIRST: Review <MutationPathsExplanation/> to understand mutation paths.**

    Review changes organized by Type+Path combinations instead of individual changes.
    This reduces overwhelming change counts (e.g., 4000+ changes → 181 combinations).

    1. **GET STRUCTURAL OVERVIEW**:
       ```bash
       python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural
       ```
       This shows all type+path combinations that have changes, grouped by type.

    2. **CREATE TODOS FOR REVIEW**:
       Create todos for reviewing structural combinations identified in the overview.

    3. **INTERACTIVE STRUCTURAL REVIEW**:
       Walk through type+path combinations one at a time:

       a. Get the next combination to review:
          ```bash
          python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural_next
          ```

       b. The tool shows complete details for one type+path combination including:
          - Type name and mutation path
          - Total changes in this combination
          - Change type summary (added, removed, value_changed, etc.)
          - Representative examples of the changes

       c. **RETRIEVE AND VALIDATE MUTATION PATH DATA**:
          The structural review shows nested field differences within mutation paths.
          These changes are already captured in the comparison data.

          Skip to next combination if no meaningful differences exist.

       d. **MANDATORY VERIFICATION BEFORE PROCEEDING**:
          - Compare the retrieved baseline vs current JSON data
          - **VERIFICATION REQUIREMENT**: The claimed change pattern MUST be visible in the data
          - If FIELD_REMOVED claimed: baseline must have the field, current must lack it
          - If FIELD_ADDED claimed: baseline must lack the field, current must have it
          - If data appears identical: IMMEDIATELY flag as "VERIFICATION FAILED"

       e. **VERIFICATION FAILURE HANDLING**:
          - If verification fails, state: "Verification failed - retrieved data shows no difference for this combination"
          - Try the next example from the same type+path combination
          - If 3 consecutive examples fail verification, mark combination as "False positive - unable to verify"
          - Skip to next combination without user interaction

       f. **ONLY PROCEED WITH USER INTERACTION IF VERIFICATION SUCCEEDS**:
          - Present using <FormatComparison/> showing the COMPLETE JSON
          - Only show examples where differences are actually visible

       g. **CRITICAL - STOP AND WAIT**: After presenting VERIFIED combination example:
          - Present the following options:

          ## Available Actions
          - **continue** - Move to next combination without any changes
          - **stop** - End the review now

          - **DO NOT PROCEED** until user responds with one of the keywords
          - If "continue": Continue to next combination
          - If "stop": End review and return to main decision prompt

    4. **SESSION MANAGEMENT**:
       - Review session is persistent - you can stop and resume
       - Use `structural_reset` to start over from beginning
       - Tool remembers current position through review

    5. **COMMON FAILURE MODES TO AVOID**:
       - Assuming combinations will show differences without verification
       - Proceeding with user interaction when verification fails
       - Using type/path combinations from previous runs instead of current session

    6. After all combinations reviewed OR user stops:
       - Return to main decision prompt from Step 6C
</ComparisonReviewWorkflow>

<PatternOverviewFormat>
## Pattern [NUMBER]: [PATTERN_NAME]

**Occurrences**: [COUNT] changes across [TYPE_COUNT] types
**What this means**: [EXPLANATION]

Retrieving first example - [TYPE_NAME] at path [PATH]:
</PatternOverviewFormat>
