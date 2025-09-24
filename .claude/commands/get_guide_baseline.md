# Get Type Guide (Baseline)

Gets the baseline type guide for a specified type from the baseline file. Optionally filters to a specific mutation path.

## Command Execution

<ParseArguments>
Parse the type name from ${ARGUMENTS}:
- If ${ARGUMENTS} is empty, the script will list all available types in the baseline
- If type name is not found, the script will show "Type not found" with suggestions
- If multiple types match (e.g., "Transform"), the script will show disambiguation options
- If a mutation path is provided as a second argument, validate it exists and extract it for filtering
- Invalid mutation paths will result in clear error messages from the script
</ParseArguments>

<ExecuteScript>
Inform the user "Loading type guide from baseline..." then run the script at .claude/scripts/get_type_guide.sh or .claude/scripts/get_mutation_path.sh to retrieve the type guide data from the baseline file at .claude/transient/all_types_baseline.json
</ExecuteScript>

<FormatOutput>
Inform the user "Formatting output..." then format the retrieved data as proper JSON with syntax highlighting for readability.
</FormatOutput>

<DisplayResults>
Present the output using the UserOutput template with the type name and optional mutation path in the header.
</DisplayResults>

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <ParseArguments/>
**STEP 2:** Execute <ExecuteScript/>
**STEP 3:** Execute <FormatOutput/>
**STEP 4:** Execute <DisplayResults/>
</ExecutionSteps>

## Usage

### Get Complete Type Guide
Get all mutation paths and type information for a type:

```bash
/get_guide Transform
/get_guide bevy_transform::components::transform::Transform
/get_guide Bloom
```

### Get Specific Mutation Path
Get details for a specific mutation path only:

```bash
/get_guide Bloom .composite_mode
/get_guide Transform .translation
/get_guide Node .grid_template_columns[0].tracks
```

<UserOutput>
## Type Guide for ${TYPE_NAME} [optional: at path ${MUTATION_PATH}]

```json
${JSON_OUTPUT}
```
</UserOutput>

## Features

- **Baseline version**: Gets type guide from the baseline file, not a running app
- **Short name support**: Use just the type name (e.g., Transform) or full path
- **Complete mutation paths**: Shows all available mutation paths for the type
- **Path filtering**: Optional second argument to show only a specific mutation path
- **Supported operations**: Lists which BRP operations work with the type
- **Schema information**: Includes type structure and field information

## Output Format

### Full Type Guide (no path specified)
Displays comprehensive JSON formatted output showing:

```json
{
  "type_name": "full::type::path",
  "has_serialize": bool,
  "has_deserialize": bool,
  "in_registry": bool,
  "supported_operations": [...],
  "mutation_paths": {
    "": { /* root mutation */ },
    ".field": { /* field mutations */ }
  },
  "schema_info": { /* type structure */ }
}
```

### Specific Mutation Path (with path argument)
Displays only the requested mutation path:

```json
{
  "type": "full::type::path",
  "path": ".requested.path",
  "data": {
    "description": "...",
    "example": {...},
    "path_info": {...}
  }
}
```

## Prerequisites

- Baseline file at `.claude/transient/all_types_baseline.json`
- Python 3 must be installed

## Script Locations

```bash
# For getting full type guide:
.claude/scripts/get_type_guide.sh

# For getting specific mutation path:
.claude/scripts/get_mutation_path.sh
```

## Examples

### Example 1: Simple type name
```bash
/get_guide Transform
```

### Example 2: Full type path
```bash
/get_guide bevy_core_pipeline::bloom::settings::Bloom
```

### Example 3: Specific mutation path
```bash
/get_guide Bloom .composite_mode
```

## Notes

- This gets the type guide from the baseline file, not a running app
- For live/current version, use get_guide_current.md instead
- When a mutation path is provided, only that specific path's data is shown
- The root mutation path is represented by an empty string `""`
- If the type is not in the baseline, it won't appear in the guide

ARGUMENTS: ${ARGUMENTS}
