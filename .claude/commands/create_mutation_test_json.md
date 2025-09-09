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
    **STEP 4:** Execute the <ResultsReporting/>
    **STEP 5:** Execute the <AppCleanup/>
    **STEP 6:** Execute the <ComparisonValidation/>
    **STEP 7:** Execute the <UserValidation/>
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
    Get all type schemas using the comprehensive discovery tool:

    Call `brp_all_type_schemas` to get schemas for all registered types in one operation:
    ```bash
    mcp__brp__brp_all_type_schemas(port=[APP_PORT])
    ```

    This automatically discovers all registered types and returns their schemas. The tool will save its result to a file and return the filepath (e.g., `/var/folders/.../mcp_response_brp_all_type_schemas_12345.json`).

    **CRITICAL**: Note the returned filepath for use in Step 3.
</TypeDiscovery>

## STEP 3: FILE TRANSFORMATION

<FileTransformation>
    Transform the BRP response into the mutation test tracking format:

    Execute the transformation script:
    ```bash
    .claude/commands/scripts/create_mutation_test_json_transform_response.sh [FILEPATH] [TARGET_FILE]
    ```

    Replace `[FILEPATH]` with the actual path from Step 2 and `[TARGET_FILE]` with the target location from <CreateContext/>.

    The script creates the target file with all discovered types initialized with `batch_number: null`.

    **File Structure Validation**:
    The completed file is structured as a JSON array of type objects with this structure:
    ```json
    {
      "type": "fully::qualified::TypeName",
      "spawn_support": "supported" | "not_supported",
      "mutation_paths": ["array", "of", "mutation", "paths"],
      "test_status": "untested" | "passed",
      "batch_number": null,
      "fail_reason": ""
    }
    ```

    Expected characteristics:
    - All types with spawn support properly identified (`"supported"` or `"not_supported"`)
    - All types with mutation paths listed as arrays (includes root path `""` for full component replacement using spawn_format)
    - All types starting with `test_status: "untested"` (except auto-passed spawn types)
    - All types starting with `batch_number: null` (batch assignment done separately)
    
    **IMPORTANT**: The root mutation path `""` (empty string) represents full component replacement and uses the same format as spawn/insert operations. This allows mutation testing to validate both spawn/insert capabilities and full component replacement.
</FileTransformation>

## STEP 4: RESULTS REPORTING

<ResultsReporting>
    Generate comprehensive statistics about the created file:

    ```bash
    .claude/commands/scripts/create_mutation_test_json_stats.sh [TARGET_FILE]
    ```

    This script provides comprehensive statistics including:
    - Capability summary
    - Test status breakdown
    - Batch information
    - Progress tracking metrics

    Review the statistics to ensure the file was created successfully.
</ResultsReporting>

## STEP 5: APP CLEANUP

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

## STEP 6: COMPARISON AND VALIDATION

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

    This single comprehensive comparison provides:
    - Binary identity check with early exit if identical
    - Structured metadata analysis (type counts, spawn support, mutations)
    - Type-level change detection (modified, new, removed types)
    - Change assessment and recommendation
    - Pre-formatted output ready for user presentation
</ComparisonValidation>

## STEP 7: USER VALIDATION

<UserValidation>
    **Present Comparison Results to User for Baseline Decision**

    **First, dynamically determine excluded types**:
    ```bash
    # Get list of excluded types from the exclusion file
    EXCLUDED_TYPES=$(.claude/commands/scripts/create_mutation_test_json_get_excluded_types.sh)
    ```

Present the comparison analysis in this format:

## Mutation Test File Generation Complete
- **File created**: [TARGET_FILE]
- **Types registered in Bevy**: [types discovered by BRP]
- **Types in mutation test file**: [total count] ([types discovered] - [excluded count] excluded)
  - Excluded: [actual excluded types from EXCLUDED_TYPES variable]
- **Spawn-supported types**: [count from statistics]
- **Types with mutations**: [count from statistics]

### Comparison with Baseline:
[Present the structured comparison output directly - it's already formatted for user consumption]

### Baseline Promotion Decision
Based on the comparison results above, should I mark this version as the new good baseline?

## Available Actions
- **promote** - Mark this version as the new good baseline
- **skip** - Keep existing baseline, don't promote this version
- **investigate** - Launch deeper investigation of the differences

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
