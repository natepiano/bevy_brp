# Create Mutation Test JSON File

**CRITICAL** before doing anything else, read the tagged sections below and use them where referenced.

**DIRECTORY VALIDATION**: Ensure you are in the correct working directory before starting:
```bash
.claude/scripts/create_mutation_test_json/validate_directory.sh
```

<ExecutionSteps/>

<CreateContext>
TARGET_FILE = .claude/transient/all_types.json
PURPOSE = Creates the mutation test tracking file by discovering all registered component types via BRP and systematically determining spawn support and mutation paths for ALL types.
APP_PORT = 22222
APP_NAME = extras_plugin
</CreateContext>

<CreateKeywords>
**Main Decision Keywords:**
- **promote** - Mark this version as the new good baseline
- **skip** - Keep existing baseline, don't promote this version
- **investigate** - Launch deeper investigation of the differences
- **comparison_review** - Review unexpected patterns with examples
- **check_type** - Check mutation paths for a specific type
- **summarize** - Summarize test results from a JSON file
</CreateKeywords>

<ComparisonReviewKeywords>
**Pattern Review Keywords:**
- **continue** - Move to next pattern without any changes
- **add_expected** - Add this pattern to expected changes JSON
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
    **add_expected**: Add pattern to expected changes JSON:
    1. Extract from current pattern: pattern_type (e.g., "FIELD_REMOVED"), field name, and affected type names
    2. Ask user for human-readable reason/description for this expected change
    3. Generate next available ID and add entry to `.claude/config/create_mutation_test_json_expected_changes.json` with:
       - `pattern_type`: Pattern type from comparison (e.g., "FIELD_REMOVED", "FIELD_ADDED")
       - `field`: Field name from pattern (e.g., "example", "examples")
       - `affected_types`: Array of actual type names from current pattern
       - `reason`: Human-readable explanation for why this change is expected
    4. Continue to next pattern in review
    **check_type**: Ask user "Which type would you like me to check?", then use:
    ```bash
    python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural
    python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural_next
    ```
    to explore type+path combinations for that type in the structural review.
</KeywordExecution>

## AFTER BASELINE PROMOTION

When you promote a baseline with the **promote** keyword, the expected_changes.json file is automatically reset to `{"expected_changes": []}`. This is intentional to start fresh, but you need to know how to rebuild expected changes.

### Recovery Instructions

1. **Get format documentation**:
   ```bash
   python3 .claude/scripts/create_mutation_test_json/compare.py --help-expected-changes
   ```

2. **Copy from template**:
   View examples in `.claude/config/expected_changes_template.json` (this file is preserved in git)

3. **Rebuild through review**:
   - Run comparison: `python3 .claude/scripts/create_mutation_test_json/compare.py baseline.json current.json`
   - Use **comparison_review** keyword to systematically review unexpected patterns
   - Use **add_expected** keyword to add patterns to expected_changes.json
   - Build up the expected changes through manual categorization

4. **Pattern examples preserved**:
   - Enum variant qualified names pattern (most common)
   - Field addition/removal patterns
   - Type change patterns
   - All examples with proper regex and value conditions

The template file and help system ensure you can quickly rebuild expected changes even after a complete reset.

## MAIN WORKFLOW

**MANDATORY**: Create TodoWrite to track command progress through all steps and decision points.

Create a todo list with the following 6 items:
1. "Execute app launch and verify BRP connectivity" (pending → in_progress when starting STEP 1)
2. "Discover all registered types using brp_all_type_guides" (pending → in_progress when starting STEP 2)
3. "Transform BRP response and create mutation test file" (pending → in_progress when starting STEP 3)
4. "Clean shutdown of test application" (pending → in_progress when starting STEP 4)
5. "Compare with baseline and categorize changes" (pending → in_progress when starting STEP 5)
6. "Present results and get user decision on baseline promotion" (pending → in_progress when starting STEP 6)

Mark each todo as "in_progress" when beginning that step, and "completed" when the step finishes successfully.

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Execute the <AppLaunch/> → **STOP** and verify success before proceeding
    **STEP 2:** Execute the <TypeDiscovery/> → **STOP** and verify success before proceeding
    **STEP 3:** Execute the <FileTransformation/> → **STOP** and verify success before proceeding
    **STEP 4:** Execute the <AppCleanup/> → **STOP** and verify success before proceeding
    **STEP 5:** Execute the <ComparisonValidation/> → **STOP** and verify success before proceeding
    **STEP 6:** Execute the <UserValidation/> → **STOP** and present final summary
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

    2. **Verify BRP connectivity**:
    ```bash
    mcp__brp__brp_status(
        app_name="${APP_NAME}",
        port=${APP_PORT}
    )
    ```

    **VALIDATION**: Confirm both launch and BRP status are successful before proceeding to Step 2.

    **STOP** - Do not proceed until both confirmations are received.
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

    **STOP** - Do not proceed until filepath is confirmed.
</TypeDiscovery>

## STEP 3: FILE TRANSFORMATION

<FileTransformation>
    Augment the BRP response with test metadata while preserving FULL TYPE GUIDES:

    Execute the augmentation script:
    ```bash
    .claude/scripts/create_mutation_test_json/augment_response.sh [FILEPATH] ${TARGET_FILE}
    ```

    Replace `[FILEPATH]` with the actual path from Step 2 and `${TARGET_FILE}` with the target location from <CreateContext/>.

    The script augments the full BRP response with test metadata for each type:
    - Preserves ALL original type guide data (mutation_paths with examples, spawn_format, etc.)
    - Adds: batch_number: null
    - Adds: test_status: "untested" (or "passed" for auto-pass types)
    - Adds: fail_reason: ""

    **File Structure**: The file is the COMPLETE BRP response with added test fields

    Expected characteristics:
    - Complete type guides with spawn_format including examples
    - Complete mutation_paths as objects with path keys and example values
    - All test metadata fields added (test_status, batch_number, fail_reason)
    - Full preservation of schema_info, supported_operations, reflection traits

    **VALIDATION**: Confirm the script completed successfully and target file was created.

    **CRITICAL**: This maintains complete fidelity with BRP responses, storing actual examples for reproducible testing

    **STOP** - Do not proceed until file creation is confirmed.
</FileTransformation>

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

    **STOP** - Do not proceed until shutdown is confirmed.
</AppCleanup>

## STEP 5: COMPARISON AND VALIDATION

<ComparisonValidation>
    **Automatic Comparison with Baseline**

    After successful file creation, automatically compare with baseline:

    **CRITICAL FILE MANAGEMENT**:
    - DO NOT create backup copies or intermediate files
    - DO NOT use `cp` to create `all_types_previous.json` or any other files
    - ONLY read existing files for comparison
    - The comparison is ALWAYS between:
      - BASELINE: `.claude/transient/all_types_baseline.json` (the known good baseline)
      - CURRENT: `.claude/transient/all_types.json` (the newly created file from Step 3)

    **Run comprehensive comparison directly**:
    ```bash
    python3 .claude/scripts/create_mutation_test_json/compare.py .claude/transient/all_types_baseline.json .claude/transient/all_types.json
    ```

    This comprehensive comparison provides:
    - Binary identity check with early exit if identical
    - Structured metadata analysis (type counts, spawn support, mutations)
    - Type-level change detection (modified, new, removed types)
    - **Deep structural analysis** when changes are detected:
      - Categorizes changes into known patterns (enum representation, vec format, etc.)
      - Identifies unknown patterns that need investigation
      - Shows specific paths and examples of what changed
    - Clear recommendations based on the type of changes found

    **VALIDATION**: Confirm the comparison script completed and generated output.

    **STOP** - Do not proceed until comparison output is available.
</ComparisonValidation>

## STEP 6: USER VALIDATION

<UserValidation>
    **STEP 6A: CATEGORIZE CHANGES**

    **Automatically categorize changes using the expected changes JSON**:

    The comparison script handles everything internally:
    ```bash
    python3 .claude/scripts/create_mutation_test_json/compare.py .claude/transient/all_types_baseline.json .claude/transient/all_types.json
    ```

    This will automatically:
    - Run the structured comparison
    - Display the comparison results
    - If expected changes file exists, run categorization and output:
      - `expected_matches`: Changes that match expected patterns with their IDs and counts
      - `unexpected_patterns`: Changes that don't match any expected pattern or are below thresholds

    **STEP 6B: PRESENT SUMMARY**

    Present the final summary using the exact template below:

    The comparison script now provides all statistics including excluded types. Extract and present them in this format:

## Mutation Test File Generation Complete
- **File created**: ${TARGET_FILE}
- **Types registered in Bevy**: [extract from comparison output]
- **Spawn-supported types**: [extract from comparison output]
- **Types with mutations**: [extract from comparison output]
- **Total mutation paths**: [extract from comparison output]
- **Excluded types**: [extract from comparison output]

### Comparison with Baseline:
[Present the comparison results including:
 - If files are identical: Simple confirmation
 - If metadata only differs: Count differences
 - If structural changes exist: Full deep analysis output with two sections:]

#### Expected Changes:
[Use the categorization JSON output to list each matched expected change:
- Expected Change #N: [Name from expected_matches]
- Occurrences: [count from expected_matches]
- Types Affected: [if available from expected_matches]]

#### Unexpected Changes (need review):
[Use the categorization JSON output to list unexpected patterns:
- Pattern: [from unexpected_patterns]
- Occurrences: [count]
- Types Affected: [count if available]
- Reason: [why it's unexpected from the JSON output]]

   [IF NO unexpected_patterns in JSON: State "None - all detected patterns map to expected changes"]

    **STEP 6C: PRESENT DECISION PROMPT**

### Baseline Promotion Decision
Based on the comparison results above, should I mark this version as the new good baseline?

## Available Actions
- **promote** - Mark this version as the new good baseline
- **skip** - Keep existing baseline, don't promote this version
- **investigate** - Launch deeper investigation of the differences
- **comparison_review** - Show actual JSON examples for each unexpected change pattern, one type at a time, for user examination and testing decisions

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
- Show the COMPLETE JSON from compare_mutation_path.sh output
- Skip if script returns "IDENTICAL"
- Only present when script returns "DIFFERENT"
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
       Create todos for reviewing unexpected structural combinations identified in the overview.

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
          Use the comparison script (no permission needed once added to allowed tools):
          ```bash
          .claude/scripts/create_mutation_test_json/compare_mutation_path.sh "[TYPE_NAME]" "[MUTATION_PATH]"
          ```

          If output shows "IDENTICAL", skip to next combination without user interaction.
          If "DIFFERENT", format the output using <FormatComparison/>.

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
          - **add_expected** - Add this pattern to expected changes JSON, then continue to next combination
          - **stop** - End the review now

          - **DO NOT PROCEED** until user responds with one of the keywords
          - If "continue": Continue to next combination
          - If "add_expected": Add pattern to expected changes JSON, then continue to next combination
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

