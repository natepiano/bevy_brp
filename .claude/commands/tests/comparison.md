# Tool Comparison Test

## Objective
Direct comparison between `brp_type_schema` (local implementation) and `brp_extras_discover_format` (extras plugin) to validate parity.

## Prerequisites
- Launch extras_plugin example on port 15702
- Both tools must be available in the MCP environment
- Clean shutdown at the end

## Test Steps

### 1. Launch Test Application
- Execute `mcp__brp__brp_launch_bevy_app` with `extras_plugin` on port 15702
- Verify app is running with `mcp__brp__brp_status`

### 2. Run Both Discovery Tools

#### 2a. Run Extras Discovery
Execute `mcp__brp__brp_extras_discover_format` with:
```json
{
  "types": ["bevy_transform::components::transform::Transform"],
  "port": 15702
}
```
Store the full response as `extras_response`.

#### 2b. Run Local Type Schema Discovery
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["bevy_transform::components::transform::Transform"],
  "port": 15702
}
```
Store the full response as `local_response`.

### 3. Compare Results

Compare the `result.type_info` objects from both responses:

#### Required Field Validation
- Both responses must contain `type_info` object
- Both must have entry for `bevy_transform::components::transform::Transform`
- All the following fields must be present and match exactly:
  - `type_name`: Full type path string
  - `type_category`: Should be "Struct"
  - `in_registry`: Should be `true`
  - `has_serialize`: Should be `true`
  - `has_deserialize`: Should be `true`
  - `supported_operations`: Array with ["query", "get", "spawn", "insert", "mutate"]
  - `mutation_paths`: Object with 13 entries (.translation, .translation.x/y/z, .rotation, .rotation.x/y/z/w, .scale, .scale.x/y/z)
  - `example_values.spawn`: Object with translation/rotation/scale arrays
  - `enum_info`: Should be `null` for Transform
  - `error`: Should be `null` for successful discovery

### 4. Extended Multi-Type Test

Run both tools with multiple types:
```json
{
  "types": [
    "bevy_transform::components::transform::Transform",
    "bevy_core::name::Name",
    "bevy_sprite::sprite::Sprite"
  ],
  "port": 15702
}
```

Validate:
- Both return same number of types in `type_info`
- Each type has identical structure between tools
- Marker components (no properties) handled correctly
- Complex components with many fields work

### 5. Cleanup
- Execute `mcp__brp__brp_shutdown` to stop the test app
- Verify clean shutdown

## Success Criteria

✅ Test passes when:
- `extras_response.result.type_info` === `local_response.result.type_info` (deep equality)
- No missing fields in local implementation
- All spawn formats match exactly
- Mutation paths identical (same paths and descriptions)
- Supported operations arrays equal
- Multi-type batch processing works identically

## Failure Investigation

If differences found:
1. Log the specific field path that differs
2. Show the values from both tools side-by-side
3. Check if it's a formatting difference vs actual data difference
4. Verify cache was populated before local tool ran
