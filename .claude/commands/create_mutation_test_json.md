# Create Mutation Test JSON File

**CRITICAL** before doing anything else, read the tagged sections below and use them where referenced.

**DIRECTORY VALIDATION**: Ensure you are in the correct working directory before starting:
```bash
.claude/scripts/create_mutation_test_json/validate_directory.sh
```

<CreateContext>
TARGET_FILE = .claude/transient/all_types.json
PURPOSE = Creates a FRESH mutation test tracking file by discovering all registered component types via BRP. This ALWAYS creates a new file - test metadata will be initialized by prepare.py when needed.
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

Create a todo list with the following 3 items:
1. "Fetch type guides with fetch_type_guides.py" (pending → in_progress when starting STEP 1)
2. "Compare against baseline" (pending → in_progress when starting STEP 2)
3. "Present results and get user decision on baseline promotion" (pending → in_progress when starting STEP 3)

Mark each todo as "in_progress" when beginning that step, and "completed" when the step finishes successfully.

<ExecutionSteps>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Execute the <FetchTypeGuides/>
    **STEP 2:** Execute the <Comparison/>
    **STEP 3:** Execute the <UserValidation/> → **STOP** and present final summary
</ExecutionSteps>

## STEP 1: FETCH TYPE GUIDES

<FetchTypeGuides>
    Run in the background (`run_in_background: true`) and wait for the completion notification:
    ```bash
    python3 .claude/scripts/mutation_test/fetch_type_guides.py
    ```

    It builds bevy_brp_mcp from this checkout, launches extras_plugin on port 22222, calls `brp_all_type_guides`, writes a FRESH ${TARGET_FILE} (BRP data only; prepare.py initializes test metadata), and shuts the app down.

    **VALIDATION**: stdout JSON has `status` "success" and a `type_guides` count. On `status` "error", report `message` and stop.

    **Fix loop**: fix the code → rerun this script → STEP 2.
</FetchTypeGuides>

## STEP 2: COMPARISON

<Comparison>
    ```bash
    python3 .claude/scripts/create_mutation_test_json/compare.py .claude/transient/all_types_baseline.json ${TARGET_FILE}
    ```

    **Comparison Output**: The compare.py script displays:
    - Current file statistics (total types, spawn-supported types, types with mutations, total mutation paths)
    - Comparison results (total changes, types modified/added/removed)
    - `✅ No changes detected!` or `⚠️ CHANGES DETECTED: X changes`

    **VALIDATION**: Confirm the comparison completed successfully.
</Comparison>

## STEP 3: USER VALIDATION

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
**Direct tool usage only** - Use only the provided scripts
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
       - Return to main decision prompt from Step 3
</ComparisonReviewWorkflow>

<PatternOverviewFormat>
## Pattern [NUMBER]: [PATTERN_NAME]

**Occurrences**: [COUNT] changes across [TYPE_COUNT] types
**What this means**: [EXPLANATION]

Retrieving first example - [TYPE_NAME] at path [PATH]:
</PatternOverviewFormat>
