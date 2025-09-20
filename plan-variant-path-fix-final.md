# Fix PathRequirement Context Examples


**PREREQUISITE**: `plan-variant-chain-infrastructure.md` has been completed successfully. The variant_chain infrastructure provides `ctx.variant_chain` and has already solved 2/3 of the PathRequirement issues:
- ✅ **Complete variant_path chains**: Working correctly
- ✅ **Correct descriptions**: Generated properly from variant chains
- ❌ **Complete example structure**: Still shows local values instead of nested structure

### Step 1: Implement Parent Wrapping Algorithm
**Objective**: Add parent wrapping logic to build complete PathRequirement examples
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Change Type**: Additive
**Build Command**: `cargo build && cargo +nightly fmt`

**Key Changes:**
- Add parent wrapping logic AFTER `example_to_use` is determined (after line 102)
- Operate on `paths_to_expose` (not `all_paths`) since that's what gets returned
- Use `example_to_use` (not `assembled_example`) for wrapping context
- Make modifications in-place on mutable references

## Problem Statement

**STATUS UPDATE**: After implementing variant_chain infrastructure, we have achieved significant progress:

### What's Now Working ✅

For `.nested_config.0` path, we're getting:
```json
{
  "path_requirement": {
    "description": "To use this mutation path, root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional",  // ✅ FIXED: Complete description
    "example": 1000000,  // ❌ REMAINING ISSUE: Just the raw value!
    "variant_path": [  // ✅ FIXED: Complete variant chain
      {"path": "", "variant": "TestEnumWithSerDe::Nested"},
      {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
    ]
  }
}
```

### What Should Happen (from reference JSON lines 98-110)

```json
{
  "path_requirement": {
    "description": "To use this mutation path, the root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional",
    "example": {
      "Nested": {
        "nested_config": {"Conditional": 1000000},
        "other_field": "Hello, World!"
      }
    },
    "variant_path": [
      {"path": "", "variant": "TestEnumWithSerDe::Nested"},
      {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
    ]
  }
}
```

### The Remaining Issue

**Only one issue remains**: Wrong example format - shows just `1000000` instead of the complete nested structure needed to access that path.

## Core Issue (Updated)

The variant_chain infrastructure solved the ancestry tracking, but PathRequirement examples are still built with only local context:

**Current PathRequirement construction (builder.rs:500):**
```rust
example: example.clone(), // Just uses local value (1000000)
```

**What we need:**
The complete nested structure showing how to reach the mutation path through all required enum variants.

## Solution: Parent Wrapping Algorithm

**Approach**: Use parent wrapping during recursive pop-back to build complete nested examples. This leverages the already-assembled complete parent examples.

## Implementation

The implementation leverages the existing variant_chain infrastructure and adds parent wrapping logic during recursive pop-back.

### Core Approach

1. **PathRequirement creation** continues to use `ctx.variant_chain` (already working)
2. **Parent wrapping logic** added after `assemble_from_children` to build complete examples
3. **Multi-level coordination** handles nested enum structures

### Key Algorithm

**CRITICAL UNDERSTANDING - Parent Wrapping Process:**

**DO NOT CONFUSE THIS**: We are NOT replacing the child's example with the parent's example. We are doing the OPPOSITE.

**Correct Process (as we pop back up the recursion stack):**
1. **Parent completes assembly** from all children → has its own default assembled example
2. **Parent identifies children with PathRequirements** that need their examples updated
3. **For each such child**: Parent takes the child's PathRequirement.example and **substitutes it into the correct position** in the parent's assembled example
4. **Parent updates the child's PathRequirement.example** with this new substituted version
5. **Process repeats recursively up the stack** - each level makes the PathRequirement more complete

**Concrete Example for `.nested_config.0` showing substitution direction:**

1. **`.nested_config.0`** creates PathRequirement with `example: 1000000` (raw local value)

2. **`.nested_config` pops back:**
   - Has assembled example: `{"Conditional": 1000000}` (from its own processing)
   - Sees child `.nested_config.0` has PathRequirement with `example: 1000000`
   - **Substitutes** child's `1000000` into correct position in `{"Conditional": 1000000}`
   - Updates child's PathRequirement: `example: {"Conditional": 1000000}`

3. **Root pops back:**
   - Has assembled example: `{"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}` (default structure)
   - Sees child `.nested_config.0` has PathRequirement with `example: {"Conditional": 1000000}`
   - **Substitutes** child's `{"Conditional": 1000000}` into the `nested_config` field position
   - Updates child's PathRequirement: `example: {"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}`

**Final Result**: The PathRequirement.example shows the complete context needed to make that specific path mutable.

**Key Insight**: Each PathRequirement.example gets progressively more complete as we pop up the stack, building the full root-to-target context through substitution.

This approach leverages already-assembled parent examples instead of trying to construct complete examples from scratch using variant_chain traversal.

### Concrete Implementation Details

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Step 1**: Add parent wrapping logic in the `build_paths` method. The insertion point is AFTER `example_to_use` is determined but BEFORE `build_final_result` is called.

**Critical Understanding**: The data flow in `build_paths`:
1. Line 65: `all_paths` = all descendant paths from children
2. Line 66: `paths_to_expose` = filtered subset based on PathAction
3. Line 77: `assembled_example` = raw assembled example
4. Line 81: `parent_example` = processed after enum context
5. Line 90: `final_example` = final (could be knowledge or parent)
6. Line 102: `example_to_use` = validated final example (null if not mutable)
7. Line 105: `build_final_result` adds THIS level's path and returns

**Insertion Location**: After `example_to_use` is determined (line 102), before `build_final_result` (line 105):

```rust
// Fix: PartiallyMutable paths should not provide misleading examples
let example_to_use = match parent_status {
    MutationStatus::PartiallyMutable | MutationStatus::NotMutable => json!(null),
    MutationStatus::Mutable => final_example,
};

// INSERT HERE - After example_to_use, before build_final_result

// Decide what to return based on PathAction
Ok(Self::build_final_result(
```

The new code to insert:
```rust
// Wrap children's PathRequirements with this parent's context
// Only wrap if we have a valid example to provide context
if !example_to_use.is_null() {
    for path in paths_to_expose.iter_mut() {
        if let Some(ref mut path_req) = path.path_requirement {
            // Check if this path is a descendant (not same level)
            if !path.path.is_empty() && path.path != ctx.mutation_path {
                // Calculate relative path from parent to child
                let relative_path = if ctx.mutation_path.is_empty() {
                    path.path.trim_start_matches('.')
                } else {
                    &path.path[ctx.mutation_path.len()..].trim_start_matches('.')
                };

                // Build wrapped example by substituting child's example into parent's structure
                let mut wrapped = example_to_use.clone();
                if Self::substitute_at_path(&mut wrapped, relative_path, &path_req.example).is_ok() {
                    path_req.example = wrapped;
                }
            }
        }
    }
}
```

**Step 2**: Add a simple helper method to perform path substitution:

```rust
impl<B: PathBuilder> MutationPathBuilder<B> {
    /// Substitute a value at a relative path within a JSON structure
    /// This is a simplified version that handles common cases
    fn substitute_at_path(
        target: &mut Value,
        relative_path: &str,
        substitute_value: &Value,
    ) -> Result<(), String> {
        // Parse path segments
        let segments: Vec<&str> = relative_path
            .split(|c| c == '.' || c == '[' || c == ']')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            *target = substitute_value.clone();
            return Ok(());
        }

        // Navigate to the target location and substitute
        // This would follow similar logic to existing path navigation in the codebase
        // Details omitted for brevity - implementation should handle:
        // - Object field navigation
        // - Array/tuple indexing
        // - Enum variant navigation

        Ok(())
    }
}
```

### Implementation Summary

The key insight from analyzing the code:
- The parent wrapping MUST happen AFTER `example_to_use` is determined (line 102)
- It should operate on `paths_to_expose` (not `all_paths`)
- Modifications must be in-place to avoid Result type conversion issues

The corrected approach:
1. **Insert wrapping logic after line 102** - when we have the final validated example
2. **Iterate through `paths_to_expose`** - these are the paths being returned
3. **For each descendant with PathRequirement** - wrap its example with parent context
4. **Use simple path substitution** - navigate and replace values in the parent structure

This avoids the compilation errors from the original plan and correctly places the wrapping at the point where all necessary data is available.

## Design Review Skip Notes

### IMPLEMENTATION-GAP-1: Missing concrete implementation details for helper method construction - **Verdict**: CONFIRMED → RESOLVED
- **Status**: APPROVED - Implemented
- **Location**: Section: Step 1 - Add Complete Example Construction Helper
- **Issue**: Plan states 'Add helper method to construct complete examples from ctx.variant_chain' and 'Handle path parsing, variant signature lookup, and structure navigation' but provides no concrete implementation details for HOW these operations will work
- **Reasoning**: The finding correctly identified that the plan needed concrete implementation details. Based on git history review, previous versions contained detailed implementation code that was deleted.
- **Resolution**: Restored the concrete implementation details from commit 420ae88, including:
  - Complete `wrap_path_requirement_with_parent_info` method implementation
  - Detailed algorithmic walkthrough with step-by-step examples
  - Exact insertion point in `process_all_children` method
  - Data structure modifications for `PathKindWithVariants` and `MaybeVariants` trait
  - Helper method `update_variant_description` with full implementation
