# Enhanced Variant Path with Step-by-Step Examples

## Overview

Replace the complex parent wrapping algorithm with a simpler approach that enhances the `variant_path` array with examples and descriptions for each step. This provides coding agents with a clear step-by-step instruction manual rather than trying to construct complex nested examples.

## Current Problem

The existing parent wrapping implementation only does partial context building, showing intermediate structures like `{"Conditional": 1000000}` instead of complete root-to-target context.

## New Approach

Instead of constructing complex nested examples in `path_requirement.example`, enhance each `variant_path` entry with:
- `description`: Clear instruction for this step
- `example`: The exact mutation value needed for this step

## Structural Changes

### Enhanced VariantPathEntry Type

**Current structure:**
```rust
pub struct VariantPathEntry {
    pub path: String,
    pub variant: String,
}
```

**New structure:**
```rust
pub struct VariantPathEntry {
    pub path: String,
    pub variant: String,
    pub description: String,  // New field - populated during recursion
    pub example: Value,       // New field - populated during recursion
}
```

### Example Output Format

**Before:**
```json
"path_requirement": {
  "description": "To use this mutation path, root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional",
  "example": {"Conditional": 1000000},
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Nested"},
    {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
  ]
}
```

**After:**
```json
"path_requirement": {
  "description": "This mutation path requires variants to be set as described in the variant_path array",
  "example": 1000000,  // Keep simple local example
  "variant_path": [
    {
      "path": "",
      "variant": "TestEnumWithSerDe::Nested",
      "description": "Set root to TestEnumWithSerDe::Nested",
      "example": {"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}
    },
    {
      "path": ".nested_config",
      "variant": "NestedConfigEnum::Conditional",
      "description": "Set .nested_config to NestedConfigEnum::Conditional",
      "example": {"Conditional": 1000000}
    }
  ]
}
```

## Implementation Plan

### Step 1: Remove Complex Parent Wrapping Logic

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Remove these method calls and helper functions:**

```rust
// REMOVE this call from build_paths method (lines 105-111):
Self::wrap_path_requirements_with_parent_context(
    &mut paths_to_expose,
    &example_to_use,
    enum_root_examples.as_ref(),
    ctx,
);

// REMOVE these entire helper methods (lines 969-1081):
fn wrap_path_requirements_with_parent_context(...)
fn wrap_single_path_requirement(...)
fn find_matching_enum_example(...)
fn navigate_index(...)
fn navigate_field(...)
fn substitute_at_path(...)
```

**Revert paths_to_expose to immutable:**
```rust
// Change line 66 from:
let ChildProcessingResult {
    all_paths,
    mut paths_to_expose,  // Remove mut
    child_examples,
} = self.process_all_children(ctx, depth)?;

// Back to:
let ChildProcessingResult {
    all_paths,
    paths_to_expose,      // Immutable again
    child_examples,
} = self.process_all_children(ctx, depth)?;
```

### Step 2: Update VariantPathEntry Type

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Modify VariantPathEntry:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPathEntry {
    pub path: String,
    pub variant: String,
    // Add new required fields:
    pub description: String,
    pub example: Value,
}
```

### Step 3: Build Enhanced Variant Path During Recursion

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` (or wherever variant_path entries are created)

**Modify how VariantPathEntry is created during recursion:**

```rust
// When creating VariantPathEntry during recursion, populate all fields:
let variant_entry = VariantPathEntry {
    path: ctx.mutation_path.clone(),
    variant: current_variant.clone(),
    description: format!(
        "Set {} to {}",
        if ctx.mutation_path.is_empty() { "root" } else { &ctx.mutation_path },
        current_variant
    ),
    example: current_example.clone(),  // The example available at this level
};
```

**Note:** Since the variant_path is built during recursion as part of the `ctx.variant_chain` infrastructure, each level already has access to its own example and can build its description. No post-processing at root level is needed.

### Step 4: Simplify PathRequirement Description

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

**Update PathRequirement creation to use simplified description:**

```rust
// In create_path_requirement method, change from complex description building to:
let description = if variant_chain.len() > 1 {
    "This mutation path requires variants to be set as described in the variant_path array".to_string()
} else {
    format!("To use this mutation path, the root must be set to {}",
        variant_chain.first().unwrap().variant)
};
```

### Step 5: Remove Unused Imports

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Remove the now-unused imports that were added for the wrapping logic:**
```rust
// Remove these imports:
use crate::error::{Error, Result};
use error_stack::Report;
```

## Advantages

✅ **Simpler implementation** - No complex parent wrapping or JSON substitution
✅ **More informative** - Step-by-step instruction manual for agents
✅ **No post-processing needed** - Everything built during recursion
✅ **Clean data flow** - Each level populates its own data
✅ **Better separation of concerns** - variant_path becomes the instruction manual

## Testing

After implementation, the `.nested_config.0` path should show:

```json
"path_requirement": {
  "description": "This mutation path requires variants to be set as described in the variant_path array",
  "example": 1000000,
  "variant_path": [
    {
      "path": "",
      "variant": "TestEnumWithSerDe::Nested",
      "description": "Set root to TestEnumWithSerDe::Nested",
      "example": {"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}
    },
    {
      "path": ".nested_config",
      "variant": "NestedConfigEnum::Conditional",
      "description": "Set .nested_config to NestedConfigEnum::Conditional",
      "example": {"Conditional": 1000000}
    }
  ]
}
```

This provides agents with clear step-by-step instructions rather than requiring them to reverse-engineer from complex nested examples.