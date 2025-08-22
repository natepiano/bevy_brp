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
- `type_category` === "Struct"
- `in_registry` === true
- `has_serialize` === false
- `has_deserialize` === false
- `supported_operations` array contains exactly ["query", "get"]

### 4. Validate Mutation Paths

#### 4a. Check All Paths Present
Verify `mutation_paths` object contains exactly these 11 keys:
- `.anchor`
- `.color`
- `.custom_size`
- `.flip_x`
- `.flip_y`
- `.image`
- `.image_mode`
- `.rect`
- `.texture_atlas`
- `.custom_size.x`
- `.custom_size.y`

#### 4b. Validate Enum Fields
Check enum fields have correct variants:

**`.anchor`**:
- `enum_variants` contains: ["Center", "BottomLeft", "BottomCenter", "BottomRight", "CenterLeft", "CenterRight", "TopLeft", "TopCenter", "TopRight", "Custom"]
- `example` === "Center"

**`.color`**:
- `enum_variants` contains: ["Srgba", "LinearRgba", "Hsla", "Hsva", "Hwba", "Laba", "Lcha", "Oklaba", "Oklcha", "Xyza"]
- `example` is object with "Srgba" key

**`.image_mode`**:
- `enum_variants` contains: ["Auto", "Scale", "Sliced", "Tiled"]
- `example` === "Auto"

#### 4c. Validate Option Fields
Check Option fields have both examples:

**`.custom_size`**:
- Has `example_some` field (array with 2 numbers)
- Has `example_none` field (null)
- Has `note` about Option field handling

**`.rect`**:
- Has `example_some` field (object with min/max arrays)
- Has `example_none` field (null)
- Has `note` about Option field handling

**`.texture_atlas`**:
- Has `example_some` field (object)
- Has `example_none` field (null)
- Has `note` about Option field handling

#### 4d. Validate Simple Fields
**`.flip_x`** and **`.flip_y`**:
- `example` === true
- `type` === "bool"

**`.image`**:
- `example` is object with "Strong" key containing array
- `type` === "bevy_asset::handle::Handle<bevy_image::image::Image>"

### 5. Multi-Type Discovery Test

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
- Transform has mutation paths for translation/rotation/scale
- Name has appropriate fields for a wrapper type
- Each type has correct `type_category`

### 6. Cache Refresh Validation

This test validates that the cache refresh parameter works correctly by switching between apps with different type registrations. The cache for port 20114 is already populated with the `extras_plugin` registry from earlier tests.

#### 6a. Switch to App WITHOUT Custom Types
- Shutdown current `extras_plugin` with `mcp__brp__brp_shutdown`
- Verify shutdown with status "clean_shutdown"
- Launch `no_extras_plugin` example on port 20114
- Verify app is running with `mcp__brp__brp_status`

#### 6b. Query Custom Type WITHOUT Cache Refresh (Should Use Stale Cache)
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["TestStructWithSerDe"],
  "port": 20114,
  "refresh_cache": false
}
```
- **Expected**: Type IS found (using stale cache from extras_plugin)
- **Validate**: 
  - Response succeeds
  - `result.type_info["TestStructWithSerDe"]` exists
  - `result.type_info["TestStructWithSerDe"].in_registry` === true
- This proves the cache is being used despite the app change

#### 6c. Query Custom Type WITH Cache Refresh (Should Get Current Registry)
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["TestStructWithSerDe"],
  "port": 20114,
  "refresh_cache": true
}
```
- **Expected**: Type NOT found (no_extras_plugin doesn't have this type)
- **Validate**:
  - Response succeeds but type not in registry
  - `result.type_info["TestStructWithSerDe"].in_registry` === false
  - `result.type_info["TestStructWithSerDe"].error` contains "not found" or similar message
- This proves refresh_cache forces a fresh registry fetch

#### 6d. Verify Standard Types Still Work After Refresh
Execute `mcp__brp__brp_type_schema` with:
```json
{
  "types": ["bevy_transform::components::transform::Transform"],
  "port": 20114,
  "refresh_cache": false
}
```
- **Validate**: Transform type is found (exists in both apps and cache was refreshed)

### 7. Final Cleanup
- Execute `mcp__brp__brp_shutdown` to stop the test app
- Verify clean shutdown with status "process_kill" (no_extras_plugin doesn't have BrpExtrasPlugin)

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
