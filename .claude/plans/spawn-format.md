# Fix spawn_format for Types with Default Trait

## Problem
Types with `Default` trait but `PartiallyMutable` root status get `spawn_format: null`, causing subagents to report `COMPONENT_NOT_FOUND` when they should spawn with `{}`.

Example: `bevy_gizmos::retained::Gizmo` - can be spawned with empty object via Default, but gets `spawn_format: null`.

## Solution
Check for Default trait during mutation path conversion and set both `spawn_format: {}` and appropriate description guidance in one consolidated location.

## Validation
Tested 27 PartiallyMutable + Default types:
- ✅ **26 types work** (96% success rate) - spawn successfully with `{}`
- ❌ **1 type fails** - `bevy_window::cursor::CursorIcon` (enum limitation - BRP requires explicit variants)

## Changes

### 1. Add constants (constants.rs)

Add after `REFLECT_TRAIT_RESOURCE`:

```rust
/// Reflection trait name for Default implementation
pub const REFLECT_TRAIT_DEFAULT: &str = "Default";
```

Add after existing description constants:

```rust
/// Description for partially mutable paths
pub const PARTIALLY_MUTABLE_DESCRIPTION: &str = "This path is partially mutable, some child paths are mutable and some are not";

/// Guidance appended to root path description for Default trait spawning
pub const DEFAULT_SPAWN_GUIDANCE: &str = " However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path.";
```

### 2. Update imports (mutation_path_builder/types.rs)

Modify the existing `serde_json` import to include the `json` macro:

**Change from**:
```rust
use serde_json::Value;
```

**To**:
```rust
use serde_json::{Value, json};
```

Add constants import after the existing module imports:

```rust
use super::super::constants::{
    DEFAULT_SPAWN_GUIDANCE, PARTIALLY_MUTABLE_DESCRIPTION, REFLECT_TRAIT_DEFAULT,
};
```

Note: `SchemaField` is already imported via `use crate::json_schema::SchemaField;` - no changes needed.

### 3. Check for Default trait once (mutation_path_builder/types.rs)

In `from_mutation_path_internal` function, after the `type_kind` variable is set:

**Add this code**:
```rust
// Check for Default trait once at the top for root paths
let has_default_for_root = if matches!(path.path_kind, PathKind::RootValue) {
    field_schema
        .get_field_array(SchemaField::ReflectTypes)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .any(|t| t == REFLECT_TRAIT_DEFAULT)
        })
        .unwrap_or(false)
} else {
    false
};
```

### 4. Update description logic (mutation_path_builder/types.rs)

In `from_mutation_path_internal` function, locate the `let description = match path.mutation_status` block:

**Current code**:
```rust
let description = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        "This path is not mutable due to some of its descendants not being mutable"
            .to_string()
    }
    _ => path.path_kind.description(&type_kind),
};
```

**New code**:
```rust
let description = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        if has_default_for_root {
            format!("{PARTIALLY_MUTABLE_DESCRIPTION}.{DEFAULT_SPAWN_GUIDANCE}")
        } else {
            PARTIALLY_MUTABLE_DESCRIPTION.to_string()
        }
    }
    _ => path.path_kind.description(&type_kind),
};
```

### 5. Update conversion logic to set spawn_format (mutation_path_builder/types.rs)

In `from_mutation_path_internal` function, locate the `let (examples, example) = match path.mutation_status` block:

**Current code**:
```rust
let (examples, example) = match path.mutation_status {
    MutationStatus::PartiallyMutable | MutationStatus::NotMutable => {
        // PartiallyMutable and NotMutable: no example at all (not even null)
        (vec![], None)
    }
    MutationStatus::Mutable => {
        // ... handles mutable paths
    }
};
```

**New code**:
```rust
let (examples, example) = match path.mutation_status {
    MutationStatus::PartiallyMutable | MutationStatus::NotMutable => {
        let example = if has_default_for_root {
            Some(json!({}))
        } else {
            None
        };
        (vec![], example)
    }
    MutationStatus::Mutable => {
        // ... handles mutable paths
    }
};
```

## Result

### For PartiallyMutable types WITH Default trait:
- `spawn_format: {}` is set and available for extraction
- Description becomes:
  > "This path is partially mutable, some child paths are mutable and some are not. However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path."

### For PartiallyMutable types WITHOUT Default trait:
- `spawn_format` remains `None` (absent)
- Description is:
  > "This path is partially mutable, some child paths are mutable and some are not"

### Example: `bevy_sprite::sprite::Sprite`
```json
{
  "spawn_format": {},
  "mutation_paths": {
    "": {
      "description": "This path is partially mutable, some child paths are mutable and some are not. However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path.",
      "path_info": {
        "mutation_status": "partially_mutable"
      }
    }
  }
}
```

## Architecture Benefits

- **Single location**: All logic in conversion function, no enum threading
- **Self-contained**: Uses `PathKind` to detect roots, `registry` to check Default
- **No signature changes**: No parameters added to functions
- **Uses constants**: No hard-coded strings
- **Clear descriptions**: Explains mutation limitations and Default capabilities
