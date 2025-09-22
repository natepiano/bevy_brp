# Create Mutation Test JSON File

**CRITICAL** before doing anything else, read the tagged sections below and use them where referenced.

**DIRECTORY VALIDATION**: Ensure you are in the correct working directory before starting:
```bash
if [[ ! -f ".claude/commands/create_mutation_test_json.md" ]]; then
    echo "❌ Not in bevy_brp root directory. Please cd to the project root."
    exit 1
fi
```

<ExecutionSteps/>

<CreateContext>
[TARGET_FILE]: `.claude/transient/all_types.json`
[PURPOSE]: Creates the mutation test tracking file by discovering all registered component types via BRP and systematically determining spawn support and mutation paths for ALL types.
[APP_PORT]: 22222
[APP_NAME]: extras_plugin
</CreateContext>

<CreateKeywords>
**For validation decisions:**
- **promote**: Mark this version as the new good baseline
- **skip**: Keep existing baseline, don't promote this version
- **investigate**: Launch deeper investigation of the differences
- **comparison_review** - Show actual JSON examples for each unexpected change pattern, one type at a time, for user examination and testing decisions
- **check_type**: Check mutation paths for a specific type across all versions
- **summarize**: Summarize test results from a JSON file

**For comparison_review decisions:**
- **continue**: Move to next pattern without any changes
- **add_expected**: Add this pattern to expected changes JSON, then continue to next pattern
- **stop**: End the review now
</CreateKeywords>

<KeywordExecution>
    **CRITICAL**: Follow tagged procedures for all execution steps.

    **promote**: Mark version as baseline:
    ```bash
    .claude/scripts/create_mutation_test_json_promote_baseline.sh
    ```
    **skip**: Keep existing baseline, document decision, continue
    **investigate**: Ask user "What specific aspect would you like me to investigate?", then launch Task tool with their focus
    **comparison_review**:
    **DATA SOURCE HIERARCHY (STRICT ORDER)**:
    1. Current comparison run output (ALWAYS most reliable)
    2. Never use cached detail files
    3. Never use examples from previous comparison runs

    1. **ALWAYS re-run fresh comparison to get current examples**:
       ```bash
       .claude/scripts/create_mutation_test_json_structured_comparison.sh .claude/transient/all_types_baseline.json .claude/transient/all_types.json
       ```
       **CRITICAL**: Use examples from THIS current run output, never cached files.

    2. **EXTRACT EXAMPLES FROM CURRENT COMPARISON OUTPUT**:
       - For each FIELD_REMOVED/FIELD_ADDED pattern in the current comparison output
       - Use the first example listed in that pattern's "Example 1:" section
       - **NEVER** rely on separate detail files or previous comparison runs

    3. Create todos for each unexpected change pattern identified

    4. **INTERACTIVE REVIEW**: For each unexpected pattern from the CURRENT comparison:
       a. Present pattern overview:
          - Pattern name and occurrence count from current run
          - Types affected count from current run
          - Brief explanation of what this pattern means

       b. **EXTRACT EXAMPLE FROM CURRENT COMPARISON OUTPUT**:
          - Take the first "Example 1:" entry from the current comparison output for this pattern
          - Extract the Type and Path from "Example 1:" (format: "Type: X, Path: Y")
          - **NEVER use examples from detail files or previous runs**

       c. **RETRIEVE AND VERIFY MUTATION PATH DATA**:
          **IMPORTANT**: Extract only the mutation path from the comparison output.

          Example: If comparison shows `Path: mutation_paths..0.0.example`, use only `.0.0` (remove `mutation_paths.` prefix and `.example` suffix)

          ```bash
          .claude/scripts/get_mutation_path.sh "[TYPE_FROM_CURRENT_OUTPUT]" "[MUTATION_PATH_ONLY]" .claude/transient/all_types_baseline.json
          .claude/scripts/get_mutation_path.sh "[TYPE_FROM_CURRENT_OUTPUT]" "[MUTATION_PATH_ONLY]" .claude/transient/all_types.json
          ```

          Where `[MUTATION_PATH_ONLY]` is the mutation path key (like `.0.0`) extracted from the full comparison path.

       d. **MANDATORY VERIFICATION BEFORE PROCEEDING**:
          - Compare the retrieved baseline vs current JSON data
          - **VERIFICATION REQUIREMENT**: The claimed change pattern MUST be visible in the data
          - If FIELD_REMOVED claimed: baseline must have the field, current must lack it
          - If FIELD_ADDED claimed: baseline must lack the field, current must have it
          - If data appears identical: IMMEDIATELY flag as "VERIFICATION FAILED"

       e. **VERIFICATION FAILURE HANDLING**:
          - If verification fails, state: "Verification failed - retrieved data shows no difference for this example"
          - Try the next example from the same pattern in current comparison output
          - If 3 consecutive examples fail verification, mark pattern as "False positive - unable to verify"
          - Skip to next pattern without user interaction

       f. **ONLY PROCEED WITH USER INTERACTION IF VERIFICATION SUCCEEDS**:
          - Present using <FormatComparison/> showing the COMPLETE JSON
          - Only show examples where differences are actually visible

       g. **CRITICAL - STOP AND WAIT**: After presenting VERIFIED pattern example:
          - Present the following options:

- **continue** - Move to next pattern without any changes
- **add_expected** - Add this pattern to expected changes JSON, then continue to next pattern
- **stop** - End the review now

          - **DO NOT PROCEED** until user responds with one of the keywords
          - If "continue": Continue to next pattern
          - If "add_expected": Add pattern to expected changes JSON, then continue to next pattern
          - If "stop": End review and return to main decision prompt

    5. **COMMON FAILURE MODES TO AVOID**:
       - Using examples from detail files instead of current comparison output
       - Assuming examples will show differences without verification
       - Proceeding with user interaction when verification fails
       - Using type/path combinations from previous runs

    6. After all patterns reviewed OR user stops:
       - Return to main decision prompt from Step 6C
    **add_expected**: Add pattern to expected changes JSON:
    1. Extract from current pattern: pattern_type (e.g., "FIELD_REMOVED"), field name, and affected type names
    2. Ask user for human-readable reason/description for this expected change
    3. Generate next available ID and add entry to `.claude/transient/create_mutation_test_json_expected_changes.json` with:
       - `pattern_type`: Pattern type from comparison (e.g., "FIELD_REMOVED", "FIELD_ADDED")
       - `field`: Field name from pattern (e.g., "example", "examples")
       - `affected_types`: Array of actual type names from current pattern
       - `reason`: Human-readable explanation for why this change is expected
    4. Continue to next pattern in review
    **check_type**: Ask user "Which type would you like me to check?", then execute:
    ```bash
    python3 .claude/scripts/compare_mutations_check_type.py "[TYPE_NAME]"
    ```
    **summarize**: Ask user "Which JSON file would you like me to summarize?", then execute:
    ```bash
    .claude/scripts/compare_mutations_summarize.sh [JSON_FILE]
    ```
</KeywordExecution>

## MAIN WORKFLOW

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
        example_name="[APP_NAME]",
        port=[APP_PORT]
    )
    ```

    2. **Verify BRP connectivity**:
    ```bash
    mcp__brp__brp_status(
        app_name="[APP_NAME]",
        port=[APP_PORT]
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
    mcp__brp__brp_all_type_guides(port=[APP_PORT])
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
    .claude/scripts/create_mutation_test_json_augment_response.sh [FILEPATH] [TARGET_FILE]
    ```

    Replace `[FILEPATH]` with the actual path from Step 2 and `[TARGET_FILE]` with the target location from <CreateContext/>.

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
        app_name="[APP_NAME]",
        port=[APP_PORT]
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

    **Run structured comparison directly**:
    ```bash
    .claude/scripts/create_mutation_test_json_structured_comparison.sh .claude/transient/all_types_baseline.json .claude/transient/all_types.json
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

    The comparison script now handles categorization internally:
    ```bash
    .claude/scripts/create_mutation_test_json_structured_comparison.sh .claude/transient/all_types_baseline.json .claude/transient/all_types.json
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
- **File created**: [TARGET_FILE]
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
**Format the side-by-side JSON comparison with proper syntax highlighting:**

When presenting JSON comparisons, use this exact format with proper markdown JSON code blocks:

```json
// BASELINE
{
  "field_name": "value",
  "nested": {
    "data": [...]
  }
}
```

```json
// CURRENT
{
  "field_name": "new_value",
  "nested": {
    "data": [...]
  }
}
```

**CRITICAL FOR comparison_review**:
- Show the COMPLETE JSON returned by get_mutation_path.sh
- DO NOT extract or show only selective fields
- DO NOT abbreviate or use [...] placeholders in comparison_review mode
- The full JSON structure must be visible for proper review
- Only use abbreviations in the initial summary, not in detailed review
- **VERIFICATION MANDATORY**: Before presenting comparison, verify that the claimed change (FIELD_ADDED/FIELD_REMOVED) is actually visible in the JSON difference
- **NEVER present comparisons where both sides appear identical**

**General formatting**:
- Use separate ```json code blocks for BASELINE and CURRENT
- Include // BASELINE and // CURRENT comments inside the code blocks
- Use proper JSON formatting with correct indentation
- Add inline comments with // <-- to highlight key differences when helpful
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

<UserValidation>
**User validation required** - Must present comparison results and get user approval before baseline promotion
</UserValidation>
