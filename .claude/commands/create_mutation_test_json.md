# Create Mutation Test JSON File

**CRITICAL** before doing anything else, read the tagged sections below and use them where referenced.

<ExecutionSteps/>

<CreateContext>
[TARGET_FILE]: `$TMPDIR/all_types.json`
[PURPOSE]: Creates the mutation test tracking file by discovering all registered component types via BRP and systematically determining spawn support and mutation paths for ALL types.
[APP_PORT]: 22222
[APP_NAME]: extras_plugin
</CreateContext>

<CreateKeywords>
    **For validation decisions:**
    - **promote**: Mark this version as the new good baseline
    - **skip**: Keep existing baseline, don't promote this version
    - **investigate**: Launch deeper investigation of the differences
    - **check_type**: Check mutation paths for a specific type across all versions
    - **summarize**: Summarize test results from a JSON file
</CreateKeywords>

<KeywordExecution>
    **CRITICAL**: Follow tagged procedures for all execution steps.

    **promote**: Mark version as baseline:
    ```bash
    .claude/commands/scripts/create_mutation_test_json_promote_baseline.sh
    ```
    **skip**: Keep existing baseline, document decision, continue
    **investigate**: Ask user "What specific aspect would you like me to investigate?", then launch Task tool with their focus
    **check_type**: Ask user "Which type would you like me to check?", then execute:
    ```bash
    python3 .claude/commands/scripts/compare_mutations_check_type.py "[TYPE_NAME]"
    ```
    **summarize**: Ask user "Which JSON file would you like me to summarize?", then execute:
    ```bash
    .claude/commands/scripts/compare_mutations_summarize.sh [JSON_FILE]
    ```
</KeywordExecution>

## MAIN WORKFLOW

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Execute the <AppLaunch/>
    **STEP 2:** Execute the <TypeDiscovery/>
    **STEP 3:** Execute the <FileTransformation/>
    **STEP 4:** Execute the <AppCleanup/>
    **STEP 5:** Execute the <ComparisonValidation/>
    **STEP 6:** Execute the <UserValidation/>
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

    Wait for confirmation that BRP is responding before proceeding to Step 2.
</AppLaunch>

## STEP 2: TYPE DISCOVERY

<TypeDiscovery>
    Get all type guides using the comprehensive discovery tool:

    Call `brp_all_type_guides` to get type guides for all registered types in one operation:
    ```bash
    mcp__brp__brp_all_type_guides(port=[APP_PORT])
    ```

    This automatically discovers all registered types and returns their type guides. The tool will save its result to a file and return the filepath (e.g., `/var/folders/.../mcp_response_brp_all_type_guides_12345.json`).

    **CRITICAL**: Note the returned filepath for use in Step 3.
</TypeDiscovery>

## STEP 3: FILE TRANSFORMATION

<FileTransformation>
    Augment the BRP response with test metadata while preserving FULL TYPE GUIDES:

    Execute the augmentation script:
    ```bash
    .claude/commands/scripts/create_mutation_test_json_augment_response.sh [FILEPATH] [TARGET_FILE]
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

    **CRITICAL**: This maintains complete fidelity with BRP responses, storing actual examples for reproducible testing
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

    Confirm the app has been cleanly shutdown before proceeding.
</AppCleanup>

## STEP 5: COMPARISON AND VALIDATION

<ComparisonValidation>
    **Automatic Comparison with Baseline**

    After successful file creation, automatically compare with baseline:

    1. **Save previous version** (if it exists):
    ```bash
    if [ -f "[TARGET_FILE]" ]; then
        cp [TARGET_FILE] $TMPDIR/all_types_previous.json
    fi
    ```

    2. **Run structured comparison**:
    ```bash
    .claude/commands/scripts/create_mutation_test_json_structured_comparison.sh $TMPDIR/all_types_baseline.json $TMPDIR/all_types.json
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
</ComparisonValidation>

## STEP 6: USER VALIDATION

<UserValidation>
    **Present Comparison Results to User for Baseline Decision**

    **First, dynamically determine excluded types**:
    ```bash
    # Get list of excluded types from the exclusion file
    EXCLUDED_TYPES=$(.claude/commands/scripts/create_mutation_test_json_get_excluded_types.sh)
    ```

    **Parse the comparison output and format the final presentation:**
    
    The comparison script provides all statistics. Extract and present them in this format:

## Mutation Test File Generation Complete
- **File created**: [TARGET_FILE]
- **Types registered in Bevy**: [extract from comparison output]
- **Spawn-supported types**: [extract from comparison output]
- **Types with mutations**: [extract from comparison output]
- **Total mutation paths**: [extract from comparison output]
- **Excluded types**: [from EXCLUDED_TYPES variable]

### Comparison with Baseline:
[Present the comparison results including:
 - If files are identical: Simple confirmation
 - If metadata only differs: Count differences
 - If structural changes exist: Full deep analysis output showing:
   * **Field Removals**: List ALL removed fields with counts (e.g., "example field: 25 removals across 18 types")
   * **Field Additions**: List ALL added fields with counts (e.g., "mutation_status_reason field: 25 additions")
   * **Value Changes**: Significant value changes while structure remains the same
   * Known patterns (enum representation, vec format changes, etc.)
   * Unknown patterns requiring investigation
   * Specific examples of what changed
 
 CRITICAL: Always explicitly list ALL field removals and additions from the "FIELD REMOVED" and "FIELD ADDED" sections of the comparison output]

### Baseline Promotion Decision
Based on the comparison results above, should I mark this version as the new good baseline?

## Available Actions
- **promote** - Mark this version as the new good baseline
- **skip** - Keep existing baseline, don't promote this version
- **investigate** - Launch deeper investigation of the differences (especially for unknown patterns)

    **CRITICAL**: STOP and wait for user's keyword response before proceeding.
</UserValidation>

## SHARED DEFINITIONS

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
