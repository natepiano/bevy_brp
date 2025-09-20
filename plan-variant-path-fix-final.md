# Fix PathRequirement Context Examples

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

### Step 1: Extend MaybeVariants Trait ⏳ PENDING
**Objective**: Add variant_signature() method with default implementation to support variant signature access
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`
**Change Type**: Additive
**Build Command**: `cargo build && cargo +nightly fmt`

### Step 2: Enhance PathKindWithVariants Structure ⏳ PENDING
**Objective**: Add variant_signature field and implement trait method for enum variant signature tracking
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
**Change Type**: Additive
**Dependencies**: Requires Step 1
**Build Command**: `cargo build && cargo +nightly fmt`

### Step 3: Add Helper Methods ⏳ PENDING
**Objective**: Implement update_variant_description and wrap_path_requirement_with_parent_info methods
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Change Type**: Additive
**Dependencies**: Requires Steps 1-2
**Build Command**: `cargo build && cargo +nightly fmt`

### Step 4: Update PathRequirement Processing Logic ⏳ PENDING
**Objective**: Modify process_all_children method to wrap PathRequirement examples with parent context
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Change Type**: Breaking (ATOMIC GROUP - must be done together to avoid breakage)
**Dependencies**: Requires Steps 1-3
**Build Command**: `cargo build && cargo +nightly fmt`

### Step 5: Complete Validation ⏳ PENDING
**Objective**: Run all tests and verify integration
**Change Type**: Validation
**Dependencies**: Requires Steps 1-4
**Build Command**: `cargo build && cargo +nightly fmt && cargo nextest run`

## Problem Statement

Comparing actual output to the reference `TestEnumWithSerde_mutation_paths.json`:

### What's Currently Happening

For `.nested_config.0` path, we're getting:
```json
{
  "path_requirement": {
    "description": "To use this mutation path, .nested_config must be set to NestedConfigEnum::Conditional",
    "example": 1000000,  // ← Just the raw value!
    "variant_path": [
      {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
      // ← Missing root entry!
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

### The Two Issues

1. **Incomplete variant_path**: Missing the root enum requirement (`TestEnumWithSerDe::Nested`)
2. **Wrong example format**: Shows just `1000000` instead of the complete nested structure needed to access that path

## Core Issue

When we recursively process `.nested_config.0`:
1. We correctly identify it needs `NestedConfigEnum::Conditional` variant
2. We build a PathRequirement with that information
3. BUT we only have the local context - we don't know `.nested_config` is inside `TestEnumWithSerDe::Nested`

The problem: PathRequirements are built bottom-up during recursion, but they need information that only exists at higher levels:
- `.nested_config.0` knows it needs `Conditional` at `.nested_config`
- But it doesn't know the parent field `.nested_config` is inside the `Nested` variant
- So it can't build the complete example showing both enum contexts

## Solution: Parent Example Replacement During Recursive Pop-Back

During recursive pop-back, each parent takes its ALREADY-COMPLETE example and creates PathRequirement examples for its children by REPLACING the specific child's field with that child's PathRequirement example. This happens as we pop back up the recursion stack.

## Implementation

### Step 1: Extend MaybeVariants Trait

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Add a method to access variant signature with a default implementation:

```rust
pub trait MaybeVariants {
    /// Returns the applicable variants for this path (if any)
    fn applicable_variants(&self) -> Option<&[String]> {
        None
    }

    /// Consumes self and returns the PathKind (if any)
    fn into_path_kind(self) -> Option<PathKind>;

    /// Returns the variant signature (if this is from an enum)
    /// Default implementation returns None for non-enum types
    fn variant_signature(&self) -> Option<&VariantSignature> {
        None
    }
}
```

### Step 2: Enhance PathKindWithVariants Structure

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`

Extend the existing `PathKindWithVariants` struct to include variant signature:

```rust
// In enum_builder.rs
#[derive(Debug, Clone)]
pub struct PathKindWithVariants {
    /// The path kind (None for unit variants)
    pub path: Option<PathKind>,
    /// Variants this path applies to
    pub applicable_variants: Vec<String>,
    /// The signature of these variants (Unit, Tuple, or Struct)
    pub variant_signature: VariantSignature,
}
```

Update `collect_children` method to populate the new `variant_signature` field:

```rust
// In collect_children method (lines 286-327):
match signature {
    VariantSignature::Unit => {
        // Unit variants have no path (no fields to mutate)
        children.push(PathKindWithVariants {
            path: None,
            applicable_variants,
            variant_signature: signature.clone(), // Populate new field
        });
    }
    VariantSignature::Tuple(types) => {
        // Create PathKindWithVariants for each tuple element
        for (index, type_name) in types.iter().enumerate() {
            children.push(PathKindWithVariants {
                path: Some(PathKind::IndexedElement {
                    index,
                    type_name: type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                }),
                applicable_variants: applicable_variants.clone(),
                variant_signature: signature.clone(), // Populate new field
            });
        }
    }
    VariantSignature::Struct(fields) => {
        // Create PathKindWithVariants for each struct field
        for (field_name, type_name) in fields {
            children.push(PathKindWithVariants {
                path: Some(PathKind::StructField {
                    field_name: field_name.clone(),
                    type_name: type_name.clone(),
                    parent_type: ctx.type_name().clone(),
                }),
                applicable_variants: applicable_variants.clone(),
                variant_signature: signature.clone(), // Populate new field
            });
        }
    }
}
```

Implement the new method for `PathKindWithVariants`:

```rust
impl MaybeVariants for PathKindWithVariants {
    fn applicable_variants(&self) -> Option<&[String]> {
        Some(&self.applicable_variants)
    }

    fn into_path_kind(self) -> Option<PathKind> {
        self.path
    }

    // Override the default to return the actual signature
    fn variant_signature(&self) -> Option<&VariantSignature> {
        Some(&self.variant_signature)
    }
}
```

### Step 3: Add Helper Methods

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Add these helper methods to the `MutationPathBuilder` implementation:

```rust
// In mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
impl<B: PathBuilder> MutationPathBuilder<B> {
    fn update_variant_description(
    &self,
    current_description: &str,
    parent_entry: &VariantPathEntry
) -> String {
    // Remove the common prefix if it exists
    let trimmed = current_description
        .trim_start_matches("To use this mutation path, ");

    if parent_entry.path.is_empty() {
        // Root level requirement
        format!("To use this mutation path, the root must be set to {} and {}",
            parent_entry.variant,
            trimmed
        )
    } else {
        // Field level requirement
        format!("To use this mutation path, {} must be set to {} and {}",
            parent_entry.path,
            parent_entry.variant,
            trimmed
        )
    }
}

// KEY INSIGHT: The parent ALREADY HAS a complete example built from all its children.
// This function takes that complete parent example and REPLACES just the specific
// field/index with the child's PathRequirement example.
//
// Example walkthrough for .nested_config.0:
//
// 1. At .nested_config.0 level:
//    - PathRequirement initially has example: 1000000
//
// 2. When .nested_config (parent) wraps this child:
//    - Parent already has complete example for NestedConfigEnum built from children
//    - Child PathRequirement example: 1000000
//    - Child is at index 0 of tuple variant NestedConfigEnum::Conditional
//    - Parent creates new example by placing child's value at index 0: {"Conditional": 1000000}
//    - This becomes the new PathRequirement example for .nested_config.0
//
// 3. When root wraps .nested_config:
//    - Root already has complete example: {"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}
//    - Child PathRequirement example: {"Conditional": 1000000} (from step 2)
//    - Child is the nested_config field of struct variant TestEnumWithSerDe::Nested
//    - Root REPLACES its nested_config field value with child's PathRequirement example
//    - Result: {"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}
//    - This becomes the final PathRequirement example for .nested_config.0
//
// CRITICAL: No defaults are generated. The parent's complete example already has
// all fields populated. We only REPLACE the specific field that corresponds to
// the child being processed.
fn wrap_path_requirement_with_parent_info(
    &self,
    parent_complete_example: &Value,  // Parent's COMPLETE example with all fields
    child_path_requirement_example: &Value,  // Child's PathRequirement example to insert
    variant_name: &str,
    variant_signature: &VariantSignature,
    path_kind: &PathKind  // Identifies which field/index to replace
) -> Result<Value> {
    match variant_signature {
        VariantSignature::Unit => {
            // Unit variant - just return the variant name
            Ok(json!(variant_name))
        }
        VariantSignature::Tuple(tuple_fields) => {
            // Get index directly from PathKind
            let index = match path_kind {
                PathKind::IndexedElement { index, .. } => *index,
                _ => return Err(Error::InvalidState("Expected indexed element for tuple variant".to_string()).into())
            };

            // Extract the tuple values from parent's complete example
            // Parent example is like: {"VariantName": [val0, val1, val2]}
            let parent_tuple_values = parent_complete_example
                .get(variant_name)
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::InvalidState("Parent example missing tuple values".to_string()))?;

            // Clone parent's values and replace the specific index
            let mut new_tuple_values = parent_tuple_values.clone();
            new_tuple_values[index] = child_path_requirement_example.clone();

            Ok(json!({ variant_name: new_tuple_values }))
        }
        VariantSignature::Struct(struct_fields) => {
            // Get field name directly from PathKind
            let field_name = match path_kind {
                PathKind::StructField { field_name, .. } => field_name.clone(),
                _ => return Err(Error::InvalidState("Expected struct field for struct variant".to_string()).into())
            };

            // Extract the struct fields from parent's complete example
            // Parent example is like: {"VariantName": {"field1": val1, "field2": val2}}
            let parent_struct_fields = parent_complete_example
                .get(variant_name)
                .and_then(|v| v.as_object())
                .ok_or_else(|| Error::InvalidState("Parent example missing struct fields".to_string()))?;

            // Clone parent's fields and replace the specific field
            let mut new_field_values = parent_struct_fields.clone();
            new_field_values.insert(field_name, child_path_requirement_example.clone());

            Ok(json!({ variant_name: new_field_values }))
        }
    }
}

}
```

### Step 4: Update PathRequirement Processing Logic

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

The PathRequirement wrapping needs to happen AFTER `assemble_from_children` when the parent has its complete example. In the `build_paths` method, after the assembled_example is created:

```rust
// In mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
// In build_paths method, AFTER assemble_from_children has created assembled_example

// First, process all children and collect their paths
let ChildProcessingResult {
    all_paths: mut child_paths,
    child_examples,
    // ... other fields
} = self.process_all_children(ctx, depth)?;

// Assemble THIS level from children (creates the complete example)
let assembled_example = match self.inner.assemble_from_children(ctx, child_examples) {
    Ok(example) => example,
    Err(e) => {
        return Self::handle_assemble_error(ctx, e);
    }
};

// NEW: PathRequirement wrapping logic
// Now that we have the parent's complete example, wrap child PathRequirements
// This only happens if this parent is an enum variant
if let Some(enum_ctx) = &ctx.enum_context {
    if let Some(EnumContext::Child { variant_chain }) = enum_ctx {
        // Get the variant info for this parent from the chain
        if let Some(parent_variant) = variant_chain.last() {
            // For each child path with a PathRequirement, wrap it with parent context
            for child_path in &mut child_paths {
                if let Some(ref mut path_req) = child_path.path_requirement {
                    // The key insight: assembled_example already has ALL fields populated
                    // We create a new example for the PathRequirement by REPLACING
                    // the specific field with the child's PathRequirement example

                    // Prepend parent's variant requirement to the chain
                    path_req.variant_path.insert(0, parent_variant.clone());

                    // Update the description to include parent requirement
                    path_req.description = self.update_variant_description(
                        &path_req.description,
                        parent_variant
                    );

                    // CRITICAL: Use parent's complete example and replace the field
                    path_req.example = self.wrap_path_requirement_with_parent_info(
                        &assembled_example,        // Parent's COMPLETE assembled example
                        &path_req.example,         // Child's PathRequirement example to insert
                        &parent_variant.variant,   // Parent's variant name
                        // Need variant signature and path_kind from somewhere...
                        // This requires storing them during child processing
                    )?;
                }
            }
        }
    }
}
```

Note: The exact implementation requires tracking variant signatures and PathKinds during child processing so they're available when wrapping PathRequirements.

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Design Review Skip Notes

### IMPLEMENTATION-1: Missing concrete insertion point for process_all_children modifications - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Implementation
- **Issue**: Plan shows code to be added but doesn't specify exactly where in the existing method to insert it
- **Reasoning**: The plan already contains clear insertion point specification in lines 77-141 of the Implementation section, showing the exact sequence: extract variant info → extract PathKind → process child → store example → add PathRequirement wrapping logic → extend collections
- **Decision**: User elected to skip this recommendation

### IMPLEMENTATION-GAP-1: Plan doesn't address how child_examples HashMap is maintained across recursion - **Verdict**: CONFIRMED → RESOLVED
- **Status**: APPROVED - Implemented
- **Location**: Section: Key Insight
- **Issue**: Plan mentions using 'sibling_examples' and 'child_examples' but doesn't explain how these are populated and passed through the recursive call stack
- **Reasoning**: The finding was initially correct - the plan had an incorrect understanding. After investigation, we clarified that the correct approach is parent example replacement during recursion pop-back. The parent already has a complete example and replaces specific fields with child PathRequirement examples.
- **Resolution**: Renamed function to wrap_path_requirement_with_parent_info, added parent_complete_example parameter, removed any concept of generating defaults, added detailed walkthrough showing the replacement process: parent example with "Always" → replaced with {"Conditional": 1000000}
- **Decision**: Plan updated with correct replacement approach

## How It Works: Parent Example Replacement Process

Starting with `.nested_config.0`:

1. **Initial state** (at `.nested_config.0` level):
   - `description = "To use this mutation path, .nested_config must be set to NestedConfigEnum::Conditional"`
   - `example = 1000000`
   - `variant_path = [{"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}]`

2. **Pop to `.nested_config`** (NestedConfigEnum):
   - Parent (`.nested_config`) has already built its complete example from all children
   - For child `.nested_config.0` with PathRequirement example `1000000`:
     - Takes Conditional variant structure
     - Places child's value at index 0: `{"Conditional": 1000000}`
     - This becomes the new PathRequirement example for `.nested_config.0`

3. **Pop to root** (TestEnumWithSerDe):
   - Root has already built its complete example: `{"Nested": {"nested_config": "Always", "other_field": "Hello, World!"}}`
   - For child `.nested_config.0` with PathRequirement example `{"Conditional": 1000000}`:
     - Takes its complete Nested variant example
     - **REPLACES** the `nested_config` field with child's PathRequirement example
     - Result: `{"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}`
   - Also updates metadata:
     - Prepends variant entry: `{"path": "", "variant": "TestEnumWithSerDe::Nested"}`
     - Updates description to include root requirement
     - Final variant_path: `[{"path": "", "variant": "TestEnumWithSerDe::Nested"}, {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}]`

## Result

Each PathRequirement shows the complete context needed from root to that specific path, built naturally through the recursion pop-back process. All three fields (description, variant_path, and example) are updated consistently as we traverse back up the stack.