# Get Mutation Path

Retrieves mutation paths from the baseline file for a given type. Shows all paths as a formatted list when only type is provided, or specific path details when both type and path are given.

## Command Execution

When you request mutation paths, I will:

1. Run the appropriate script to get mutation path data from baseline
2. If only type is provided: Display all paths as a formatted bullet list
3. If type and path are provided: Display the specific path's JSON details
4. Present the output in a clear, readable format

<UserOutput>
## For type only:
## Mutation Paths for $TYPE_NAME

**Total paths:** $COUNT

- `""` (root)
- `.field1`
- `.field2`
- ... (all paths)

## For type + path:
## Mutation Path for $TYPE_NAME $PATH

```json
$JSON_OUTPUT
```
</UserOutput>

## Usage

### List All Mutation Paths for a Type
Get a formatted bullet list of all available mutation paths:

```bash
/get_path Transform
/get_path bevy_transform::components::transform::Transform
/get_path BoxShadow
```

### Get Specific Mutation Path
Retrieve JSON details for a specific mutation path:

```bash
/get_path BoxShadow .0[0].color
/get_path Transform .translation
/get_path Node .grid_template_columns[0].tracks
```

## Features

- **Short name support**: Use just the type name (e.g., Transform) or full path
- **Case-insensitive matching**: transform matches Transform
- **Disambiguation**: When multiple types share the same short name, shows a numbered list
- **Dual mode**: Lists all paths when only type provided, shows specific path details with both arguments
- **Formatted output**: Bullet list for all paths, JSON for specific path
- **Error handling**: Clear messages when type or path not found
- **No quotes required**: Arguments don't need quotes unless they contain spaces

## Output Format

### When Listing All Paths (type only)
Shows a formatted bullet list with:
- Type name (full path)
- Total number of mutation paths
- All paths as bullet points
- Root path labeled as "(root)" for clarity

### When Getting Specific Path
Displays JSON formatted output showing:

```json
{
  "type": "full::type::path",
  "path": ".mutation.path",
  "data": {
    "description": "...",
    "example": {...},
    "path_info": {...},
    // other fields as present
  }
}
```

The JSON will be properly formatted with:
- Syntax highlighting
- Proper indentation
- All fields from the mutation path data

## Script Locations

```bash
# For listing all paths:
.claude/commands/scripts/get_mutation_path_list.sh

# For getting specific path:
.claude/commands/scripts/get_mutation_path.sh
```

## Prerequisites

- Baseline file at `$TMPDIR/all_types_baseline.json`
- Python 3 must be installed

## Examples

### Example 1: List all paths for a type
```bash
/get_path Node
```

### Example 2: Get root path details for a type
```bash
/get_path Transform ""
```

### Example 3: Get nested path details
```bash
/get_path Node .grid_template_columns[0].tracks
```

## Notes

- The root mutation path is represented by an empty string `""`
- Array indices use bracket notation: `[0]`, `[1]`, etc.
- Nested fields use dot notation: `.field.subfield`
- The script reads from the baseline file, not the current/live version
- Quotes are optional unless the argument contains spaces

ARGUMENTS: $ARGUMENTS
