# Create Mutation Test JSON File

## Purpose
This command creates the mutation test tracking file (`$TMPDIR/all_types.json`) by:
1. Launching the extras_plugin example app
2. Getting the list of all registered component types via BRP
3. Creating a fresh tracking file with all types marked as "untested"
4. Discovering spawn support and mutation paths for ALL types systematically

This will create a fresh `all_types.json` file in `$TMPDIR` containing all currently registered types ready for testing.

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

### 3. Get all type schemas

Call `brp_all_type_schemas` to get schemas for all registered types in one operation:
```bash
mcp__brp__brp_all_type_schemas(port=22222)
```

This automatically discovers all registered types and returns their schemas. The tool will save its result to a file and return the filepath (e.g., `/var/folders/.../mcp_response_brp_all_type_schemas_12345.json`).


### 4. Transform the result with the shell script

Execute the transformation script with the exclusions file:

```bash
./test-app/tests/transform_brp_response.sh FILEPATH $TMPDIR/all_types.json
```

Replace `FILEPATH` with the actual path from step 3 (e.g., `/var/folders/.../mcp_response_brp_all_type_schemas_12345.json`).

The script creates `$TMPDIR/all_types.json` with all discovered types initialized with `batch_number: null`.

### 5. Verify final file structure
The completed file is structured as a JSON array of type objects (not an object with type names as keys).

Each array element contains a type object with the structure:
```json
{
  "type": "fully::qualified::TypeName",
  "spawn_support": "supported" | "not_supported", 
  "mutation_paths": ["array", "of", "mutation", "paths"],
  "test_status": "untested" | "passed",
  "batch_number": null,
  "fail_reason": ""
}
```

The completed file should have:
- All types with spawn support properly identified (`"supported"` or `"not_supported"`)
- All types with mutation paths listed as arrays
- All types starting with `test_status: "untested"` (except auto-passed spawn types)
- All types starting with `batch_number: null` (batch assignment done separately)

Types that support spawn typically have:
- `has_deserialize: true` and `has_serialize: true` in the BRP response
- A `spawn_format` field in the BRP response
- `["query", "get", "mutate", "spawn", "insert"]` in supported_operations

### 6. Report results
```bash
# Generate summary statistics using the stats script
./test-app/tests/type_stats.sh $TMPDIR/all_types.json
```

This script provides comprehensive statistics including capability summary, test status, batch information, and progress tracking.

### 7. Cleanup
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
3. **Single output file** - Only create/modify `$TMPDIR/all_types.json`
4. **Use actual BRP responses** - Base spawn support and mutation paths on actual BRP discovery
5. **Execute shell scripts** - Use the provided `transform_brp_response.sh` and `type_stats.sh` scripts

## Expected Results

- Spawn-supported types: Types with Serialize/Deserialize traits (Name, Transform, Node, Window, BackgroundColor, test components, etc.)
- Non-spawn types: Most rendering/internal components (Sprite, Camera components, visibility components, etc.)
- All types should have their actual mutation paths populated as arrays
- All types start with `test_status: "untested"` and empty `fail_reason`
