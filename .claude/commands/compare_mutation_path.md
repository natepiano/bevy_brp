# Compare Mutation Path

Compares a specific mutation path between the baseline and current running app to identify changes.

## Usage

Provide the type name and mutation path as arguments:

```bash
# Format: TYPE_NAME MUTATION_PATH
bevy_ui::ui_node::Node .grid_template_columns
bevy_ui::ui_node::Node .grid_template_columns[0].tracks
```

## Argument Processing

Parse and validate the required arguments:

```bash
.claude/scripts/compare_mutation_path_validate_args.sh $ARGUMENTS
```

This script will:
- Extract TYPE_NAME and MUTATION_PATH from $ARGUMENTS
- Validate both arguments are provided
- Validate TYPE_NAME format (contains :: namespace separators)
- Exit with error message if validation fails
- Output validated arguments for use in execution steps

## Argument Processing Output

After successful validation, output the parsed arguments:
"Processing comparison for type: `${TYPE_NAME}`, mutation path: `${MUTATION_PATH}`"

## Configuration Note

The `extras_plugin` example and port `23456` are intentionally hardcoded throughout this command to ensure consistent testing environment and avoid port conflicts with other running BRP instances.

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <LaunchApp/>
**STEP 2:** Execute <VerifyBrpConnectivity/>
**STEP 3:** Execute <SetWindowTitle/>
**STEP 4:** Execute <GetCurrentTypeGuide/>
**STEP 5:** Execute <GetBaselineTypeGuide/>
**STEP 6:** Execute <CompareAndPresentResults/>
**STEP 7:** Execute <ShutdownApp/>
</ExecutionSteps>

## STEP 1: LAUNCH APP

<LaunchApp>
Launch extras_plugin example:

```bash
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=23456
)
```
</LaunchApp>

## STEP 2: VERIFY BRP CONNECTIVITY

<VerifyBrpConnectivity>
Verify BRP connectivity:

```bash
mcp__brp__brp_status(
    app_name="extras_plugin",
    port=23456
)
```
</VerifyBrpConnectivity>

## STEP 3: SET WINDOW TITLE

<SetWindowTitle>
Set window title for identification:

```bash
mcp__brp__brp_extras_set_window_title(
    port=23456,
    title="compare_mutation_path - port 23456"
)
```
</SetWindowTitle>

## STEP 4: GET CURRENT TYPE GUIDE

<GetCurrentTypeGuide>
Get type guide from running app:

```bash
mcp__brp__brp_type_guide(
    types=["TYPE_NAME"],
    port=23456
)
```
</GetCurrentTypeGuide>

## STEP 5: GET BASELINE TYPE GUIDE

<GetBaselineTypeGuide>
Get baseline type guide using the filepath returned from STEP 4:

```bash
.claude/scripts/get_type_guide.sh TYPE_NAME --file <filepath_from_step_4_result>
```

Replace `<filepath_from_step_4_result>` with the actual filepath returned in the `result.filepath` field from STEP 4.
</GetBaselineTypeGuide>

## STEP 6: COMPARE AND PRESENT RESULTS

<CompareAndPresentResults>
Compare and present results:

**DO NOT create temporary files or run comparison scripts**. Instead:
- Extract the specific mutation path from both the current BRP response and baseline response
- Present both JSON structures side-by-side
- Analyze and summarize the differences directly
</CompareAndPresentResults>

## STEP 7: SHUTDOWN APP

<ShutdownApp>
Shutdown the application:

```bash
mcp__brp__brp_shutdown(
    app_name="extras_plugin",
    port=23456
)
```
</ShutdownApp>

## Output

The comparison will show:

### Raw JSON Comparison
- **Baseline**: The mutation path example from baseline
- **Current**: The mutation path example from running app

### Difference Summary
- Structure changes (array length, field additions/removals)
- Value changes (maintaining same structure)
- Type changes

## Prerequisites

- extras_plugin example must be available
- Baseline file at `$TMPDIR/all_types_baseline.json`
- Port 23456 must be available

## Notes

- Mutation paths use dot notation (e.g., `.field.subfield`)
- Array elements use bracket notation (e.g., `[0]`)
- The root path is represented by an empty string `""`

## IMPORTANT BASH EXECUTION RULES

**NEVER use environment variables for script outputs** - This breaks the workflow and requires user approval.

✅ **CORRECT**: Run scripts directly and use their output immediately:
```bash
.claude/scripts/create_mutation_test_json_get_excluded_types.sh
```

❌ **WRONG**: Do NOT store script outputs in environment variables:
```bash
EXCLUDED_TYPES=$(.claude/scripts/create_mutation_test_json_get_excluded_types.sh)
```

The user must approve environment variable assignments, which interrupts the workflow.

ARGUMENTS: $ARGUMENTS
