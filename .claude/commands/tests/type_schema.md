# Type Schema Validation Test

## Objective
Validate that `brp_type_schema` tool correctly discovers type information and produces the expected output structure for Bevy components.

## Prerequisites
- Launch extras_plugin example on port 20114
- Tool `brp_type_schema` must be available in the MCP environment
- Expected output file at `.claude/commands/expected_sprite.json`

## Test Steps

### 1. Launch Test Application
- Execute `mcp__brp__brp_launch_bevy_example` with `extras_plugin` on port 20114
- Verify app is running with `mcp__brp__brp_status`

### 2. Run Type Schema Discovery

Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["bevy_sprite::sprite::Sprite"],
  "port": 20114
}
```
Store the response as `schema_response`.

### 3. Validate Response Structure

#### 3a. Check Top-Level Fields
Verify the response contains:
- `status` === "success"
- `result` object exists
- `result.type_info` object exists
- `result.discovered_count` === 1
- `result.summary` object with:
  - `successful_discoveries` === 1
  - `failed_discoveries` === 0
  - `total_requested` === 1

#### 3b. Validate Type Info
Verify `result.type_info["bevy_sprite::sprite::Sprite"]` contains:
- `type_name` === "bevy_sprite::sprite::Sprite"
- `in_registry` === true
- `has_serialize` === false
- `has_deserialize` === false
- `supported_operations` array contains exactly ["query", "get"]

### 4. Validate Sprite Has No Mutation Paths (No Serialize Trait)

#### 4a. Verify Mutation Paths Are Absent
For Sprite (which lacks Serialize trait):
- Verify `mutation_paths` field does NOT exist in the response
- The field should be completely absent, not just empty

#### 4b. Verify Spawn Format Is Absent
- Verify `spawn_format` field does NOT exist in the response
- The field should be completely absent since Sprite cannot be spawned/inserted

### 5. Validate Schema Info for Sprite

#### 5a. Check Schema Info Exists
Verify `result.type_info["bevy_sprite::sprite::Sprite"].schema_info` exists and contains:
- `type_kind` === "Struct"
- `properties` object with field definitions
- `required` array
- `module_path` === "bevy_sprite::sprite"
- `crate_name` === "bevy_sprite"

#### 5b. Validate Required Fields
Verify `schema_info.required` contains exactly:
- "image"
- "color"
- "flip_x"
- "flip_y"
- "anchor"
- "image_mode"

#### 5c. Validate Properties Structure
Verify `schema_info.properties` contains these fields:
- `anchor` with type reference
- `color` with type reference
- `custom_size` with type reference
- `flip_x` with type reference
- `flip_y` with type reference
- `image` with type reference
- `image_mode` with type reference
- `rect` with type reference
- `texture_atlas` with type reference

### 6. Multi-Type Discovery Test

Execute `mcp__brp__brp_type_schema` with multiple types:
```json
{
  "types": [
    "bevy_transform::components::transform::Transform",
    "bevy_ecs::name::Name",
    "bevy_sprite::sprite::Sprite"
  ],
  "port": 20114
}
```

Validate:
- `result.discovered_count` === 3
- All three types present in `result.type_info`
- Transform:
  - Has `mutation_paths` for translation/rotation/scale with all subfields
  - Has `spawn_format` with example values for translation, rotation, scale
  - Has `schema_info` with properties and required fields
- Sprite:
  - Does NOT have `mutation_paths` field
  - Does NOT have `spawn_format` field
  - Has `schema_info` with properties and required fields
- Name has appropriate fields for a wrapper type
- Each type has correct `type_kind` in its `schema_info`

### 7. Cache Refresh Validation

This test validates that the cache refresh parameter works correctly by switching between apps with different type registrations. The cache for port 20114 is already populated with the `extras_plugin` registry from earlier tests.

#### 7a. Switch to App WITHOUT Custom Types
- Shutdown current `extras_plugin` with `mcp__brp__brp_shutdown`
- Verify shutdown with status "clean_shutdown"
- Launch `test_extras_plugin_app` binary on port 20114
- Verify app is running with `mcp__brp__brp_status`

#### 7b. Query Custom Type WITHOUT Cache Refresh (Should Use Stale Cache)
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["extras_plugin::TestStructWithSerDe"],
  "port": 20114,
  "refresh_cache": false
}
```
- **Expected**: Type IS found (using stale cache from extras_plugin)
- **Validate**:
  - Response succeeds
  - `result.type_info["extras_plugin::TestStructWithSerDe"]` exists
  - `result.type_info["extras_plugin::TestStructWithSerDe"].in_registry` === true
- This proves the cache is being used despite the app change

#### 7c. Query Custom Type WITH Cache Refresh (Should Get Current Registry)
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["extras_plugin::TestStructWithSerDe"],
  "port": 20114,
  "refresh_cache": true
}
```
- **Expected**: Type NOT found (test_extras_plugin_app doesn't have this type)
- **Validate**:
  - Response succeeds but type not in registry
  - `result.type_info["extras_plugin::TestStructWithSerDe"].in_registry` === false
  - `result.type_info["extras_plugin::TestStructWithSerDe"].error` contains "not found" or similar message
- This proves refresh_cache forces a fresh registry fetch

#### 7d. Verify Standard Types Still Work After Refresh
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["bevy_transform::components::transform::Transform"],
  "port": 20114,
  "refresh_cache": false
}
```
- **Validate**: Transform type is found (exists in both apps and cache was refreshed)

### 8. Final Cleanup
- Execute `mcp__brp__brp_shutdown` to stop the test app
- Verify clean shutdown with status "clean_shutdown" (test_extras_plugin_app has BrpExtrasPlugin)

## Success Criteria

✅ Test passes when:
- Type schema discovery returns successfully
- All expected fields are present in the response
- Mutation paths match expected structure
- Enum variants are correctly populated
- Option types show both Some and None examples
- Multi-type discovery handles different component types correctly
- Cache refresh validation shows:
  - Cache persists across app switches (same port uses same cache)
  - `refresh_cache: false` returns stale cached data
  - `refresh_cache: true` fetches fresh registry from current app
- Tool provides comprehensive type information for BRP operations

## Failure Investigation

If test fails:
1. Check if app is running with BRP enabled
2. Verify type exists in registry
3. Compare actual vs expected mutation paths
4. Check if enum variants are being discovered
5. Verify Option type handling is correct
