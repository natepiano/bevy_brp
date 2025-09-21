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
[TARGET_FILE]: `.claude/types/all_types.json`
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
    1. Create todos for each unexpected change pattern identified
    2. For each pattern, select one representative type/mutation path
    3. Extract and format the actual JSON from baseline vs current using <FormatComparison/>
    4. Present to user with pattern context for examination and testing decision
    5. Wait for user response before proceeding to next pattern
    6. Stop when user says to stop or all patterns reviewed
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

    1. **Save previous version** (if it exists):
    ```bash
    if [ -f "[TARGET_FILE]" ]; then
        cp [TARGET_FILE] .claude/types/all_types_previous.json
    fi
    ```

    2. **Run structured comparison**:
    ```bash
    .claude/scripts/create_mutation_test_json_structured_comparison.sh .claude/types/all_types_baseline.json .claude/types/all_types.json
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

    **IMPORTANT**: Before presenting results, systematically categorize ALL changes:

    1. **Read Expected Changes**: Read the .claude/EXPECTED_CHANGES.md file to understand all expected patterns

    2. **Map Comparison Output to Expected Changes**: For each pattern in the comparison output, determine if it matches an expected change:
       - **FIELD REMOVED with 'variants' field**: Maps to Expected Change #1 (variants removal)
       - **FIELD REMOVED with 'enum_info' field**: Maps to Expected Change #2 (enum_info removal)
       - **VALUE CHANGE with path_requirement additions**: Maps to Expected Change #3 (path_requirement addition)
       - **New Types with extras_plugin::NestedConfigEnum**: Maps to Expected Change #4 (test type addition)
       - **VALUE CHANGE with enum example simplification**: Maps to Expected Change #5 (enum example format)

    3. **Identify Unexpected Patterns**: ANY pattern or change not covered by the above mapping is UNEXPECTED:
       - **TYPE CHANGE patterns** (unless covered by expected changes)
       - **FIELD ADDED patterns** (unless covered by expected changes)
       - **VALUE CHANGE patterns** that don't match expected change descriptions
       - **New/Removed types** not listed in expected changes

    4. **Count and Summarize**:
       - Count total changes for each expected change category
       - Count total changes for unexpected patterns
       - Prepare summaries for both categories

    **STEP 6B: ANALYZE COMPARISON OUTPUT**

    **MANDATORY**: Before using the template, explicitly analyze each pattern from the comparison output:

    1. **List ALL patterns detected**: Write out every "IDENTIFIED PATTERN" from the comparison output
    2. **Map each pattern**: For each pattern, state which expected change it maps to OR mark as "UNEXPECTED"
    3. **Calculate totals**: Sum up changes for expected vs unexpected categories
    4. **Verify completeness**: Ensure every single pattern has been categorized

    **Example Analysis Structure**:
    ```
    PATTERN: FIELD REMOVED (variants field, 872 removals) → Expected Change #1 ✓
    PATTERN: FIELD REMOVED (enum_info field, 18 removals) → Expected Change #2 ✓
    PATTERN: VALUE CHANGE (1149 changes) → Expected Change #3 & #5 ✓
    PATTERN: TYPE CHANGE (18 types, 22 changes) → UNEXPECTED ❌
    PATTERN: FIELD ADDED (39 types, 122 changes) → UNEXPECTED ❌
    ```

    **STEP 6C: PRESENT SUMMARY**

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

   #### Expected Changes (from .claude/EXPECTED_CHANGES.md):
   [For each matched expected change pattern, list:
    - Expected Change #N: [Name from EXPECTED_CHANGES.md]
    - Summary of what matched this pattern
    - Count of changes matching this pattern]

   #### Unexpected Changes (need review):
   **CRITICAL**: MUST analyze ALL patterns not mapped to expected changes above:

   [FOR EACH UNEXPECTED PATTERN found in Step 6A:
    - Pattern Name: [e.g., "TYPE CHANGE", "FIELD ADDED", etc.]
    - Types Affected: [count]
    - Total Changes: [count]
    - Brief Summary: [what kind of changes these are]
    - Example: [show 1-2 specific examples of the change]]

   [IF TRULY NO UNEXPECTED CHANGES: State "None - all detected patterns map to expected changes"]

   **WARNING**: If TYPE CHANGE or FIELD ADDED patterns exist and are not explicitly covered by expected changes, they MUST be listed here as unexpected.

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

**CRITICAL**:
- Use separate ```json code blocks for BASELINE and CURRENT
- Include // BASELINE and // CURRENT comments inside the code blocks
- Use proper JSON formatting with correct indentation
- Use [...] or {...} to abbreviate unchanged nested content
- Add inline comments with // <-- to highlight key differences
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
