# Refactor spawn_format to Self-Documenting SpawnInsertExample Enum

## Overview
Replace `spawn_format: Option<Value>` with a self-erasing `SpawnInsertExample` enum that uses variant names (`SpawnExample` for Components, `ResourceExample` for Resources) to self-document the operation type. Uses custom `Serialize` implementation like `PathExample` to achieve field name transformation and conditional field inclusion.

## Implementation Steps

### 1. Create SpawnInsertExample Enum with Custom Serialization
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**First, add required import at the top of the file:**
```rust
use serde::ser::SerializeMap;
```

**Add helper method to Example enum:**
```rust
impl Example {
    // ... existing to_value() method ...

    /// Returns true if this Example represents a null-equivalent value
    /// (OptionNone or NotApplicable)
    pub fn is_null_equivalent(&self) -> bool {
        matches!(self, Self::OptionNone | Self::NotApplicable)
    }
}
```

Add new enum with custom `Serialize` implementation:
```rust
/// Spawn/insert example with educational guidance for AI agents
///
/// Serializes differently based on variant:
/// - `SpawnExample` → `{"spawn_example": {"agent_guidance": "...", "example": <value>}}`
/// - `ResourceExample` → `{"resource_example": {"agent_guidance": "...", "example": <value>}}`
///
/// When `example` is `Example::NotApplicable`, only `agent_guidance` is included.
///
/// Note: Only derives Debug and Clone (NOT Deserialize) because we implement
/// Deserialize manually below with a stub that returns an error.
#[derive(Debug, Clone)]
pub enum SpawnInsertExample {
    SpawnExample {
        agent_guidance: String,
        example: Example,
    },
    ResourceExample {
        agent_guidance: String,
        example: Example,
    },
}
```

Custom `Serialize` implementation (mirrors `PathExample` pattern):
- Use `serializer.serialize_map(Some(1))`
- `SpawnExample` → serializes with `"spawn_example"` key
- `ResourceExample` → serializes with `"resource_example"` key
- Check `example.is_null_equivalent()` to conditionally include/exclude `"example"` field
- Always include `"agent_guidance"` field
- Add stub `Deserialize` implementation (required for `#[serde(flatten)]`)
  - Note: `use serde::Deserialize;` already exists in types.rs, no new import needed

### 2. Rename Guidance Constants
**File**: `mcp/src/brp_tools/brp_type_guide/constants.rs`

Rename the existing spawn_format constants to reflect spawn/insert terminology:
- `SPAWN_FORMAT_COMPONENT_GUIDANCE` → `SPAWN_COMPONENT_GUIDANCE` (line 106)
- `SPAWN_FORMAT_RESOURCE_GUIDANCE` → `INSERT_RESOURCE_GUIDANCE` (line 109)
- `NO_SPAWN_FORMAT_COMPONENT` → `NO_SPAWN_COMPONENT_EXAMPLE` (line 113)
- `NO_SPAWN_FORMAT_RESOURCE` → `NO_INSERT_RESOURCE_EXAMPLE` (line 116)

**Update constant values to match new field names:**
- `SPAWN_COMPONENT_GUIDANCE`: Change "spawn_format" to "spawn_example" in the text
- `INSERT_RESOURCE_GUIDANCE`: Change "spawn_format" to "resource_example" in the text
- `NO_SPAWN_COMPONENT_EXAMPLE`: Change "spawn_format" to "spawn_example" in the text
- `NO_INSERT_RESOURCE_EXAMPLE`: Change "spawn_format" to "resource_example" in the text

### 3. Create Helper Function
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

**Import the renamed constants at the top of the file:**
```rust
use super::super::constants::{
    INSERT_RESOURCE_GUIDANCE,
    NO_INSERT_RESOURCE_EXAMPLE,
    NO_SPAWN_COMPONENT_EXAMPLE,
    SPAWN_COMPONENT_GUIDANCE,
};
```
(Note: These constants were renamed in step 2)

Replace `extract_spawn_format()` with `extract_spawn_insert_example()`:

**Function signature:**
```rust
pub fn extract_spawn_insert_example(
    mutation_paths: &[MutationPathExternal],
    reflect_traits: &[String],
) -> Option<SpawnInsertExample>
```

**Implementation logic:**
- Check if type is Component or Resource using reflect_traits (return `None` if neither)
- Extract root path example from mutation_paths
- Build appropriate variant:
  - Component → `SpawnExample { agent_guidance: SPAWN_COMPONENT_GUIDANCE or NO_SPAWN_COMPONENT_EXAMPLE, example: ... }`
  - Resource → `ResourceExample { agent_guidance: INSERT_RESOURCE_GUIDANCE or NO_INSERT_RESOURCE_EXAMPLE, example: ... }`
- Use guidance constants based on whether example is available

### 4. Update TypeGuide Structure
**File**: `mcp/src/brp_tools/brp_type_guide/guide.rs`

- Change field: `spawn_format: Option<Value>` → `spawn_insert_example: Option<SpawnInsertExample>`
- Add `#[serde(flatten)]` to make `spawn_example`/`resource_example` appear at TypeGuide top level
- Update `build()` method (around lines 84-91):

  **Before:**
  ```rust
  // Extract reflect traits for guidance generation
  let reflect_traits = registry_schema
      .get_field_array(SchemaField::ReflectTypes)
      .map(|arr| arr.iter().filter_map(Value::as_str).into_strings())
      .unwrap_or_default();

  // Extract spawn format if type is spawnable (Component or Resource)
  let spawn_format =
      Self::extract_spawn_format_if_spawnable(registry_schema, &mutation_paths);
  ```

  **After:**
  ```rust
  // Extract reflect traits for guidance generation and spawn/insert example
  let reflect_traits = registry_schema
      .get_field_array(SchemaField::ReflectTypes)
      .map(|arr| arr.iter().filter_map(Value::as_str).into_strings())
      .unwrap_or_default();

  // Extract spawn/insert example (calls api.rs directly, no wrapper needed)
  let spawn_insert_example =
      mutation_path_builder::extract_spawn_insert_example(&mutation_paths, &reflect_traits);
  ```

  **Note**: This eliminates the duplication where `extract_spawn_format_if_spawnable()` was re-extracting reflect_traits from registry_schema. Now we pass the already-extracted reflect_traits directly to the api function.

- Update the TypeGuide struct field assignment in `build()` to use the new field name:
  - Change `spawn_format` field assignment to `spawn_insert_example`
  - The value is now `spawn_insert_example` (from the call above) instead of `spawn_format`

- Simplify `generate_agent_guidance()`:
  - **New signature**: `fn generate_agent_guidance(mutation_paths: &[MutationPathExternal]) -> Result<String>`
  - **Remove parameters**: `spawn_format: Option<&Value>` and `reflect_traits: &[String]`
  - **Remove logic**: Delete spawn format guidance section (lines 164-186) - all conditional appending for Components/Resources
  - **Keep**: Only the Entity warning logic (lines 148-156)
  - **Update doc comment**: Change to "Generate agent guidance with Entity warning" and remove "spawn format guidance" from description
  - **Update call site at line 98**: Change from `Self::generate_agent_guidance(&mutation_paths, spawn_format.as_ref(), &reflect_traits)?` to `Self::generate_agent_guidance(&mutation_paths)?`
- Delete `extract_spawn_format_if_spawnable()` helper (lines 191-214)
- Update error builders (`not_found_in_registry`, `processing_failed`) to use new field name

### 5. Update Public Exports
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

- Add: `pub(super) use types::SpawnInsertExample;`
- Change: `pub(super) use api::extract_spawn_format;` → `pub(super) use api::extract_spawn_insert_example;`

**Note**: Use `pub(super)` visibility to maintain the existing encapsulation pattern where mutation_path_builder internals are only accessible to the parent brp_type_guide module. This matches all other exports in mod.rs (build_mutation_paths, MutationPathExternal, VariantSignature).

## Output Transformation Examples

### Component with Available Example
**Before**:
```json
{
  "agent_guidance": "The 'mutation_paths'... The 'spawn_format' field provides an example...",
  "spawn_format": {"is_active": true, "hdr": false}
}
```

**After**:
```json
{
  "agent_guidance": "The 'mutation_paths' field provides...",
  "spawn_example": {
    "agent_guidance": "The 'example' below can be used to spawn this component on an entity.",
    "example": {"is_active": true, "hdr": false}
  }
}
```

### Resource with Unavailable Example
**Before**:
```json
{
  "agent_guidance": "The 'mutation_paths'... This resource does not have a 'spawn_format' field...",
  "spawn_format": null
}
```

**After**:
```json
{
  "agent_guidance": "The 'mutation_paths' field provides...",
  "resource_example": {
    "agent_guidance": "This resource does not have an insert example because it does not have a 'mutable' root mutation path."
  }
}
```

## Benefits
- **Self-documenting**: Field name (`spawn_example` vs `resource_example`) indicates the operation type
- **Guidance co-location**: Educational text appears with the data it describes
- **Type-safe**: Uses existing `Example` enum for compile-time safety
- **Pattern consistency**: Matches `PathExample` custom serialization pattern exactly
- **Cleaner agent_guidance**: No more conditional string appending

## Detailed Implementation: Custom Serialize

```rust
impl Serialize for SpawnInsertExample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::SpawnExample { agent_guidance, example } => {
                // Check if example is NotApplicable (null-equivalent)
                if example.is_null_equivalent() {
                    // Only serialize agent_guidance field
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("spawn_example", &serde_json::json!({
                        "agent_guidance": agent_guidance
                    }))?;
                    map.end()
                } else {
                    // Serialize both fields
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("spawn_example", &serde_json::json!({
                        "agent_guidance": agent_guidance,
                        "example": example.to_value()
                    }))?;
                    map.end()
                }
            }
            Self::ResourceExample { agent_guidance, example } => {
                // Check if example is NotApplicable (null-equivalent)
                if example.is_null_equivalent() {
                    // Only serialize agent_guidance field
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("resource_example", &serde_json::json!({
                        "agent_guidance": agent_guidance
                    }))?;
                    map.end()
                } else {
                    // Serialize both fields
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("resource_example", &serde_json::json!({
                        "agent_guidance": agent_guidance,
                        "example": example.to_value()
                    }))?;
                    map.end()
                }
            }
        }
    }
}

/// Stub `Deserialize` implementation for `SpawnInsertExample`
///
/// Required by serde's flatten attribute but never actually used.
impl<'de> Deserialize<'de> for SpawnInsertExample {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "SpawnInsertExample deserialization not implemented - this type is write-only",
        ))
    }
}
```

## Design Review Skip Notes

## TYPE-SYSTEM-1: Missing derive attributes on SpawnInsertExample enum - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section 1: Create SpawnInsertExample Enum with Custom Serialization
- **Issue**: The new SpawnInsertExample enum only derives Debug and Clone, but should follow the same pattern as Example enum which derives PartialEq and Eq for value comparison. This is especially important since the enum contains an Example field that already implements these traits.
- **Reasoning**: The finding is incorrect because the `SpawnInsertExample` enum follows the same pattern as `PathExample` enum, which also only derives `Debug` and `Clone` (lines 11-12 of path_example.rs). Both enums serve the same purpose: custom serialization wrappers that are write-only (they have stub Deserialize implementations that return errors). These enums are never compared for equality in the codebase. The plan explicitly models `SpawnInsertExample` after `PathExample` (section 1 states "Uses custom `Serialize` implementation like `PathExample`"), so omitting `PartialEq` and `Eq` is intentional and consistent with the existing design pattern. The plan is correct in deriving only `Debug` and `Clone` for this write-only serialization type. The `Example` enum is different - it's used as a field value and needs comparison semantics, which is why it has `PartialEq` and `Eq`. But the outer wrapper enums that handle custom serialization (`PathExample` and `SpawnInsertExample`) intentionally omit these derives because they're never compared.
- **Decision**: User elected to skip this recommendation

## IMPLEMENTATION-1: Incomplete custom Serialize implementation for error handling - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Detailed Implementation: Custom Serialize
- **Issue**: The custom Serialize implementation calls example.to_value() which always succeeds since Example::to_value() is infallible, but wraps the result in serde_json::json! macro unnecessarily. The PathExample serialization pattern is simpler and more direct - it serializes the Value directly without the json! macro wrapper. This creates inconsistency with the established pattern.
- **Reasoning**: The finding is incorrect because it fundamentally misunderstands the difference in serialization contexts between PathExample and SpawnInsertExample. PathExample serializes a SINGLE field with a Value directly: `map.serialize_entry("example", &value)` where value is already a JSON Value. It doesn't need json! macro because it's just passing through a Value. SpawnInsertExample serializes a NESTED OBJECT with TWO fields: `{"agent_guidance": "...", "example": {...}}`. The json! macro is the correct and idiomatic Rust way to construct this composite structure. Without json!, you'd need to manually build a Map<String, Value> and insert both fields - the macro is cleaner. The finding claims the plan's approach wraps to_value() in json! 'unnecessarily', but the suggested code does THE EXACT SAME THING - it extracts `let value = example.to_value()` and then uses `json!({ "agent_guidance": ..., "example": value })`. This is functionally identical to the plan's `json!({ "agent_guidance": ..., "example": example.to_value() })`. There is no pattern inconsistency here. PathExample uses Value directly because it's serializing a single field. SpawnInsertExample uses json! because it's building a nested object. These are appropriately different approaches for different contexts.
- **Decision**: User elected to skip this recommendation

## DESIGN-1: Pattern inconsistency with PathExample serialization approach - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section 1: Create SpawnInsertExample Enum with Custom Serialization
- **Issue**: The plan claims to mirror the PathExample pattern but uses a different approach. PathExample serializes the Value directly with 'map.serialize_entry("example", &value)?' but the proposed SpawnInsertExample wraps everything in 'serde_json::json!({...})' before serializing. This creates two different serialization patterns in the same module, making the code harder to maintain and understand.
- **Reasoning**: This finding is incorrect because the two serialization patterns serve fundamentally different purposes and produce different JSON structures, making the divergence intentional and appropriate. PathExample serializes a flat structure where the variant determines which single field appears at the top level: Simple variant → {"example": <value>} or EnumRoot variant → {"examples": <array>}. SpawnInsertExample needs to serialize a nested structure where each variant creates a parent key containing an object with multiple fields: SpawnExample → {"spawn_example": {"agent_guidance": "...", "example": <value>}} or ResourceExample → {"resource_example": {"agent_guidance": "...", "example": <value>}}. The plan correctly uses serde_json::json!() to construct the nested object because the inner object needs two fields (agent_guidance and conditionally example). Using json!() macro is the idiomatic way to build multi-field JSON objects inline. The alternative (creating another SerializeMap for the inner object) would be more verbose and complex. PathExample doesn't need json!() because it serializes single values directly - there's no nested object to construct. The finding mistakenly treats pattern similarity as a requirement when the different data structures justify different approaches.
- **Decision**: User elected to skip this recommendation
