# Fix spawn_format for Types with Default Trait

## Problem
Types with `Default` trait but `PartiallyMutable` root status get `spawn_format: null`, causing subagents to report `COMPONENT_NOT_FOUND` when they should spawn with `{}`.

Example: `bevy_gizmos::retained::Gizmo` - can be spawned with empty object via Default, but gets `spawn_format: null`.

## Solution
Use an enum to track spawn format source and set `spawn_format: {}` for types with Default trait, with clear documentation in root path description.

## Changes

### 1. Add SpawnFormatSource enum (builder.rs)
Create new enum before `TypeGuide` impl block:

```rust
/// Tracks the source of a spawn format value
#[derive(Debug, Clone)]
enum SpawnFormatSource {
    /// No spawn format available - type doesn't support spawning or has no Default
    None,
    /// Spawn format extracted from root mutation path example
    Example(Value),
    /// Spawn format is empty object {} via Default trait
    DefaultTrait,
}

impl SpawnFormatSource {
    /// Convert to Option<Value> for spawn_format field
    fn to_spawn_format(&self) -> Option<Value> {
        match self {
            Self::None => None,
            Self::Example(value) => Some(value.clone()),
            Self::DefaultTrait => Some(json!({})),
        }
    }

    /// Check if this represents Default trait usage
    const fn is_default_trait(&self) -> bool {
        matches!(self, Self::DefaultTrait)
    }
}
```

### 2. Add constants (constants.rs)
Add after `REFLECT_TRAIT_RESOURCE`:

```rust
/// Reflection trait name for Default implementation
pub const REFLECT_TRAIT_DEFAULT: &str = "Default";
```

Add after `ERROR_GUIDANCE`:

```rust
/// Guidance appended to root path description for Default trait spawning
pub const DEFAULT_SPAWN_GUIDANCE: &str = " Note: This type supports spawning via its Default trait - use empty object {} with world_spawn_entity or world_insert_resources.";
```

### 3. Modify extract_spawn_format_if_spawnable (builder.rs:173-192)

**Current signature**:
```rust
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &HashMap<String, MutationPath>,
) -> Option<Value>
```

**New signature**:
```rust
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &HashMap<String, MutationPath>,
) -> SpawnFormatSource
```

**New implementation**:
```rust
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &HashMap<String, MutationPath>,
) -> SpawnFormatSource {
    // Check if type is spawnable (has Component or Resource trait)
    let reflect_types = registry_schema
        .get_field_array(SchemaField::ReflectTypes)
        .map(|arr| arr.iter().filter_map(Value::as_str).into_strings())
        .unwrap_or_default();

    let is_spawnable = reflect_types.iter().any(|trait_name| {
        trait_name == REFLECT_TRAIT_COMPONENT || trait_name == REFLECT_TRAIT_RESOURCE
    });

    if !is_spawnable {
        return SpawnFormatSource::None;
    }

    // Try to get spawn format from root path example
    if let Some(value) = Self::extract_spawn_format_from_paths(mutation_paths) {
        return SpawnFormatSource::Example(value);
    }

    // Check if type has Default trait
    let has_default = reflect_types
        .iter()
        .any(|trait_name| trait_name == REFLECT_TRAIT_DEFAULT);

    if has_default {
        SpawnFormatSource::DefaultTrait
    } else {
        SpawnFormatSource::None
    }
}
```

### 4. Update from_registry_schema (builder.rs:62-104)

Change lines 85-86 from:
```rust
let spawn_format =
    Self::extract_spawn_format_if_spawnable(registry_schema, &mutation_paths);
```

To:
```rust
let spawn_format_source =
    Self::extract_spawn_format_if_spawnable(registry_schema, &mutation_paths);

let spawn_format = spawn_format_source.to_spawn_format();
```

Change line 82 from:
```rust
let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec, &registry);
```

To (move after spawn_format_source):
```rust
let mutation_paths = Self::convert_mutation_paths(
    &mutation_paths_vec,
    &registry,
    &spawn_format_source,
);
```

### 5. Update convert_mutation_paths (builder.rs:213-229)

**Current signature**:
```rust
fn convert_mutation_paths(
    paths: &[MutationPathInternal],
    registry: &HashMap<BrpTypeName, Value>,
) -> HashMap<String, MutationPath>
```

**New signature**:
```rust
fn convert_mutation_paths(
    paths: &[MutationPathInternal],
    registry: &HashMap<BrpTypeName, Value>,
    spawn_format_source: &SpawnFormatSource,
) -> HashMap<String, MutationPath>
```

**New implementation**:
```rust
fn convert_mutation_paths(
    paths: &[MutationPathInternal],
    registry: &HashMap<BrpTypeName, Value>,
    spawn_format_source: &SpawnFormatSource,
) -> HashMap<String, MutationPath> {
    paths
        .iter()
        .map(|path| {
            let is_root_with_default = spawn_format_source.is_default_trait()
                && path.full_mutation_path.is_empty();
            let path_info = MutationPath::from_mutation_path_internal(
                path,
                registry,
                is_root_with_default,
            );
            let key = (*path.full_mutation_path).clone();
            (key, path_info)
        })
        .collect()
}
```

### 6. Update MutationPath::from_mutation_path_internal (types.rs:341-415)

**Current signature**:
```rust
pub fn from_mutation_path_internal(
    path: &MutationPathInternal,
    registry: &HashMap<BrpTypeName, Value>,
) -> Self
```

**New signature**:
```rust
pub fn from_mutation_path_internal(
    path: &MutationPathInternal,
    registry: &HashMap<BrpTypeName, Value>,
    is_root_with_default: bool,
) -> Self
```

**Update description logic** (lines 352-358):
```rust
let description = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        let base = "This path is not mutable due to some of its descendants not being mutable";
        if is_root_with_default {
            use super::super::constants::DEFAULT_SPAWN_GUIDANCE;
            format!("{base}.{DEFAULT_SPAWN_GUIDANCE}")
        } else {
            base.to_string()
        }
    }
    _ => path.path_kind.description(&type_kind),
};
```

### 7. Update imports (builder.rs)

Add `json` macro import to serde_json (line 15):
```rust
use serde_json::{Value, json};
```

Update constants import to include new constant (lines 17-20):
```rust
use super::constants::{
    AGENT_GUIDANCE, ENTITY_WARNING, ERROR_GUIDANCE, REFLECT_TRAIT_COMPONENT,
    REFLECT_TRAIT_DEFAULT, REFLECT_TRAIT_RESOURCE, TYPE_BEVY_ENTITY,
};
```

## Result
- Types with Default trait get `spawn_format: {}`
- Root path description explains Default spawning capability
- Type-safe enum eliminates boolean flag confusion
- Self-documenting code via enum variant names
- `bevy_gizmos::retained::Gizmo` and similar types work correctly
