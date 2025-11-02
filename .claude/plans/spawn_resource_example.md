# Refactor spawn_format to Self-Documenting SpawnInsertExample Enum

## Overview
Replace `spawn_format: Option<Value>` with a self-erasing `SpawnInsertExample` enum that uses variant names (`SpawnExample` for Components, `ResourceExample` for Resources) to self-document the operation type. Uses custom `Serialize` implementation like `PathExample` to achieve field name transformation and conditional field inclusion.

## Implementation Steps

### 1. Create SpawnInsertExample Enum with Custom Serialization
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Add new enum with custom `Serialize` implementation:
```rust
/// Spawn/insert example with educational guidance for AI agents
///
/// Serializes differently based on variant:
/// - `SpawnExample` → `{"spawn_example": {"agent_guidance": "...", "example": <value>}}`
/// - `ResourceExample` → `{"resource_example": {"agent_guidance": "...", "example": <value>}}`
///
/// When `example` is `Example::NotApplicable`, only `agent_guidance` is included.
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

### 2. Create Helper Function
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

Replace `extract_spawn_format()` with `extract_spawn_insert_example()`:
- Check if type is Component or Resource (return `None` if neither)
- Extract root path example from mutation_paths
- Build appropriate variant:
  - Component → `SpawnExample { agent_guidance: SPAWN_FORMAT_COMPONENT_GUIDANCE or NO_SPAWN_FORMAT_COMPONENT, example: ... }`
  - Resource → `ResourceExample { agent_guidance: SPAWN_FORMAT_RESOURCE_GUIDANCE or NO_SPAWN_FORMAT_RESOURCE, example: ... }`
- Use guidance constants based on whether example is available

### 3. Update TypeGuide Structure
**File**: `mcp/src/brp_tools/brp_type_guide/guide.rs`

- Change field: `spawn_format: Option<Value>` → `spawn_insert_example: Option<SpawnInsertExample>`
- Add `#[serde(flatten)]` to make `spawn_example`/`resource_example` appear at TypeGuide top level
- Update `build()` to call `extract_spawn_insert_example(mutation_paths, reflect_traits)`
- Simplify `generate_agent_guidance()` - remove spawn format guidance section (lines 164-186), remove reflect_traits and spawn_format parameters
- Delete `extract_spawn_format_if_spawnable()` helper (lines 191-214)
- Update error builders (`not_found_in_registry`, `processing_failed`) to use new field name

### 4. Update Public Exports
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

- Add: `pub use types::SpawnInsertExample;`
- Change: `pub use api::extract_spawn_format;` → `pub use api::extract_spawn_insert_example;`

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
use serde::ser::SerializeMap;

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
