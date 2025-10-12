# Get Type Guide (Current)

EXAMPLE_NAME = extras_plugin

Gets the current type guide for a specified type by launching the ${EXAMPLE_NAME} example and running brp_type_guide. Optionally filters to a specific mutation path.

## Command Execution

<ArgumentProcessing>
Process the provided arguments:

```bash
.claude/scripts/get_guide_current_validate_args.sh $ARGUMENTS
```

This script will:
- Extract TYPE_NAME from $ARGUMENTS (required)
- Extract MUTATION_PATH from $ARGUMENTS (optional)
- Validate TYPE_NAME is provided
- Exit with error message if validation fails
- Output validated arguments for use in execution steps
</ArgumentProcessing>

<AppManagement>
Ensure ${EXAMPLE_NAME} is available:
1. Check if ${EXAMPLE_NAME} is already running (skip if known running)
2. Launch ${EXAMPLE_NAME} if not running (and remember that I launched it)
</AppManagement>

<TypeGuideExecution>
Execute type guide generation:
1. Use TodoWrite to track workflow progress for user visibility
2. Run brp_type_guide on the specified type
3. If type not found: "Type '${TYPE_NAME}' not registered with BRP. Use brp_list to see available types."
4. If mutation path provided, filter results to show only that path
5. If mutation path invalid: "Mutation path '${MUTATION_PATH}' not found for type '${TYPE_NAME}'. Available paths: [list from type guide]"
6. Display results formatted as JSON with syntax highlighting
7. Present output in clear, readable format
8. Mark current todo as "in_progress" before asking: "Would you like me to shutdown the ${EXAMPLE_NAME} app?"
9. **STOP** - Wait for user response about app shutdown
10. If user responds **yes** or **shutdown**: Execute brp_shutdown
11. If user responds **no** or **keep**: Leave app running
12. If unclear response: Ask for clarification with yes/no options
13. After handling response, mark todo as "completed"
</TypeGuideExecution>

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <ArgumentProcessing/>
**STEP 2:** Execute <AppManagement/>
**STEP 3:** Execute <TypeGuideExecution/>
</ExecutionSteps>

<UserOutput>
## Type Guide for $TYPE_NAME [optional: at path $MUTATION_PATH]

```json
$JSON_OUTPUT
```

Would you like me to shutdown the ${EXAMPLE_NAME} app? (It will remain running if you plan to run more type guides)
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

- **Short name support**: Use just the type name (e.g., Transform) or full path
- **Path filtering**: Optional second argument to show only a specific mutation path
- **Supported operations**: Lists which BRP operations work with the type
- **Schema information**: Includes type structure and field information

## Output Format

### Full Type Guide (no path specified)
Displays comprehensive JSON formatted output showing:

```json
{
  "type_name": "full::type::path",
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

- Bevy app with BRP and ${EXAMPLE_NAME} example available
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

- For baseline comparison, use get_path.md instead
- If the type is not registered with BRP, it won't appear in the guide
- When a mutation path is provided, only that specific path's data is shown
- The root mutation path is represented by an empty string `""`

ARGUMENTS: $@
