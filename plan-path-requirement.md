# Enhanced Variant Path with Step-by-Step Examples

## Overview

Replace the complex parent wrapping algorithm with a simpler approach that enhances the `variant_path` array with examples and descriptions for each step. This provides coding agents with a clear step-by-step instruction manual rather than trying to construct complex nested examples.

## Current Problem

The existing parent wrapping implementation is **fundamentally broken** - it produces incorrect examples that don't provide the full context agents need. For example, it shows intermediate structures like `{"Conditional": 1000000}` instead of the complete root-to-target context like `{"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}`. This isn't just suboptimal - it's wrong and prevents agents from understanding how to properly construct mutations.

## Design Review Skip Notes

⚠️ PREJUDICE WARNING - **DESIGN-2**: Unclear rationale for simplification benefits over current functionality - **Verdict**: REJECTED
- **Status**: PERMANENTLY REJECTED
- **Critical Note**: DO NOT SUGGEST THIS AGAIN - The finding fundamentally misunderstood that the current code is BROKEN, not just complex. The current implementation produces incorrect examples that fail to provide proper context for agents. This isn't about "simplification for its own sake" - it's about fixing broken functionality.

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
    pub instructions: String,  // New field - populated during recursion
    pub example: Value,        // New field - populated during recursion
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
      "instructions": "Set root to TestEnumWithSerDe::Nested",
      "example": {"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}
    },
    {
      "path": ".nested_config",
      "variant": "NestedConfigEnum::Conditional",
      "instructions": "Set .nested_config to NestedConfigEnum::Conditional",
      "example": {"Conditional": 1000000}
    }
  ]
}
```

## Implementation Plan

### Step 1: Remove Complex Parent Wrapping Logic

⚠️ **REQUIRES USER ACTION**: This step involves removing existing broken code. The implementation must STOP here and ask the USER to manually remove these functions before proceeding to Step 2.

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**USER MUST REMOVE these method calls and helper functions:**

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

**USER MUST ALSO revert paths_to_expose to immutable:**
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

**IMPLEMENTATION NOTE**: After showing the user what needs to be removed, WAIT for user confirmation that the removal is complete before proceeding to Step 2. This ensures the codebase is in the correct state for adding the new functionality.

### Step 2: Update VariantPathEntry Type

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Modify VariantPathEntry:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPathEntry {
    pub path: String,
    pub variant: String,
    // Add new required fields:
    pub instructions: String,
    pub example: Value,
}
```

### Step 3: Update Variant Path Entries During Recursion Ascent

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Location**: After line 72 where we assemble the example for the current level, before line 106 where parent wrapping occurs

**Key Insight**: As we pop back up the recursion chain, we have the perfect examples at each level:
- At `.nested_config.0`: example = `1000000`
- At `.nested_config`: example = `{"Conditional": 1000000}`
- At root: example = `{"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}`

**Replace the complex parent wrapping (lines 106-111) with variant path updating:**

```rust
// After assembling the example (line 72), update variant_path entries in children
// This happens BEFORE we create our own MutationPathInternal
Self::update_child_variant_paths(
    &mut paths_to_expose,
    &ctx.mutation_path,
    &ctx.variant_chain,
    &example_to_use,
    enum_root_examples.as_ref(),
);
```

**Add the new helper method:**

```rust
/// Updates variant_path entries in child paths with level-appropriate examples
fn update_child_variant_paths(
    paths: &mut [MutationPathInternal],
    current_path: &str,
    current_variant_chain: &[VariantPathEntry],
    current_example: &Value,
    enum_examples: Option<&Vec<ExampleGroup>>,
) {
    // For each child path that has a path_requirement
    for child in paths.iter_mut() {
        if let Some(ref mut path_req) = child.path_requirement {
            // Find matching entry in child's variant_path that corresponds to our level
            for entry in path_req.variant_path.iter_mut() {
                if entry.path == current_path {
                    // This entry represents our current level - update it
                    entry.instructions = format!(
                        "Set {} to {}",
                        if entry.path.is_empty() { "root" } else { &entry.path },
                        &entry.variant
                    );

                    // If this is an enum and we have enum_examples, find the matching variant example
                    if let Some(examples) = enum_examples {
                        entry.example = examples
                            .iter()
                            .find(|ex| ex.applicable_variants.contains(&entry.variant))
                            .map(|ex| ex.example.clone())
                            .unwrap_or_else(|| current_example.clone());
                    } else {
                        // Non-enum case: use the assembled example
                        entry.example = current_example.clone();
                    }
                }
            }
        }
    }
}
```

**Modify PathRequirement creation at lines 416-420:**

```rust
Some(super::types::PathRequirement {
    description: Self::generate_variant_description(&ctx.variant_chain),
    example: example.clone(),
    // Initialize variant_path with basic info - examples will be filled during ascent
    variant_path: ctx.variant_chain
        .iter()
        .map(|entry| {
            super::types::VariantPathEntry {
                path: entry.path.clone(),
                variant: entry.variant.clone(),
                instructions: String::new(), // Will be filled during ascent
                example: json!(null),        // Will be filled during ascent
            }
        })
        .collect(),
})
```

**Note:** The variant_chain is populated during descent (line 217) with path and variant. During ascent, as we have the proper examples at each level, we update the variant_path entries in all child paths. This eliminates the need for complex example extraction or reconstruction.


### Step 4: Simplify PathRequirement Description

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Remove the `generate_variant_description` function (lines 566-593)** - it's no longer needed.

**Update PathRequirement creation at lines 416-420:**

```rust
Some(super::types::PathRequirement {
    description: if ctx.variant_chain.len() > 1 {
        format!("`{}` mutation path requires {} variant selections. Follow the instructions in variant_path array to set each variant in order.",
            ctx.mutation_path, ctx.variant_chain.len())
    } else {
        format!("`{}` mutation path requires a variant selection. See variant_path for instructions.",
            ctx.mutation_path)
    },
    example: example.clone(),
    // Enhance each variant_path entry with instructions and example
    variant_path: ctx.variant_chain
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            super::types::VariantPathEntry {
                path: entry.path.clone(),
                variant: entry.variant.clone(),
                instructions: format!(
                    "Set {} to {}",
                    if entry.path.is_empty() { "root" } else { &entry.path },
                    entry.variant
                ),
                example: Self::extract_example_for_variant_level(
                    &example,
                    &ctx.variant_chain,
                    i
                ),
            }
        })
        .collect(),
})
```

**Note:** The PathRequirement description now serves as a general instruction pointing to the variant_path array, while each VariantPathEntry contains specific actionable instructions.

## Advantages

✅ **Simpler implementation** - No complex JSON navigation or substitution, just direct updates during recursion ascent
✅ **More informative** - Step-by-step instruction manual with level-appropriate examples for agents
✅ **Leverages existing data** - Uses examples already available at each recursion level
✅ **Clean data flow** - Examples naturally propagate during recursion unwinding
✅ **Better separation of concerns** - variant_path becomes a complete instruction manual with context
✅ **Eliminates complexity** - Removes ~330 lines of complex parent wrapping logic (substitute_at_path, navigate_field, navigate_index)

## Testing

After implementation, the `.nested_config.0` path should show:

```json
"path_requirement": {
  "description": "`nested_config.0` mutation path requires 2 variant selections. Follow the instructions in variant_path array to set each variant in order.",
  "example": 1000000,
  "variant_path": [
    {
      "path": "",
      "variant": "TestEnumWithSerDe::Nested",
      "instructions": "Set root to TestEnumWithSerDe::Nested",
      "example": {"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}
    },
    {
      "path": ".nested_config",
      "variant": "NestedConfigEnum::Conditional",
      "instructions": "Set .nested_config to NestedConfigEnum::Conditional",
      "example": {"Conditional": 1000000}
    }
  ]
}
```

Each variant_path entry has the exact example needed at that level:
- Root level: Full `Nested` variant structure
- `.nested_config` level: Just the `Conditional` variant with its u32 value

This provides agents with clear step-by-step instructions with contextually appropriate examples, rather than requiring them to reverse-engineer from complex nested structures.