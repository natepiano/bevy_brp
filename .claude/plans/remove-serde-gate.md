# Plan: Remove Serialize/Deserialize Gating Logic

## Background
Bevy 0.17.2 fixed the bug that previously required `Serialize`/`Deserialize` traits for spawn/insert operations. Components now only need the `Reflect` trait for all BRP operations (spawn, insert, mutate, query, get).

**Verified:** Successfully spawned `extras_plugin::TestStructNoSerDe` which has `Reflect` and `FromReflect` but NO `Serialize`/`Deserialize` traits.

## Migration Strategy

**Migration Strategy: Atomic**

All changes in this plan must be implemented as a single, complete, indivisible unit. This is not a phased migration - we are removing obsolete gating logic that is no longer correct given Bevy 0.17.2's behavior. Either keep current design unchanged OR replace it entirely with the updated logic. No hybrid approaches or gradual rollouts are appropriate for this cleanup.

## Scope

**SerDe gating**: Logic that conditionally enables spawn/insert operations based on Serialize/Deserialize trait presence, now obsolete in Bevy 0.17.2+.

This allows removal of:
1. SerDe gating from spawn/insert operations
2. Test components specifically designed to test "NoSerDe" failure cases
3. Test documentation about SerDe gating and expected failures
4. Help text mentioning SerDe gating requirements

## Important Distinction

**REMOVE (SerDe gating and redundant fields):**
- SerDe gating from `get_supported_operations()` function
- All logic that uses `has_serialize`/`has_deserialize` to control which operations are available
- Top-level `has_serialize` and `has_deserialize` fields (replaced by `reflect_types` in `schema_info`)
- `supported_operations` field (now derivable from `reflect_types`)

**ADD (consolidated reflection trait info):**
- `reflect_types` array in `schema_info` containing all reflection traits: `["Component", "Resource", "Serialize", "Deserialize", "Default", etc.]`
- Clients can check for Component/Resource/SerDe by looking at this single array
- Internal code uses same `reflect_types.contains(&ReflectTrait::Component)` pattern

## Determining Mutate Operation Support

After removing the `supported_operations` field, clients determine whether a type supports Mutate operations by checking the `mutation_paths` HashMap:

**Client-side detection:**
```javascript
// Check if any path is mutable or partially mutable
const supportsMutate = Object.values(mutation_paths).some(path =>
  ["mutable", "partially_mutable"].includes(path.path_info.mutation_status)
);
```

**Why this works:**
- Components/Resources ALWAYS support Query, Get, Spawn, Insert (if they have Component/Resource trait)
- Mutate support is CONDITIONAL - depends on whether the type has any mutable fields
- The `mutation_paths` HashMap already contains this information via `path_info.mutation_status`
- No new field needed - clients query existing data

**Migration example:**
```javascript
// Old approach:
if (type_guide.supported_operations.includes("mutate")) { ... }

// New approach:
if (Object.values(type_guide.mutation_paths).some(p =>
  ["mutable", "partially_mutable"].includes(p.path_info.mutation_status)
)) { ... }
```

## Changes Required

### 1. **Rust Code Changes**

#### `mcp/src/brp_tools/brp_type_guide/type_guide.rs`
**Function: `get_supported_operations()`**

**Remove SerDe gating:**

Current logic in component handling section:
```rust
let has_component = reflect_types.contains(&ReflectTrait::Component);
let has_resource = reflect_types.contains(&ReflectTrait::Resource);
let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

if has_component {
    operations.push(BrpSupportedOperation::Get);
    if has_serialize && has_deserialize {
        operations.push(BrpSupportedOperation::Spawn);
        operations.push(BrpSupportedOperation::Insert);
    }
}
```

New logic:
```rust
let has_component = reflect_types.contains(&ReflectTrait::Component);
let has_resource = reflect_types.contains(&ReflectTrait::Resource);

if has_component {
    operations.push(BrpSupportedOperation::Get);
    operations.push(BrpSupportedOperation::Spawn);
    operations.push(BrpSupportedOperation::Insert);
}
```

Current resource logic in resource handling section:
```rust
if has_resource && has_serialize && has_deserialize {
    // Resources support Insert but mutation capability is determined dynamically
    // based on actual mutation path analysis in from_schema()
    operations.push(BrpSupportedOperation::Insert);
}
```

New logic:
```rust
if has_resource {
    // Resources support Insert but mutation capability is determined dynamically
    // based on actual mutation path analysis in from_schema()
    operations.push(BrpSupportedOperation::Insert);
}
```

**Elements to remove:**
- Remove `has_serialize` and `has_deserialize` variable declarations from `get_supported_operations()`
- Remove the `if has_serialize && has_deserialize {` condition wrapper around component Spawn/Insert operations
- Remove the `&& has_serialize && has_deserialize` condition from resource Insert operation check

#### `mcp/src/brp_tools/brp_type_guide/type_guide.rs`
**Function: `from_registry_schema()`**
**REMOVE** - serialization trait detection at top level is no longer needed:
```rust
// REMOVE THESE LINES:
let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);
```

These checks are replaced by including `reflect_types` array in `schema_info` (see `extract_schema_info()` changes below).

#### `mcp/src/brp_tools/brp_type_guide/type_guide.rs`
**Remove `supported_operations` field entirely** - After removing SerDe gating, supported operations become completely deterministic based on reflection traits. This field adds no value and should be removed.

**Changes required:**

**1. Remove `supported_operations`, `has_serialize`, and `has_deserialize` fields from `TypeGuide` struct** (after the `in_registry` field)
```rust
// REMOVE THESE LINES:
pub has_serialize:        bool,
pub has_deserialize:      bool,
pub supported_operations: Vec<BrpSupportedOperation>,
```

**2. In `from_registry_schema()` function:**

Remove the `get_supported_operations()` call (after reflect traits extraction):
```rust
// REMOVE THIS LINE:
let supported_operations = Self::get_supported_operations(&reflect_types);
```

Remove the mutable clone and Mutate operation logic (after the `get_supported_operations()` call removal):
```rust
// REMOVE THESE LINES:
let mut supported_operations = supported_operations;
if Self::has_mutable_paths(&mutation_paths) {
    supported_operations.push(BrpSupportedOperation::Mutate);
}
```

Simplify spawn_format generation (in the spawn format conditional block):

Current:
```rust
let spawn_format = if supported_operations.contains(&BrpSupportedOperation::Spawn)
    || supported_operations.contains(&BrpSupportedOperation::Insert)
{
    Self::extract_spawn_format_from_paths(&mutation_paths)
} else {
    None
};
```

Change to:
```rust
let has_component = reflect_types.contains(&ReflectTrait::Component);
let has_resource = reflect_types.contains(&ReflectTrait::Resource);

let spawn_format = if has_component || has_resource {
    Self::extract_spawn_format_from_paths(&mutation_paths)
} else {
    None
};
```

Remove from struct construction (in the `TypeGuide` initialization at the end of `from_registry_schema()`):
```rust
// REMOVE THESE LINES:
has_serialize,
has_deserialize,
supported_operations,
```

**3. In `not_found_in_registry()` function:**

Remove from struct construction (in the `TypeGuide` initialization):
```rust
// REMOVE THESE LINES:
has_serialize: false,
has_deserialize: false,
supported_operations: Vec::new(),
```

**4. Delete entire `get_supported_operations()` function** (private helper function in the impl block)
```rust
// DELETE THIS ENTIRE FUNCTION
fn get_supported_operations(reflect_types: &[ReflectTrait]) -> Vec<BrpSupportedOperation> {
    // ... entire function body
}
```

**Rationale:**
- Components ALWAYS support: Query, Get, Spawn, Insert (and conditionally Mutate based on mutation_paths)
- Resources ALWAYS support: Query, Get, Insert (and conditionally Mutate based on mutation_paths)
- These are now deterministic - no need to calculate and store them
- Clients can derive operation support by checking `schema_info.reflect_types` for "Component" or "Resource"
- Removes redundant fields from API responses - all reflection trait info now in one place

#### `mcp/src/brp_tools/brp_type_guide/type_guide.rs`
**Function: `extract_schema_info()`** - Add `reflect_types` to schema_info

Current implementation (lines 263-309) extracts: `type_kind`, `properties`, `required`, `module_path`, `crate_name`

**Specific changes required:**

1. **After extracting `crate_name` (line 290), add reflect_types extraction:**
   ```rust
   // ADD THIS after line 290:
   let reflect_types = Self::extract_reflect_types(registry_schema);
   ```

2. **Update the conditional check (lines 293-297) to include reflect_types:**
   ```rust
   // MODIFY the if condition to add:
   if type_kind.is_some()
       || properties.is_some()
       || required.is_some()
       || module_path.is_some()
       || crate_name.is_some()
       || !reflect_types.is_empty()  // ADD THIS LINE
   {
   ```

3. **Add reflect_types field to SchemaInfo construction (after line 304):**
   ```rust
   Some(SchemaInfo {
       type_kind,
       properties,
       required,
       module_path,
       crate_name,
       reflect_types: Some(reflect_types),  // ADD THIS LINE
   })
   ```

**Complete modified function:**

```rust
fn extract_schema_info(registry_schema: &Value) -> Option<SchemaInfo> {
    let type_kind = registry_schema
        .get_field(SchemaField::Kind)
        .and_then(Value::as_str)
        .and_then(|s| TypeKind::from_str(s).ok());

    let properties = registry_schema.get_field(SchemaField::Properties).cloned();

    let required = registry_schema
        .get_field(SchemaField::Required)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        });

    let module_path = registry_schema
        .get_field(SchemaField::ModulePath)
        .and_then(Value::as_str)
        .map(String::from);

    let crate_name = registry_schema
        .get_field(SchemaField::CrateName)
        .and_then(Value::as_str)
        .map(String::from);

    // Extract reflection traits
    let reflect_types = Self::extract_reflect_types(registry_schema);

    // Only return SchemaInfo if we have at least some information
    if type_kind.is_some()
        || properties.is_some()
        || required.is_some()
        || module_path.is_some()
        || crate_name.is_some()
        || !reflect_types.is_empty()
    {
        Some(SchemaInfo {
            type_kind,
            properties,
            required,
            module_path,
            crate_name,
            reflect_types: Some(reflect_types),
        })
    } else {
        None
    }
}
```

**Note:** The `extract_reflect_types()` helper already exists at line 250 (private function in the impl block), we're just reusing it here. Empty `reflect_types` array is allowed - the conditional returns Some if ANY field has data.

#### `mcp/src/brp_tools/brp_type_guide/response_types.rs`
**Update `SchemaInfo` struct** - Add `reflect_types` field

```rust
/// Schema information extracted from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Category of the type (Struct, Enum, etc.) from registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_kind:     Option<TypeKind>,
    /// Field definitions from the registry schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties:    Option<Value>,
    /// Required fields list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required:      Option<Vec<String>>,
    /// Module path of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path:   Option<String>,
    /// Crate name of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crate_name:    Option<String>,
    /// Reflection traits available on this type (Component, Resource, Serialize, Deserialize, etc.)
    /// Clients can check this array to determine supported operations:
    /// - Contains "Component" → supports Query, Get, Spawn, Insert (+ Mutate if mutable)
    /// - Contains "Resource" → supports Query, Get, Insert (+ Mutate if mutable)
    /// - Contains "Serialize"/"Deserialize" → type can be serialized (informational only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflect_types: Option<Vec<ReflectTrait>>,
}
```

#### `mcp/src/brp_tools/brp_type_guide/response_types.rs`
**Update `ReflectTrait` enum** - Add serialization support

Current:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum ReflectTrait {
    Component,
    Resource,
    Serialize,
    Deserialize,
}
```

New:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "PascalCase")]
#[serde(rename_all = "PascalCase")]
pub enum ReflectTrait {
    Component,
    Resource,
    Serialize,
    Deserialize,
}
```

**Rationale:** Required for serializing the `reflect_types` field in SchemaInfo. The PascalCase format matches Bevy's reflection trait naming convention ("Component", "Resource", etc.). Without these derives, compilation will fail when SchemaInfo (which has `#[derive(Serialize, Deserialize)]`) tries to serialize the `Option<Vec<ReflectTrait>>` field.

#### `mcp/src/brp_tools/brp_type_guide/response_types.rs`
**Update `BrpSupportedOperation` enum documentation:**

Current Insert variant documentation:
```rust
/// Insert operation - requires Serialize + Deserialize traits
Insert,
```

New:
```rust
/// Insert operation - requires Reflect trait
Insert,
```

Current Spawn variant documentation:
```rust
/// Spawn operation - requires Serialize + Deserialize traits
Spawn,
```

New:
```rust
/// Spawn operation - requires Reflect trait
Spawn,
```

#### `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/value_builder.rs`
**Remove SerDe gating from mutation path building**

The `ValueMutationBuilder` currently gates mutation based on Serialize/Deserialize traits, contradicting the plan's goal that only Reflect is required.

**Current code** (in `assemble_from_children()` method):
```rust
// Check if this Value type has serialization support
if !ctx.value_type_has_serialization(ctx.type_name()) {
    return Err(BuilderError::NotMutable(
        NotMutableReason::MissingSerializationTraits(ctx.type_name().clone()),
    ));
}
```

**Remove these lines** - Types with only Reflect should be mutable if they have mutation knowledge or are composite types.

**After removal**, the function should return the appropriate NotMutableReason for types without mutation knowledge:
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    _children: HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Value, BuilderError> {
    // For leaf types without mutation knowledge, return appropriate reason
    Err(BuilderError::NotMutable(
        NotMutableReason::NoExampleAvailable(ctx.type_name().clone()),
    ))
}
```

#### `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`
**Remove now-obsolete SerDe helper method**

**Dependency check performed:** Searched codebase for all usages of `value_type_has_serialization`:
- Found in: `recursion_context.rs` (definition), `value_builder.rs` (single call site - being removed), `.claude/plans/remove-serde-gate.md` (this plan)
- **Verified safe to remove** - only call site is in `value_builder.rs` lines 36-41, which is being removed as part of this plan

Remove the `value_type_has_serialization()` method (no longer needed after removing SerDe gating from value_builder.rs):

```rust
// REMOVE THIS ENTIRE METHOD:
pub fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
    self.get_registry_schema(type_name).is_some_and(|schema| {
        let reflect_types: Vec<ReflectTrait> = ...
        reflect_types.contains(&ReflectTrait::Serialize)
            && reflect_types.contains(&ReflectTrait::Deserialize)
    })
}
```

**Rationale:** This method's sole purpose was SerDe gating, which is now obsolete. Bevy 0.17.2 requires only the Reflect trait for all BRP operations including mutation. Keeping this method would suggest SerDe is still relevant for mutations.

### 2. **Test Component Removal from extras_plugin.rs**

**Remove these test entities that were specifically testing spawn failures without SerDe:**

- **Entity spawning with `TestStructNoSerDe`** (in `spawn_test_component_entities` function, after the TestMixedMutability entities section)
  ```rust
  // Entity with TestStructNoSerDe - REMOVE THIS
  commands.spawn((...));
  ```

- **Entity spawning with `TestEnumNoSerDe`** (in `spawn_test_component_entities` function, in the test entities section)
  ```rust
  // Entity with TestEnumNoSerDe - REMOVE THIS
  commands.spawn((...));
  ```

**Remove these test component definitions:**

- **`TestEnumNoSerDe` enum definition** (located in component definitions section)
  ```rust
  /// Test component enum WITHOUT Serialize/Deserialize (only Reflect)
  #[derive(Component, Default, Reflect)]
  #[reflect(Component)]
  enum TestEnumNoSerDe { ... }
  ```

**KEEP (still needed for mutation testing):**

- **`TestStructNoSerDe` struct definition** - This type is still used by `TestMixedMutabilityCore` to test mutation behavior for fields without serialization traits. Update its documentation:
  ```rust
  /// Test component struct WITHOUT Serialize/Deserialize (only Reflect)
  /// Used by TestMixedMutabilityCore to test mutation_status_reason for fields
  /// that lack serialization traits (demonstrates NotMutableReason::MissingSerializationTraits)
  #[derive(Component, Default, Reflect)]
  #[reflect(Component, FromReflect)]
  struct TestStructNoSerDe { ... }
  ```

- **`TestConfigResource` struct definition** - Keep unchanged as the standard test resource

- **`RuntimeStatsResource` struct definition** - Keep unchanged. While it was originally used to test insertion without SerDe traits (now irrelevant), it remains useful as a test resource and doesn't need removal.

**Rationale:** TestStructNoSerDe serves TWO purposes:
1. ~~Testing spawn failures without SerDe~~ (obsolete - remove entity spawn)
2. Testing mutation behavior for fields without SerDe (still relevant - keep struct definition)

The TestMixedMutability types (TestMixedMutabilityCore, Vec, Array, Tuple, Enum) test the `mutation_status_reason` diagnostic system, not spawn operations. They remain valuable for validating mutation analysis.

### 3. **Test Documentation Updates**

#### `.claude/tests/type_guide.md`

**Search and replace ALL references to removed fields:**

1. **Search the entire file for:** `has_serialize`, `has_deserialize`, `supported_operations`
2. **Update each reference:**
   - Replace field checks with `schema_info.reflect_types` array checks
   - Example: `has_serialize: false` → check that `schema_info.reflect_types` does NOT contain "Serialize"
   - Example: `supported_operations: ["query", "get", "mutate"]` → derive from `schema_info.reflect_types` containing "Component"

**Remove entire test sections that tested SerDe failure cases:**

- **### 4. Validate Sprite Component (No Serialize Trait)** - Remove entire section
  - This entire section is obsolete - Sprite now supports spawn like any other component

- **#### 9e. Test Component Without Serialize/Deserialize - Spawn Failure** - Remove entire subsection
  - This test expected spawn to fail without SerDe - no longer true

**Update test expectations:**

- **In #### 4a. Validate Type Info (line 59):**
  - BEFORE: `has_serialize: false, has_deserialize: false, supported_operations: ["query", "get", "mutate"]`
  - AFTER: `schema_info.reflect_types` should NOT contain "Serialize" or "Deserialize", but SHOULD contain "Component"
  - Delete the entire `supported_operations` check - this field no longer exists

- **In #### 4c. Verify Spawn Format Is Absent:** Update this subsection
  - Rename to: "#### 4c. Verify Spawn Format Is Present"
  - Current expectation: `null (cannot spawn without Serialize)`
  - New expectation: Spawn format should now be present for Sprite (all components with Reflect can be spawned)
  - Update the extraction command to verify spawn_format is NOT null

- **In ### 7. Validate Name Component (line 138):**
  - BEFORE: "Has both `mutation_paths` and `spawn_format` (has Serialize/Deserialize)"
  - AFTER: "Has both `mutation_paths` and `spawn_format` (has Reflect trait)" OR check `schema_info.reflect_types` contains "Component"

- **In ## Success Criteria section (line 325):**
  - Remove: "Components without Serialize can be mutated but not spawned"
  - Remove: "Verify `supported_operations` includes spawn/insert for all components" (field no longer exists)
  - Add: "All components with Reflect trait can be spawned, mutated, and queried"
  - Add: "Check `schema_info.reflect_types` array to determine type capabilities"

**Add new verification:**
- Consider adding a test that verifies components with only Reflect (no SerDe) CAN be spawned successfully

#### `.claude/tests/resource.md`

**Major simplification** - the distinction between TestConfigResource and RuntimeStatsResource is no longer relevant:

- **## Objective section:** Simplify the test objective
  - Remove mentions of testing SerDe vs non-SerDe resources
  - Focus on general resource operations
  - Update: "Validate BRP behavior with resources" (remove the "that lack Serialize/Deserialize traits" part)

- **## Test Resources section:** Remove or simplify
  - Both resources now work the same way
  - No need to distinguish them based on SerDe traits
  - Keep the list but remove the SerDe trait distinctions

- **Within ### 1. Insert Resource Test:**
  - **STEP 4** (Insert RuntimeStatsResource): Keep but update comment
    - Current: "Insert/update RuntimeStatsResource (no Serialize/Deserialize traits)"
    - New: "Insert/update RuntimeStatsResource"
    - Current success note: "Verify operation succeeds despite lacking Serialize/Deserialize traits"
    - New: "Verify operation succeeds"
  - **STEP 5** (Get RuntimeStatsResource): Keep but simplify
    - No longer needs to emphasize "despite lacking SerDe"

**Simplification:** This test file could potentially be merged into general resource operation tests since there's no longer a special SerDe case to test.

### 4. **Help Text Updates**

#### `mcp/help_text/brp_type_guide.txt`

Current reflection traits description:
```
- reflection traits: Reflect, Serialize, Deserialize,
```

New:
```
- schema_info.reflect_types: Array of reflection traits on this type (Component, Resource, Serialize, Deserialize, Default, etc.)
```

Add clarifying note:
```
Note: As of Bevy 0.17.2, only the Reflect trait is required for spawn/insert operations.
Check schema_info.reflect_types array to determine type capabilities:
- Contains "Component" → supports Query, Get, Spawn, Insert operations (+ Mutate if mutable fields exist)
- Contains "Resource" → supports Query, Get, Insert operations (+ Mutate if mutable fields exist)
- Contains "Serialize"/"Deserialize" → type can be serialized (informational only, does not affect supported operations)
```

#### `mcp/help_text/brp_all_type_guides.txt`

Same changes as `brp_type_guide.txt` above (reflection traits description and clarifying note).

#### `mcp/help_text/world_spawn_entity.txt`

Current note:
```
Note: Requires BRP registration
```

New:
```
Note: Requires component to be registered with BRP and have the Reflect trait
```

#### `mcp/help_text/world_insert_components.txt`

Current note:
```
Note: Requires BRP registration
```

New:
```
Note: Requires component to be registered with BRP and have the Reflect trait
```

#### `mcp/help_text/world_insert_resources.txt`

Current note:
```
Note: Requires BRP registration and reflection traits.
```

New:
```
Note: Requires resource to be registered with BRP and have the Reflect trait
```

### 5. **Verification After Changes**

1. **Build:** `cargo build && cargo +nightly fmt`
2. **Test:** `cargo nextest run`
3. **Manual verification:**
   - Launch extras_plugin
   - Verify spawn works for components with only Reflect trait (like Sprite)
   - Verify type_guide no longer returns `supported_operations` field
   - Verify spawn_format is present for all components/resources
4. **Update baselines if needed:**
   - Type guide test baselines may need updating
   - Check `.claude/transient/` for baseline files
5. **API compatibility check:**
   - The removal of `supported_operations`, `has_serialize`, and `has_deserialize` fields is a **breaking change** for clients
   - Document in CHANGELOG or migration guide:
     * Old: Clients checked `supported_operations` array for operation availability
     * Old: Clients checked `has_serialize`/`has_deserialize` booleans for trait info
     * New: Clients check `schema_info.reflect_types` array for all reflection trait info
     * Migration: `supported_operations.includes("spawn")` → `schema_info.reflect_types.includes("Component") || schema_info.reflect_types.includes("Resource")`
     * Migration: `has_serialize` → `schema_info.reflect_types.includes("Serialize")`
     * Migration: `has_deserialize` → `schema_info.reflect_types.includes("Deserialize")`
     * Migration for Mutate support: `supported_operations.includes("mutate")` → `Object.values(mutation_paths).some(path => ["mutable", "partially_mutable"].includes(path.path_info.mutation_status))`

## Summary

**Estimated changes:**
- **Rust code:** ~40 lines removed/modified in type_guide.rs, ~15 lines updated in response_types.rs
- **Test components:** ~100+ lines removed from extras_plugin.rs
- **Test docs:** ~50+ lines removed/updated across type_guide.md and resource.md
- **Help text:** ~30 lines updated across 5 files

**Total files modified:** ~10 files

**Key improvements:**
1. Codebase now correctly reflects Bevy 0.17.2 capabilities where only `Reflect` trait is required for all BRP operations
2. API consolidation: All reflection trait information now in single `schema_info.reflect_types` array
3. Removes redundant top-level fields (`has_serialize`, `has_deserialize`, `supported_operations`)
4. Cleaner API: Clients query one array instead of multiple scattered fields
5. Forward-compatible: New reflection traits automatically exposed without API changes

## Testing Strategy

After implementation:
1. Verify all existing tests still pass
2. Verify components with only `Reflect` can be spawned/inserted (test with Sprite component)
3. Verify `supported_operations`, `has_serialize`, and `has_deserialize` fields removed from type_guide responses
4. Verify `schema_info.reflect_types` array is present and contains correct traits
5. Verify `spawn_format` is present for all components/resources (including those without SerDe traits)
6. Verify help text accurately describes requirements (only Reflect needed)
7. Test migration path: Ensure clients can derive operation support from `reflect_types` array
8. Consider adding explicit test case for Reflect-only spawn success
