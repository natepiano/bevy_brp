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
- Add `wrap_children_path_requirements` method after `assemble_from_children`
- Add `wrap_descendant` and `substitute_at_relative_path` helper methods
- Update PathRequirement examples to show complete nested structures

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

**Step 1**: Add parent wrapping logic in the `build_paths` method. The insertion point is after `assembled_example` is created but BEFORE it gets processed for enum contexts.

**Insertion Location**: After the assembled_example is created and before the enum context processing starts.

Look for this code pattern:
```rust
// Assemble THIS level from children (post-order)
let assembled_example = match self.inner.assemble_from_children(ctx, child_examples) {
    Ok(example) => example,
    Err(e) => {
        // Use helper method to handle NotMutatable errors cleanly
        return Self::handle_assemble_error(ctx, e);
    }
};

// INSERT PARENT WRAPPING LOGIC HERE - BEFORE ENUM CONTEXT PROCESSING

// Process the assembled example based on EnumContext
// Extract enum_root_examples if this is an enum root
tracing::debug!(
    "Processing assembled example for {} with path '{}' and enum_context: {:?}",
    ...
);
```

The new code to insert:
```rust
// NEW: PathRequirement parent wrapping logic
// Wrap children's PathRequirement examples with parent context
Self::wrap_children_path_requirements(&mut all_paths, &assembled_example, ctx, &child_examples)?;
```

**Step 2**: Add the helper method to perform parent wrapping:

```rust
impl<B: PathBuilder> MutationPathBuilder<B> {
    /// Wrap children's PathRequirement examples with parent context
    /// ALL descendants with PathRequirements get wrapped, not just direct children,
    /// to ensure complete examples show full root-to-leaf context
    fn wrap_children_path_requirements(
        all_paths: &mut Vec<MutationPathInternal>,
        assembled_example: &Value,
        ctx: &RecursionContext,
        child_examples: &HashMap<MutationPathDescriptor, Value>,
    ) -> Result<()> {
        tracing::debug!(
            "wrap_children_path_requirements: parent at '{}', parent PathKind={:?}, processing {} paths",
            ctx.mutation_path,
            ctx.path_kind,
            all_paths.len()
        );

        // Only process if we have children with PathRequirements
        if all_paths.is_empty() {
            tracing::debug!("No paths to wrap - returning early");
            return Ok(());
        }

        for path in all_paths.iter_mut() {
            if let Some(ref mut path_req) = path.path_requirement {
                tracing::debug!(
                    "Processing PathRequirement for descendant at '{}', PathKind={:?}, current example type: {}",
                    path.path,
                    path.path_kind,
                    match &path_req.example {
                        Value::Object(_) => "Object",
                        Value::Array(_) => "Array",
                        Value::String(_) => "String",
                        Value::Number(_) => "Number",
                        Value::Bool(_) => "Bool",
                        Value::Null => "Null",
                    }
                );

                // Check if this path is a descendant that should be wrapped
                let should_wrap = Self::should_wrap_descendant(&path.path, &ctx.mutation_path);

                if !should_wrap {
                    tracing::debug!(
                        "Path '{}' is not a descendant of '{}' - skipping",
                        path.path,
                        ctx.mutation_path
                    );
                    continue;
                }

                // Build the wrapped example by incorporating the child's variant requirements
                // into the parent's structure
                match Self::wrap_descendant_with_parent_context(
                    &path_req.example,
                    assembled_example,
                    &path.path,
                    &ctx.mutation_path,
                    &path.path_kind,
                    &ctx.path_kind,
                ) {
                    Ok(wrapped_example) => {
                        tracing::debug!(
                            "Successfully wrapped example for path '{}', result type: {}",
                            path.path,
                            match &wrapped_example {
                                Value::Object(_) => "Object",
                                Value::Array(_) => "Array",
                                Value::String(_) => "String",
                                Value::Number(_) => "Number",
                                Value::Bool(_) => "Bool",
                                Value::Null => "Null",
                            }
                        );
                        // Update the descendant's PathRequirement with fuller context
                        path_req.example = wrapped_example;
                    }
                    Err(e) => {
                        return Err(Error::SchemaProcessing {
                            message: "Failed to wrap PathRequirement example".to_string(),
                            type_name: Some(ctx.type_name().display_name()),
                            operation: Some("parent wrapping".to_string()),
                            details: Some(format!("Failed at path '{}': {}", path.path, e)),
                        }.into());
                    }
                }
            } else {
                tracing::debug!("Path '{}' has no PathRequirement - skipping", path.path);
            }
        }

        Ok(())
    }

    /// Check if a path is a descendant of the parent path
    fn should_wrap_descendant(descendant_path: &str, parent_path: &str) -> bool {
        // Root (empty path) wraps everything
        if parent_path.is_empty() {
            return !descendant_path.is_empty();
        }

        // Check if descendant starts with parent path
        if !descendant_path.starts_with(parent_path) {
            return false;
        }

        // Ensure proper boundary (next char should be '.' or '[')
        let remainder = &descendant_path[parent_path.len()..];
        remainder.is_empty() || remainder.starts_with('.') || remainder.starts_with('[')
    }

    /// Wrap a descendant's example with parent context, preserving variant requirements
    fn wrap_descendant_with_parent_context(
        descendant_example: &Value,
        parent_assembled: &Value,
        descendant_path: &str,
        parent_path: &str,
        descendant_kind: &PathKind,
        parent_kind: &PathKind,
    ) -> Result<Value> {
        tracing::debug!(
            "wrap_descendant: descendant_path='{}', parent_path='{}', descendant_kind={:?}, parent_kind={:?}",
            descendant_path,
            parent_path,
            descendant_kind,
            parent_kind
        );

        // Calculate the relative path from parent to descendant
        let relative_path = if parent_path.is_empty() {
            descendant_path.trim_start_matches('.')
        } else {
            &descendant_path[parent_path.len()..].trim_start_matches('.')
        };

        tracing::debug!("Relative path from parent to descendant: '{}'", relative_path);

        // Start with parent's assembled structure
        let mut result = parent_assembled.clone();

        // Navigate to the descendant's position and substitute its example
        if relative_path.is_empty() {
            // Same level - replace entire structure
            result = descendant_example.clone();
        } else {
            // Parse the relative path and navigate to substitution point
            Self::substitute_at_relative_path(&mut result, relative_path, descendant_example)?;
        }

        Ok(result)
    }

    /// Substitute a value at a relative path within a JSON structure
    fn substitute_at_relative_path(
        target: &mut Value,
        relative_path: &str,
        substitute_value: &Value,
    ) -> Result<()> {
        tracing::debug!(
            "substitute_at_relative_path: path='{}', target_type={:?}, substitute_type={:?}",
            relative_path,
            match target {
                Value::Object(_) => "Object",
                Value::Array(_) => "Array",
                _ => "Other"
            },
            match substitute_value {
                Value::Object(_) => "Object",
                Value::Array(_) => "Array",
                _ => "Other"
            }
        );

        // Split path into segments (handling both . and [] notation)
        let segments: Vec<&str> = relative_path
            .split(|c| c == '.' || c == '[' || c == ']')
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
                tracing::debug!("Navigating to index {}", index);

                match current {
                    Value::Array(ref mut arr) => {
                        if index >= arr.len() {
                            return Err(Error::SchemaProcessing {
                                message: "Index out of bounds".to_string(),
                                type_name: None,
                                operation: Some("path substitution".to_string()),
                                details: Some(format!("Index {index} exceeds array length")),
                            }.into());
                        }
                        if is_last {
                            arr[index] = substitute_value.clone();
                            return Ok(());
                        }
                        current = &mut arr[index];
                    }
                    Value::Object(ref mut obj) if obj.len() == 1 => {
                        // Enum variant with tuple - navigate into it
                        let variant_value = obj.values_mut().next().unwrap();

                        // Handle single-element tuple (value stored directly)
                        if index == 0 && !variant_value.is_array() {
                            if is_last {
                                *variant_value = substitute_value.clone();
                                return Ok(());
                            }
                            current = variant_value;
                        } else if let Value::Array(ref mut arr) = variant_value {
                            if index >= arr.len() {
                                return Err(Error::SchemaProcessing {
                                    message: "Index out of bounds".to_string(),
                                    type_name: None,
                                    operation: Some("path substitution".to_string()),
                                    details: Some(format!("Index {index} exceeds tuple length")),
                                }.into());
                            }
                            if is_last {
                                arr[index] = substitute_value.clone();
                                return Ok(());
                            }
                            current = &mut arr[index];
                        } else {
                            return Err(Error::SchemaProcessing {
                                message: "Type mismatch for path operation".to_string(),
                                type_name: None,
                                operation: Some("path substitution".to_string()),
                                details: Some("Cannot index into non-array variant value".to_string()),
                            }.into());
                        }
                    }
                    _ => return Err(Error::SchemaProcessing {
                        message: "Type mismatch for path operation".to_string(),
                        type_name: None,
                        operation: Some("path substitution".to_string()),
                        details: Some(format!("Cannot index into value of type {current:?}")),
                    }.into())
                }
            } else {
                // String segment - field name
                tracing::debug!("Navigating to field '{}'", segment);

                match current {
                    Value::Object(ref mut obj) => {
                        // Check if this is an enum variant
                        if obj.len() == 1 && !obj.contains_key(segment) {
                            // Navigate into the enum variant first
                            let variant_value = obj.values_mut().next().unwrap();
                            if let Value::Object(inner) = variant_value {
                                current = variant_value;
                                // Now we're inside the variant, continue to the field
                                if let Value::Object(ref mut inner_obj) = current {
                                    if is_last {
                                        inner_obj.insert(segment.to_string(), substitute_value.clone());
                                        return Ok(());
                                    }
                                    current = inner_obj.get_mut(segment)
                                        .ok_or_else(|| Error::SchemaProcessing {
                                            message: "Field not found in object".to_string(),
                                            type_name: None,
                                            operation: Some("path navigation".to_string()),
                                            details: Some(format!("Field '{segment}' does not exist")),
                                        }.into())?;
                                } else {
                                    return Err(Error::SchemaProcessing {
                                        message: "Unexpected variant structure".to_string(),
                                        type_name: None,
                                        operation: Some("path navigation".to_string()),
                                        details: Some("Expected object inside variant".to_string()),
                                    }.into());
                                }
                            } else {
                                return Err(Error::SchemaProcessing {
                                    message: "Type mismatch for field navigation".to_string(),
                                    type_name: None,
                                    operation: Some("path navigation".to_string()),
                                    details: Some(format!("Cannot navigate field '{segment}' in non-object variant")),
                                }.into());
                            }
                        } else {
                            // Regular object field navigation
                            if is_last {
                                obj.insert(segment.to_string(), substitute_value.clone());
                                return Ok(());
                            }
                            current = obj.get_mut(segment)
                                .ok_or_else(|| anyhow!("Field '{}' not found", segment))?;
                        }
                    }
                    _ => return Err(Error::SchemaProcessing {
                        message: "Type mismatch for field access".to_string(),
                        type_name: None,
                        operation: Some("path navigation".to_string()),
                        details: Some(format!("Cannot access field '{segment}' in non-object")),
                    }.into())
                }
            }
        }

        Ok(())
    }

// Note: The old direct-child-only helper functions have been removed
// in favor of wrapping all descendants to ensure complete examples

}


```

### Implementation Summary

The implementation adds a single insertion point in `build_paths` after `assembled_example` is available, and provides helper methods to:

1. **Identify children with PathRequirements** (iterate through `all_paths`)
2. **Parse mutation paths** (e.g., ".nested_config.0" → field + index)
3. **Substitute child examples** into the correct position within parent's assembled example
4. **Update PathRequirement examples** with the wrapped complete structure

This approach leverages the existing `assembled_example` that already contains the complete parent structure with all sibling fields populated, and simply substitutes each child's PathRequirement example into the correct location to show the full mutation context.

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
