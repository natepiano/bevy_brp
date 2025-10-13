# BRP world.query Type Safety Upgrade - Bevy 0.17.2

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for user confirmation ("go ahead" or similar)

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & VALIDATE**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt
   ```

5. **CONFIRM**: Wait for user to confirm the build succeeded

6. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

7. **PROCEED**: Move to next step only after confirmation
</Instructions>

<ExecuteImplementation>
Find the next ⏳ PENDING step in the INTERACTIVE IMPLEMENTATION SEQUENCE below.

For the current step:
1. Follow the <Instructions/> above for executing the step
2. When step is complete, use Edit tool to mark it as ✅ COMPLETED
3. Continue to next PENDING step

If all steps are COMPLETED:
    Display: "✅ Implementation complete! All steps have been executed."
</ExecuteImplementation>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### Step 1: Add Type Definitions ⏳ PENDING

**Objective**: Add `ComponentSelector`, `BrpQuery`, and `BrpQueryFilter` type definitions to support Bevy 0.17.2's new query API

**Changes**:
- Add three new type definitions to `mcp/src/brp_tools/tools/world_query.rs`
- All types are additive - won't break existing code
- Proper serde attributes for JSON serialization

**Files to Modify**:
- `mcp/src/brp_tools/tools/world_query.rs` (add types before `QueryParams` struct)

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**:
- ✅ Code compiles successfully
- ✅ New types available for use in Step 2
- ✅ No changes to existing functionality

**Implementation Details**:

Add these type definitions to `mcp/src/brp_tools/tools/world_query.rs` before the `QueryParams` struct:

```rust
/// Selector for optional components in a query (mirrors Bevy's ComponentSelector)
///
/// **Default Implementation**: Uses `#[derive(Default)]` with `#[default]` attribute on the
/// `Paths` variant. This provides automatic Default implementation returning `Paths(vec![])`.
/// Do NOT add a manual `impl Default` - it would conflict with the derived implementation.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComponentSelector {
    /// Select all components present on the entity
    All,
    /// Select specific components by their full type paths
    ///
    /// This is the default variant - `ComponentSelector::default()` returns `Paths(vec![])`
    #[serde(untagged)]
    #[default]
    Paths(Vec<String>),
}

/// Query data specification - what component data to retrieve
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct BrpQuery {
    /// Required components - entities must have all of these
    #[serde(default)]
    pub components: Vec<String>,

    /// Optional components - retrieve if present. Can be "all" or array of paths
    #[serde(default)]
    pub option: ComponentSelector,

    /// Components to check for presence (returns boolean, not data)
    #[serde(default)]
    pub has: Vec<String>,
}

/// Query filter specification - which entities to include
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct BrpQueryFilter {
    /// Entities must have all of these components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<String>,

    /// Entities must NOT have any of these components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
}
```

---

### Step 2: Update QueryParams Struct ⏳ PENDING

**Objective**: Replace `Value` types with typed structs in `QueryParams` for type safety and better IDE support

**Changes**:
- Change `pub data: Value` to `pub data: BrpQuery`
- Change `pub filter: Option<Value>` to `pub filter: Option<BrpQueryFilter>`
- Update doc comments to reflect new types

**Files to Modify**:
- `mcp/src/brp_tools/tools/world_query.rs` (modify `QueryParams` struct)

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**:
- ✅ Code compiles successfully
- ✅ JSON serialization works automatically via serde
- ✅ Backward compatible at JSON API level (serde handles both formats)
- ✅ Type safety at Rust level

**Implementation Details**:

In `mcp/src/brp_tools/tools/world_query.rs`, update the `QueryParams` struct:

**Before**:
```rust
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve. Required.
    /// Structure: {components: string[], option: string[], has: string[]}.
    /// Use {} to get entity IDs only withoutcomponent data.
    pub data: Value,

    /// Object specifying which entities to query. Optional. Structure: {with: string[],
    /// without: string[]}. Defaults to {} (no filter) if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,

    /// If true, returns error on unknown component types (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}
```

**After**:
```rust
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve
    pub data: BrpQuery,

    /// Object specifying which entities to query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<BrpQueryFilter>,

    /// If true, returns error on unknown component types (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}
```

**Serialization Verification**:
- Serde's `#[derive(Serialize)]` handles nested `BrpQuery` and `BrpQueryFilter` recursively
- The `BrpTools` macro calls `serde_json::to_value(&params)` (line 81 of `mcp_macros/src/brp_tools.rs`)
- `ComponentSelector` enum serializes correctly:
  - `All` variant → `"all"` (via `#[serde(rename_all = "snake_case")]`)
  - `Paths` variant → `["path1", "path2"]` (via `#[serde(untagged)]`)

**Expected JSON Output**:
```json
// With ComponentSelector::All
{
  "data": {
    "components": [],
    "option": "all",
    "has": []
  },
  "filter": {
    "with": ["bevy_transform::components::transform::Transform"]
  }
}

// With ComponentSelector::Paths (backward compatible)
{
  "data": {
    "components": ["bevy_transform::components::transform::Transform"],
    "option": ["bevy_sprite::sprite::Sprite"],
    "has": []
  }
}
```

---

### Step 3: Update Help Text Documentation ⏳ PENDING

**Objective**: Document the new `ComponentSelector` enum and "all" option syntax in user-facing help text

**Changes**:
- Update `option` field documentation to show both array and "all" syntax
- Add example showing "all" option usage
- Update inline comments to reflect new capabilities

**Files to Modify**:
- `mcp/help_text/world_query.txt`

**Build Command**:
```bash
# No build needed - documentation only
```

**Expected Impact**:
- ✅ AI agents understand the new "all" option
- ✅ Examples show both backward-compatible array syntax and new "all" syntax
- ✅ Educational goal of MCP tool is fulfilled

**Implementation Details**:

**Update 1 - Line 17-18**: Change from:
```
- `option`: Array of components to retrieve if present (optional components)
```

To:
```
- `option`: Components to retrieve if present (optional components). Can be:
  - Array of component paths: `["bevy_sprite::sprite::Sprite", "bevy_transform::components::transform::Transform"]`
  - `"all"` to select all components on matching entities
```

**Update 2 - After line 56**: Add new example:
```

Get all components from entities with Transform:
```json
{
  "data": {
    "option": "all"
  },
  "filter": {
    "with": ["bevy_transform::components::transform::Transform"]
  }
}
```
```

**Update 3 - Lines 6-11**: Update inline comment:
```json
{
  "components": ["bevy_transform::components::transform::Transform"],
  "option": ["bevy_sprite::sprite::Sprite"],  // or "all" to get all components
  "has": ["bevy_render::camera::camera::Camera"]
}
```

---

### Step 4: Complete Validation ⏳ PENDING

**Objective**: Run integration test to verify all query formats work correctly with Bevy 0.17.2

**Changes**:
- Execute integration test suite
- Verify backward compatibility (array syntax)
- Verify new functionality ("all" syntax)
- Verify edge cases and error handling

**Files to Modify**:
- None (testing only)

**Test Command**:
```bash
/test query
```

**Expected Impact**:
- ✅ All 11 test scenarios pass
- ✅ Array syntax works (backward compatibility confirmed)
- ✅ "all" syntax works (new feature confirmed)
- ✅ Edge cases handled correctly
- ✅ Error messages are clear for invalid inputs

**Test Coverage**:

The integration test `.claude/tests/query.md` validates:

1. **Backward Compatibility**: Array syntax for `option` field
2. **New "all" Syntax**: ComponentSelector::All variant
3. **Default Behavior**: Empty/omitted option field
4. **Entity IDs Only**: Empty data object
5. **Filter Combinations**: with + without
6. **Mixed Fields**: components + option + has together
7. **Filter Omission vs Empty**: Serialization equivalence
8. **"all" with Filter**: New syntax combined with filtering
9. **Error Handling**: Invalid option values
10. **Entity Setup/Cleanup**: Proper test isolation

**Success Criteria**:
- Test output shows "✅ PASSED" for all scenarios
- No "❌ FAILED" sections in test results
- Test summary shows 0 critical issues

---

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Background Information

### Summary

Between Bevy 0.16.1 and 0.17.2, the BRP (Bevy Remote Protocol) underwent significant changes beyond just method renaming. There are **meaningful argument structure changes** that affect how methods are called.

### Key Changes in Bevy 0.17.2

**world.query - Optional Parameters Added**

In 0.17.2, `world.query` can now be called without parameters to get all entities.

**BrpQuery.option - Type Changed from Vec to Enum**

The `option` field now accepts either:
- `"all"` - to select all components
- An array of component paths (backward compatible with 0.16.1 format)

### Current Status

- **bevy_brp_mcp** is already using Bevy 0.17.2
- **Both JSON formats already work** through the current implementation
- Current implementation uses `pub data: Value` which passes through raw JSON to Bevy's BRP

### Benefits of This Upgrade

1. **Type Safety**: Compile-time validation of query structure
2. **Better IDE Support**: Autocomplete and type hints when using the MCP tool
3. **Clear Documentation**: The enum makes it explicit that `option` accepts either "all" or an array
4. **JSON Schema Generation**: The `JsonSchema` derive will generate proper schema showing both options
5. **Validation**: Invalid query structures will be caught during deserialization
6. **Maintainability**: Changes to Bevy's BRP types can be mirrored in our code

### Version Compatibility

Since the changes are backward compatible (array syntax still works in 0.17.2), we can update the MCP tool to use explicit types without breaking existing usage. Users on Bevy 0.17.2+ will get the full benefits of both formats.

### Scope Limitations

**Other tools with `Value` fields are intentionally untyped:**

- `world.mutate_components` - `value: Value` (arbitrary component field data)
- `world.mutate_resources` - `value: Value` (arbitrary resource field data)
- `world.insert_resources` - `value: Value` (arbitrary resource data)

These remain as `Value` because they hold dynamic, type-dependent data that cannot be statically typed. The `world.query` case is unique because its structure (`data` and `filter` fields) is fixed and defined by Bevy's BRP specification, making it suitable for typed structs.

## Method Files Locations

- **Bevy 0.16.1**: `/Users/natemccoy/rust/bevy-0.16.1/crates/bevy_remote/src/builtin_methods.rs`
- **Bevy 0.17.2**: `/Users/natemccoy/rust/bevy/crates/bevy_remote/src/builtin_methods.rs`

## Design Review Skip Notes

This implementation has been reviewed for:
- ✅ Breaking change analysis - all changes are backward compatible at JSON level
- ✅ Dependency analysis - single file changes with no downstream impacts
- ✅ Serialization verification - serde handles nested structs automatically
- ✅ Test coverage - comprehensive integration test created
- ✅ Documentation - help text updated to reflect new functionality
