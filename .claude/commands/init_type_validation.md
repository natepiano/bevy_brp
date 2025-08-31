# Initialize Type Validation Tracking File

## Purpose
This command initializes or reinitializes the type validation tracking file (`.claude/commands/type_validation.json`) by:
1. Launching the extras_plugin example app
2. Getting the list of all registered component types via BRP
3. Creating a fresh tracking file with all types marked as "untested"

## Usage
```
/init_type_validation
```

This will overwrite any existing `type_validation.json` file with a fresh one containing all currently registered types.

## Execution Steps

### 1. Launch the extras_plugin app
```bash
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=20116
)
```

### 2. Verify BRP connectivity
```bash
mcp__brp__brp_status(
    app_name="extras_plugin",
    port=20116
)
```

Wait for confirmation that BRP is responding before proceeding.

### 3. Get list of all component types
```bash
result = mcp__brp__bevy_list(port=20116)
```

This returns an array of all registered component type names.

### 4. Create the tracking file
**IMPORTANT: Use the Write tool to create a new file. Do NOT use the Edit tool on any existing file.**

Write a new `.claude/commands/type_validation.json` file with the following structure:

```python
import json

# Get the component list from step 3
components = result["result"]  # Array of type names

# Build the tracking structure
validation_data = []
for component_type in components:
    validation_data.append({
        "type": component_type,
        "spawn_test": "untested",
        "mutation_tests": "untested",
        "notes": ""
    })

# Use the Write tool to create a fresh file (this will overwrite any existing file)
# DO NOT use Edit tool - always create a new file with Write
with open('.claude/commands/type_validation.json', 'w') as f:
    json.dump(validation_data, f, indent=2)
```

### 5. Report results
```
✅ Initialized type validation tracking file
- Total types: [count]
- File location: .claude/commands/type_validation.json
- All types marked as "untested"
```

### 6. Cleanup (optional)
If you don't need the app running after initialization:
```bash
mcp__brp__brp_shutdown(
    app_name="extras_plugin",
    port=20116
)
```

## Important Notes

- **Port**: Uses port 20116 by default (same as type_validation test)
- **Overwrites**: This command will overwrite any existing tracking file by using the Write tool to create a completely new file
- **Fresh start**: All types will be marked as "untested" regardless of previous test results
- **Component discovery**: Only components registered with BRP reflection will be included
- **File Creation**: ALWAYS use the Write tool to create a new file. NEVER use the Edit tool to modify an existing type_validation.json file

## Error Handling

If the app fails to launch:
- Check if port 20116 is already in use
- Ensure the extras_plugin example is built

If BRP doesn't respond:
- Verify the app includes the RemotePlugin
- Check that the app launched successfully

## Example Output

After running this command, the file will contain:
```json
[
  {
    "type": "bevy_core_pipeline::bloom::settings::Bloom",
    "spawn_test": "untested",
    "mutation_tests": "untested",
    "notes": ""
  },
  {
    "type": "bevy_core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpening",
    "spawn_test": "untested",
    "mutation_tests": "untested",
    "notes": ""
  },
  // ... all other types ...
]
```