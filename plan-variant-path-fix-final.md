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

**Step 1**: Make `paths_to_expose` mutable in the destructuring at line 66:

```rust
let ChildProcessingResult {
    all_paths,
    mut paths_to_expose,
    child_examples,
} = self.process_all_children(ctx, depth)?;
```

**Step 2**: Add imports at the top of the file for error handling:

```rust
use crate::error::{Error, Result};
use error_stack::Report;
```

**Step 3**: Add parent wrapping logic in the `build_paths` method. The insertion point is AFTER `example_to_use` is determined but BEFORE `build_final_result` is called.

**Critical Understanding**: The data flow in `build_paths`:
1. Line 65: `all_paths` = all descendant paths from children
2. Line 66: `paths_to_expose` = filtered subset based on PathAction (NOW MUTABLE)
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

The new code to insert (VARIANT-AWARE WRAPPING ALGORITHM):
```rust
// Wrap children's PathRequirements with this parent's context
// Handle both non-enum and enum root cases with variant-aware selection
tracing::debug!(
    "Parent wrapping check: example_to_use.is_null()={}, enum_root_examples.is_some()={}, mutation_path='{}', paths_to_expose.len()={}",
    example_to_use.is_null(),
    enum_root_examples.is_some(),
    ctx.mutation_path,
    paths_to_expose.len()
);
if !example_to_use.is_null() || enum_root_examples.is_some() {
    tracing::debug!("Starting variant-aware parent wrapping loop for {} paths", paths_to_expose.len());
    for path in &mut paths_to_expose {
        tracing::debug!("Checking path: '{}' with path_requirement: {}", path.path, path.path_requirement.is_some());
        if let Some(ref mut path_req) = path.path_requirement {
            // Check if this path is a descendant (not same level)
            tracing::debug!("Path '{}' has requirement, checking if descendant: path.is_empty()={}, path != ctx.mutation_path={}",
                path.path, path.path.is_empty(), path.path != ctx.mutation_path);
            if !path.path.is_empty() && path.path != ctx.mutation_path {
                // Determine wrapping example based on parent type
                let wrapping_example = if !example_to_use.is_null() {
                    // Non-enum case: use the computed example
                    tracing::debug!("Using non-enum example for path '{}'", path.path);
                    example_to_use.clone()
                } else if let Some(ref examples) = enum_root_examples {
                    // Enum root case: find example that matches child's variant requirements
                    tracing::debug!("Finding matching enum example for path '{}' with variant_path: {:?}",
                        path.path, path_req.variant_path);
                    Self::find_matching_enum_example(examples, &path_req.variant_path, &ctx.mutation_path)
                        .unwrap_or_else(|| {
                            tracing::warn!("No matching enum example found for path '{}', using first available", path.path);
                            examples.first().map(|ex| ex.example.clone()).unwrap_or(json!(null))
                        })
                } else {
                    tracing::debug!("No wrapping example available for path '{}'", path.path);
                    continue; // No example available
                };

                if !wrapping_example.is_null() {
                    // Calculate relative path from parent to child
                    let relative_path = if ctx.mutation_path.is_empty() {
                        path.path.trim_start_matches('.')
                    } else {
                        path.path[ctx.mutation_path.len()..].trim_start_matches('.')
                    };

                    // Build wrapped example by substituting child's example into parent's structure
                    let mut wrapped = wrapping_example;
                    if Self::substitute_at_path(&mut wrapped, relative_path, &path_req.example).is_ok() {
                        tracing::debug!("Successfully wrapped path '{}' with variant-aware example", path.path);
                        path_req.example = wrapped;
                    } else {
                        tracing::warn!("Failed to substitute at path '{}' for variant-aware wrapping", relative_path);
                    }
                } else {
                    tracing::debug!("Skipping wrapping for path '{}' - null wrapping example", path.path);
                }
            }
        }
    }
}
```

**Step 4**: Add helper method for variant-aware example selection:

```rust
impl<B: PathBuilder> MutationPathBuilder<B> {
    /// Find enum example that matches child's variant requirements
    fn find_matching_enum_example(
        examples: &[super::types::ExampleGroup],
        child_variant_path: &[super::types::VariantPathEntry],
        current_path: &str,
    ) -> Option<Value> {
        // Find the variant requirement for the current level
        let variant_for_current_level = child_variant_path
            .iter()
            .find(|entry| entry.path == current_path)?
            .variant
            .clone();

        tracing::debug!(
            "Looking for enum example with variant '{}' at path '{}'",
            variant_for_current_level,
            current_path
        );

        // Find example that contains this variant
        examples
            .iter()
            .find(|ex| {
                let matches = ex.applicable_variants.contains(&variant_for_current_level);
                tracing::debug!(
                    "Checking example with variants {:?}: matches={}",
                    ex.applicable_variants,
                    matches
                );
                matches
            })
            .map(|ex| {
                tracing::debug!("Found matching example: {}", ex.example);
                ex.example.clone()
            })
    }
}
```

**Step 5**: Add helper methods to perform path substitution - broken into smaller functions for clippy compliance:

```rust
impl<B: PathBuilder> MutationPathBuilder<B> {
    /// Navigate to a numeric index in the JSON structure
    fn navigate_index<'a>(
        current: &'a mut Value,
        index: usize,
        is_last: bool,
        substitute_value: &Value,
    ) -> Result<Option<&'a mut Value>> {
        match current {
            Value::Array(arr) => {
                if index >= arr.len() {
                    return Err(Report::new(Error::SchemaProcessing {
                        message: "Index out of bounds".to_string(),
                        type_name: None,
                        operation: Some("path substitution".to_string()),
                        details: Some(format!("Index {index} exceeds array length")),
                    }));
                }
                if is_last {
                    arr[index] = substitute_value.clone();
                    return Ok(None);
                }
                Ok(Some(&mut arr[index]))
            }
            Value::Object(obj) if obj.len() == 1 => {
                // Enum variant with tuple - navigate into it
                let variant_value = obj.values_mut().next().ok_or_else(|| {
                    Report::new(Error::SchemaProcessing {
                        message: "Invalid enum variant structure".to_string(),
                        type_name: None,
                        operation: Some("path substitution".to_string()),
                        details: Some("Enum variant object has no values".to_string()),
                    })
                })?;

                // Handle single-element tuple (value stored directly)
                if index == 0 && !variant_value.is_array() {
                    if is_last {
                        *variant_value = substitute_value.clone();
                        return Ok(None);
                    }
                    return Ok(Some(variant_value));
                }
                if let Value::Array(arr) = variant_value {
                    if index >= arr.len() {
                        return Err(Report::new(Error::SchemaProcessing {
                            message: "Index out of bounds".to_string(),
                            type_name: None,
                            operation: Some("path substitution".to_string()),
                            details: Some(format!("Tuple index {index} exceeds length")),
                        }));
                    }
                    if is_last {
                        arr[index] = substitute_value.clone();
                        return Ok(None);
                    }
                    Ok(Some(&mut arr[index]))
                } else {
                    Err(Report::new(Error::SchemaProcessing {
                        message: "Type mismatch for path operation".to_string(),
                        type_name: None,
                        operation: Some("path substitution".to_string()),
                        details: Some("Cannot index into non-array variant value".to_string()),
                    }))
                }
            }
            _ => Err(Report::new(Error::SchemaProcessing {
                message: "Type mismatch for path operation".to_string(),
                type_name: None,
                operation: Some("path substitution".to_string()),
                details: Some("Cannot index into non-array value".to_string()),
            })),
        }
    }

    /// Navigate to a field in the JSON structure
    fn navigate_field<'a>(
        current: &'a mut Value,
        segment: &str,
        is_last: bool,
        substitute_value: &Value,
    ) -> Result<Option<&'a mut Value>> {
        match current {
            Value::Object(obj) => {
                // Check if this is an enum variant
                if obj.len() == 1 && !obj.contains_key(segment) {
                    // Navigate into the enum variant first
                    let variant_value = obj.values_mut().next().ok_or_else(|| {
                        Report::new(Error::SchemaProcessing {
                            message: "Invalid enum variant structure".to_string(),
                            type_name: None,
                            operation: Some("path navigation".to_string()),
                            details: Some("Enum variant object has no values".to_string()),
                        })
                    })?;
                    if variant_value.is_object() {
                        // Recurse into the variant's object
                        Self::navigate_field(variant_value, segment, is_last, substitute_value)
                    } else {
                        Err(Report::new(Error::SchemaProcessing {
                            message: "Type mismatch for field navigation".to_string(),
                            type_name: None,
                            operation: Some("path navigation".to_string()),
                            details: Some(format!(
                                "Cannot navigate field '{segment}' in non-object variant"
                            )),
                        }))
                    }
                } else {
                    // Regular object field navigation
                    if is_last {
                        obj.insert(segment.to_string(), substitute_value.clone());
                        return Ok(None);
                    }
                    obj.get_mut(segment)
                        .map(Some)
                        .ok_or_else(|| {
                            Report::new(Error::SchemaProcessing {
                                message: "Field not found in object".to_string(),
                                type_name: None,
                                operation: Some("path navigation".to_string()),
                                details: Some(format!("Field '{segment}' does not exist")),
                            })
                        })
                }
            }
            _ => Err(Report::new(Error::SchemaProcessing {
                message: "Type mismatch for field access".to_string(),
                type_name: None,
                operation: Some("path navigation".to_string()),
                details: Some(format!("Cannot access field '{segment}' in non-object")),
            })),
        }
    }

    /// Substitute a value at a relative path within a JSON structure
    fn substitute_at_path(
        target: &mut Value,
        relative_path: &str,
        substitute_value: &Value,
    ) -> Result<()> {
        // Parse path segments (handling both . and [] notation)
        let segments: Vec<&str> = relative_path
            .split(['.', '[', ']'])
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            *target = substitute_value.clone();
            return Ok(());
        }

        // Navigate through the structure
        let mut current = target;
        let mut segments_iter = segments.iter().peekable();

        while let Some(segment) = segments_iter.next() {
            let is_last = segments_iter.peek().is_none();

            if let Ok(index) = segment.parse::<usize>() {
                // Numeric segment - index into array or tuple
                if let Some(next) = Self::navigate_index(current, index, is_last, substitute_value)? {
                    current = next;
                } else {
                    return Ok(()); // Value was substituted
                }
            } else {
                // String segment - field name
                if let Some(next) = Self::navigate_field(current, segment, is_last, substitute_value)? {
                    current = next;
                } else {
                    return Ok(()); // Value was substituted
                }
            }
        }

        Ok(())
    }
}
```

### Implementation Summary

The key insight from analyzing the code:
- The parent wrapping MUST happen AFTER `example_to_use` is determined (line 102)
- It should operate on `paths_to_expose` (not `all_paths`)
- Modifications must be in-place to avoid Result type conversion issues
- `paths_to_expose` must be made mutable in the destructuring

The corrected approach:
1. **Insert wrapping logic after line 102** - when we have the final validated example
2. **Make `paths_to_expose` mutable** - add `mut` in the destructuring at line 66
3. **Iterate through `paths_to_expose`** - these are the paths being returned
4. **For each descendant with PathRequirement** - wrap its example with parent context
5. **Use simple path substitution** - navigate and replace values in the parent structure

### Critical Implementation Details Learned

**Error Handling Requirements:**
- MUST use `Error::SchemaProcessing` for all errors in this context, NOT `Error::General`
- All errors MUST be wrapped with `error_stack::Report::new()`
- The Result type is `Result<(), error_stack::Report<Error>>` not `Result<(), String>`
- Must import `error_stack::Report` and `crate::error::{Error, Result}`

**Example of correct error construction:**
```rust
return Err(Report::new(Error::SchemaProcessing {
    message: "Index out of bounds".to_string(),
    type_name: None,
    operation: Some("path substitution".to_string()),
    details: Some(format!("Index {} exceeds array length", index)),
}));
```

**Error propagation with `?` operator:**
```rust
current = obj.get_mut(*segment)
    .ok_or_else(|| Report::new(Error::SchemaProcessing {
        message: "Field not found".to_string(),
        type_name: None,
        operation: Some("path navigation".to_string()),
        details: Some(format!("Field '{}' does not exist", segment)),
    }))?;
```

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
