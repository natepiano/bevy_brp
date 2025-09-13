# Compare Mutation Path

Compares a specific mutation path between the baseline and current running app to identify changes.

<InstallWarning>
## IMPORTANT NOTE ##
If you have recently made changes and haven't intalled it, remember you need to ask the user to install it. If you haven't made changes in this session without installing, then you can ignore this and continue to the rest of the instructions without commenting on this
</InstallWarning>

## Command Execution

When comparing a mutation path, I will:

1. Launch extras_plugin example on port 23456
2. Verify BRP connectivity and set window title
3. Get the type guide from the running app
4. Get the type guide from the baseline
5. Run the comparison script to show both versions
6. Present the raw JSON and summarize differences
7. Shutdown the app

## Usage

Provide the type name and mutation path as arguments:

```bash
# Format: TYPE_NAME MUTATION_PATH
bevy_ui::ui_node::Node .grid_template_columns
bevy_ui::ui_node::Node .grid_template_columns[0].tracks
```

## Execution Steps

1. **Launch App**:
```bash
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=23456
)
```

2. **Verify BRP Connectivity**:
```bash
mcp__brp__brp_status(
    app_name="extras_plugin",
    port=23456
)
```

3. **Set Window Title**:
```bash
mcp__brp__brp_extras_set_window_title(
    port=23456,
    title="compare_mutation_path - port 23456"
)
```

4. **Get Current Type Guide**:
```bash
mcp__brp__brp_type_guide(
    types=["TYPE_NAME"],
    port=23456
)
```

5. **Get Baseline Type Guide**:
```bash
.claude/commands/scripts/get_type_guide.sh TYPE_NAME
```

6. **Compare and Present Results**:
**DO NOT create temporary files or run comparison scripts**. Instead:
- Extract the specific mutation path from both the current BRP response and baseline response
- Present both JSON structures side-by-side
- Analyze and summarize the differences directly

7. **Shutdown App**:
```bash
mcp__brp__brp_shutdown(
    app_name="extras_plugin",
    port=23456
)
```

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
.claude/commands/scripts/create_mutation_test_json_get_excluded_types.sh
```

❌ **WRONG**: Do NOT store script outputs in environment variables:
```bash
EXCLUDED_TYPES=$(.claude/commands/scripts/create_mutation_test_json_get_excluded_types.sh)
```

The user must approve environment variable assignments, which interrupts the workflow.

ARGUMENTS: $ARGUMENTS
