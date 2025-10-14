# Fix spawn_format for Types with Default Trait

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

### Step 1: Add reflection and spawn constants ✅ COMPLETED

**Objective**: Add new constants for Default trait detection and spawn format descriptions

**Change Type**: Additive (SAFE - no existing code affected)

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/constants.rs`

**Changes**:
Add two new constants to support Default trait detection and spawn format guidance.

**Expected impact**:
- Constants are added but not yet used
- No behavior changes until later steps

**Build command**:
```bash
cargo build && cargo +nightly fmt
```

**Success criteria**: Build succeeds, new constants are available for import

---

### Step 2: Add required imports ✅ COMPLETED

**Objective**: Import the json macro, JsonObjectAccess trait, and new constants into types.rs

**Change Type**: Additive (SAFE - imports added but not yet used)

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes**:
1. Modify `serde_json` import to include `json` macro
2. Add `JsonObjectAccess` trait import (required for `get_field_array` method)
3. Add constants import from parent directory

**Dependencies**: Requires Step 1 (constants must exist)

**Expected impact**:
- Imports are available for use
- No behavior changes yet

**Build command**:
```bash
cargo build && cargo +nightly fmt
```

**Success criteria**: Build succeeds, all imports resolve correctly

---

### Step 3: Implement Default trait handling logic ✅ COMPLETED

**Objective**: Add logic to detect Default trait and set spawn_format to {} for PartiallyMutable types

**Change Type**: Breaking (ATOMIC GROUP - all changes must be done together)

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes**:
1. Add `has_default_for_root` variable to check for Default trait on root paths
2. Update description logic to append DEFAULT_SPAWN_GUIDANCE when Default trait is present
3. Update spawn_format/example logic to set `json!({})` for PartiallyMutable types with Default

**Dependencies**: Requires Steps 1 and 2

**Expected impact**:
- PartiallyMutable types with Default trait now get `spawn_format: {}`
- Description includes guidance about spawning with empty object
- 26 of 27 tested types will now work correctly (96% success rate)

**Build command**:
```bash
cargo build && cargo +nightly fmt
```

**Success criteria**:
- Build succeeds
- All three logic changes compile together
- Type guide generation works correctly

---

### Step 4: Complete Validation ✅ COMPLETED

**Objective**: Verify the implementation works correctly with test types

**Validation steps**:
1. Run full build: `cargo build && cargo +nightly fmt`
2. Test with the 27 PartiallyMutable + Default types mentioned in validation section
3. Verify `spawn_format: {}` appears for types like `bevy_gizmos::retained::Gizmo`
4. Confirm descriptions include Default spawn guidance

**Success criteria**:
- All builds pass
- Type guide shows spawn_format for PartiallyMutable + Default types
- Descriptions are clear and accurate
- 26 of 27 types work (bevy_window::cursor::CursorIcon expected to fail due to enum limitation)

---

## Problem

Types with `Default` trait but `PartiallyMutable` root status get `spawn_format: null`, causing subagents to report `COMPONENT_NOT_FOUND` when they should spawn with `{}`.

Example: `bevy_gizmos::retained::Gizmo` - can be spawned with empty object via Default, but gets `spawn_format: null`.

## Solution

Check for Default trait during mutation path conversion and set both `spawn_format: {}` and appropriate description guidance in one consolidated location.

## Validation

Tested 27 PartiallyMutable + Default types:
- ✅ **26 types work** (96% success rate) - spawn successfully with `{}`
- ❌ **1 type fails** - `bevy_window::cursor::CursorIcon` (enum limitation - BRP requires explicit variants)

## Implementation Details

### Step 1 Details: Add constants (mcp/src/brp_tools/brp_type_guide/constants.rs)

**File location**: `mcp/src/brp_tools/brp_type_guide/constants.rs` (parent directory of mutation_path_builder)

Add after `REFLECT_TRAIT_RESOURCE`:

```rust
/// Reflection trait name for Default implementation
pub const REFLECT_TRAIT_DEFAULT: &str = "Default";
```

Add after existing description constants:

```rust
/// Guidance appended to root path description for Default trait spawning
pub const DEFAULT_SPAWN_GUIDANCE: &str = " However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path";
```

**Note**: The `PARTIALLY_MUTABLE_DESCRIPTION` constant from the original plan is no longer needed because commit dfc77aa already implemented type-specific partial mutability messages using `type_kind.child_terminology()` (e.g., "fields", "elements", "entries", "variants").

### Step 2 Details: Update imports (mutation_path_builder/types.rs)

Modify the existing `serde_json` import to include the `json` macro:

**Change from**:
```rust
use serde_json::Value;
```

**To**:
```rust
use serde_json::{Value, json};
```

Add the `JsonObjectAccess` trait import (required for `get_field_array` method):

```rust
use crate::json_object::JsonObjectAccess;
```

Add constants import after the existing module imports:

```rust
use super::super::constants::{DEFAULT_SPAWN_GUIDANCE, REFLECT_TRAIT_DEFAULT};
```

Note: `SchemaField` is already imported via `use crate::json_schema::SchemaField;` - no changes needed.

**Important**: No changes to `mutation_path_builder/mod.rs` are needed. The constants are imported from the parent directory (`brp_type_guide/constants.rs`) using `super::super::constants`, so no module declaration is required in the mutation_path_builder module.

### Step 3 Details: Check for Default trait once (mutation_path_builder/types.rs)

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

### Step 3 Details: Update description logic (mutation_path_builder/types.rs)

In `from_mutation_path_internal` function, locate the `let description = match path.mutation_status` block (around line 334):

**Current code**:
```rust
let description = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        format!(
            "This {} path is partially mutable due to some of its {} not being mutable",
            type_kind.as_ref().to_lowercase(),
            type_kind.child_terminology()
        )
    }
    _ => path.path_kind.description(&type_kind),
};
```

**New code**:
```rust
let description = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        let base_msg = format!(
            "This {} path is partially mutable due to some of its {} not being mutable",
            type_kind.as_ref().to_lowercase(),
            type_kind.child_terminology()
        );
        if has_default_for_root {
            format!("{base_msg}.{DEFAULT_SPAWN_GUIDANCE}")
        } else {
            base_msg
        }
    }
    _ => path.path_kind.description(&type_kind),
};
```

### Step 3 Details: Update conversion logic to set spawn_format (mutation_path_builder/types.rs)

In `from_mutation_path_internal` function, locate the `let (examples, example) = match path.mutation_status` block:

**Current code**:
```rust
let (examples, example) = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        // PartiallyMutable enums: show examples array with per-variant status
        // PartiallyMutable non-enums: no examples
        path.enum_example_groups.as_ref().map_or_else(
            || (vec![], None),
            |enum_examples| (enum_examples.clone(), None),
        )
    }
    MutationStatus::NotMutable => {
        // NotMutable: no example at all (not even null)
        (vec![], None)
    }
    MutationStatus::Mutable => {
        path.enum_example_groups.as_ref().map_or_else(
            || {
                // Mutable paths: use the example value
                // This includes enum children (with embedded `applicable_variants`) and
                // regular values
                (vec![], Some(path.example.clone()))
            },
            |enum_examples| {
                // Enum root: use the examples array
                (enum_examples.clone(), None)
            },
        )
    }
};
```

**New code**:
```rust
let (examples, example) = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        // PartiallyMutable enums: show examples array with per-variant status
        // PartiallyMutable non-enums: check for Default trait
        path.enum_example_groups.as_ref().map_or_else(
            || {
                let example = if has_default_for_root {
                    Some(json!({}))
                } else {
                    None
                };
                (vec![], example)
            },
            |enum_examples| (enum_examples.clone(), None),  // Enum: use examples array
        )
    }
    MutationStatus::NotMutable => {
        // NotMutable: no example (no NotMutable+Default types found in practice)
        (vec![], None)
    }
    MutationStatus::Mutable => {
        path.enum_example_groups.as_ref().map_or_else(
            || {
                // Mutable paths: use the example value
                // This includes enum children (with embedded `applicable_variants`) and
                // regular values
                (vec![], Some(path.example.clone()))
            },
            |enum_examples| {
                // Enum root: use the examples array
                (enum_examples.clone(), None)
            },
        )
    }
};
```

## Result

### For PartiallyMutable types WITH Default trait:
- `spawn_format: {}` is set and available for extraction
- Description becomes:
  > "This struct path is partially mutable due to some of its fields not being mutable. However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path"

### For PartiallyMutable types WITHOUT Default trait:
- `spawn_format` remains `None` (absent)
- Description is:
  > "This struct path is partially mutable due to some of its fields not being mutable"

### Example: `bevy_sprite::sprite::Sprite`
```json
{
  "spawn_format": {},
  "mutation_paths": {
    "": {
      "description": "This struct path is partially mutable due to some of its fields not being mutable. However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path",
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

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.
