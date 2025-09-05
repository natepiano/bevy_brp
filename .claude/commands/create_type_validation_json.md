# Initialize Type Validation Tracking File

## Purpose
This command initializes or reinitializes the type validation tracking file (`test-app/tests/all_types.json`) by:
1. Launching the extras_plugin example app
2. Getting the list of all registered component types via BRP
3. Creating a fresh tracking file with all types marked as "untested"
4. Discovering spawn support and mutation paths for ALL types systematically

## Usage
```
/init_type_validation
```

This will overwrite any existing `all_types.json` file with a fresh one containing all currently registered types ready for testing.

## Execution Steps

### 1. Launch the extras_plugin app
```bash
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=22222
)
```

### 2. Verify BRP connectivity
```bash
mcp__brp__brp_status(
    app_name="extras_plugin",
    port=22222
)
```

Wait for confirmation that BRP is responding before proceeding.

### 3. Get list of all component types
```bash
result = mcp__brp__bevy_list(port=22222)
```

This returns an array of all registered component type names.

### 4. Call brp_type_schema with ALL types

Call `mcp__brp__brp_type_schema` with ALL types from step 3:
```bash
mcp__brp__brp_type_schema(
    types=<all_types_from_step_3>,
    port=22222
)
```

The tool will automatically save its result to a file and return the filepath (e.g., `/var/folders/.../mcp_response_brp_type_schema_12345.json`).


### 5. Transform the result with the shell script

Execute the transformation script with the exclusions file:

```bash
./test-app/tests/transform_brp_response.sh FILEPATH test-app/tests/all_types.json test-app/tests/excluded_types.txt
```

Replace `FILEPATH` with the actual path from step 4 (e.g., `/var/folders/.../mcp_response_brp_type_schema_12345.json`).

The script handles exclusion filtering using `excluded_types.txt` and creates `test-app/tests/all_types.json` with all types initialized with `batch_number: null`.

### 6. Verify final file structure
The completed file should have:
- All types with spawn support properly identified (`"supported"` or `"not_supported"`)
- All types with mutation paths listed as arrays
- All types starting with `test_status: "untested"`
- All types starting with `batch_number: null` (batch assignment done separately)

Types that support spawn typically have:
- `has_deserialize: true` and `has_serialize: true` in the BRP response
- A `spawn_format` field in the BRP response
- `["query", "get", "mutate", "spawn", "insert"]` in supported_operations

### 7. Report results
```bash
# Generate summary statistics using the stats script
./test-app/tests/type_stats.sh test-app/tests/all_types.json
```

This script provides comprehensive statistics including capability summary, test status, batch information, and progress tracking.

### 9. Cleanup
Shutdown the app:
```bash
mcp__brp__brp_shutdown(
    app_name="extras_plugin",
    port=22222
)
```

## Critical Success Factors

1. **NO intermediate files** - Do NOT create Python scripts, temp files, or any other files
2. **Direct tool usage only** - Use only MCP tools and the provided shell scripts
3. **Single output file** - Only create/modify `test-app/tests/all_types.json`
4. **Use actual BRP responses** - Base spawn support and mutation paths on actual BRP discovery
5. **Execute shell scripts** - Use the provided `transform_brp_response.sh` and `type_stats.sh` scripts

## Expected Results

- Spawn-supported types: Types with Serialize/Deserialize traits (Name, Transform, Node, Window, BackgroundColor, test components, etc.)
- Non-spawn types: Most rendering/internal components (Sprite, Camera components, visibility components, etc.)
- All types should have their actual mutation paths populated as arrays
- All types start with `test_status: "untested"` and empty `fail_reason`
