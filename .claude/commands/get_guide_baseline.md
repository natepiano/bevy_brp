# Get Type Guide (Baseline)

Gets the baseline type guide for a specified type from the baseline file. Optionally filters to a specific mutation path.

## Command Execution

When you request a type guide, I will:

1. Run the script to get the type guide data from baseline
2. If a mutation path is provided, filter the results to show only that path
3. Display the results formatted as proper JSON with syntax highlighting
4. Present the output in a clear, readable format

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
## Type Guide for $TYPE_NAME [optional: at path $MUTATION_PATH]

```json
$JSON_OUTPUT
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

- Baseline file at `$TMPDIR/all_types_baseline.json`
- Python 3 must be installed

## Script Locations

```bash
# For getting full type guide:
.claude/commands/scripts/get_type_guide.sh

# For getting specific mutation path:
.claude/commands/scripts/get_mutation_path.sh
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

ARGUMENTS: $ARGUMENTS
