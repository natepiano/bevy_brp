# Get Type Guide

<InstallWarning>
## IMPORTANT NOTE ##
If you have recently made changes and haven't intalled it, then you need to install it according to the instructions in ./~claude/commands/build_and_install.md

You can ignore this if no changes have been made.
</InstallWarning>

Retrieves the complete type guide for a specific type from the baseline file.

## Command Execution

When you request a type guide, I will:

1. Run the script to get the raw JSON data
2. Process and format the output with proper markdown
3. Display a summary with key information

## Usage

### List All Types (no arguments)
Get a list of all available types:

```bash
.claude/commands/scripts/get_type_guide.sh
```

This will display a two-column table showing short names and full paths, sorted by short name.

### Basic Usage (short name)
Get type guide using the short type name (case-insensitive):

```bash
.claude/commands/scripts/get_type_guide.sh Transform
.claude/commands/scripts/get_type_guide.sh transform  # case-insensitive
```

### Full Path Usage
Get type guide using the full type path:

```bash
.claude/commands/scripts/get_type_guide.sh bevy_transform::components::transform::Transform
```

### Handling Multiple Matches
When multiple types share the same short name, the script will show a numbered list:

```
Found 2 types matching "Monitor":

1. bevy_window::monitor::Monitor
2. extras_plugin::Monitor

Please run the command again with either:
  - The number of your choice (1, 2, 3, etc.)
  - The full type path
```

## Output Processing

The script returns JSON data that I will process to show:

1. **Type name**
2. **Full Type Guide** in formatted JSON
3. **Mutation Paths Summary**
4. **Spawn Format Example** (if available)

### Full Type Guide
Display the ENTIRE contents of the "guide" field from the JSON response, formatted as JSON:

Example - show the complete guide object:
```json
{
  "has_deserialize": false,
  "has_serialize": false,
  "in_registry": true,
  "mutation_paths": {...},
  "schema_info": {...},
  "supported_operations": [...],
  "type_name": "...",
  "batch_number": null,
  "test_status": "...",
  "fail_reason": ""
}
```

## Script Execution

Run the script with the requested type:

```bash
.claude/commands/scripts/get_type_guide.sh $ARGUMENTS
```

Then process the JSON output to display:
- If status is "list_all": Format as a compact list with arrow separators
- If status is "found": Format and display the type guide
- If multiple matches: Show the list for disambiguation
- If no matches: Report "No type was found"

When listing all types, format as:
```
ShortName                       → full::path::to::Type
```

## Prerequisites

- Requires baseline file at `$TMPDIR/all_types_baseline.json`
- Python 3 must be installed
- The baseline file must have the expected structure with `type_guide` array

## Notes

- Short names match the last segment after `::` (e.g., "Transform" matches "bevy_transform::components::transform::Transform")
- Matching is case-insensitive for convenience
- Full paths can be used to disambiguate when multiple types share the same short name
- The spawn format example shows the JSON structure needed for `bevy/spawn` operations

ARGUMENTS: $ARGUMENTS
