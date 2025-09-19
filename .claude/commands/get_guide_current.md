# Get Type Guide (Current)

Gets the current type guide for a specified type by launching the extras_plugin example and running brp_type_guide. Optionally filters to a specific mutation path.

## Command Execution

When you request a type guide, I will:

1. Check if extras_plugin is already running (skip if I know it's still running from a previous invocation)
2. Launch extras_plugin if not running (and remember that I launched it)
3. Run brp_type_guide on the specified type
4. If a mutation path is provided, filter the results to show only that path
5. Display the results formatted as proper JSON with syntax highlighting
6. Present the output in a clear, readable format
7. Ask if you want to shutdown the app (useful if you plan to run more type guides)

<UserOutput>
## Type Guide for $TYPE_NAME [optional: at path $MUTATION_PATH]

```json
$JSON_OUTPUT
```

Would you like me to shutdown the extras_plugin app? (It will remain running if you plan to run more type guides)
</UserOutput>

## Usage

### Get Complete Type Guide
Get all mutation paths for a type:

```bash
/get_guide_current Transform
/get_guide_current bevy_transform::components::transform::Transform
/get_guide_current Bloom
```

### Get Specific Mutation Path
Get details for a specific mutation path only:

```bash
/get_guide_current Bloom .composite_mode
/get_guide_current Transform .translation
/get_guide_current Node .grid_template_columns[0].tracks
```

## Features

- **Auto-launch**: Automatically launches extras_plugin if not already running
- **Short name support**: Use just the type name (e.g., Transform) or full path
- **Complete mutation paths**: Shows all available mutation paths for the type
- **Path filtering**: Optional second argument to show only a specific mutation path
- **Supported operations**: Lists which BRP operations work with the type
- **Schema information**: Includes type structure and field information
- **Live version**: Gets the current implementation, not baseline

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

- Bevy app with BRP and extras_plugin example available
- MCP tool must be installed (use build_and_install.md if changes were made)

## Examples

### Example 1: Simple type name
```bash
/get_guide_current Transform
```

### Example 2: Full type path
```bash
/get_guide_current bevy_core_pipeline::bloom::settings::Bloom
```

### Example 3: UI type
```bash
/get_guide_current Node
```

## Notes

- This gets the CURRENT implementation from a running app
- For baseline comparison, use get_path.md instead
- The app will be kept running if you might run more type guides
- I'll remember if the app is already running to avoid unnecessary checks
- If the type is not registered with BRP, it won't appear in the guide
- When a mutation path is provided, only that specific path's data is shown
- The root mutation path is represented by an empty string `""`

ARGUMENTS: $@
