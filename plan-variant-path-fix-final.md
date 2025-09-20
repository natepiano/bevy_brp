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

## Solution: Recursive Wrapping During Pop-Back

Build PathRequirement examples recursively as we pop back up the stack. Each parent wraps its children's PathRequirement examples with its own context.

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

// Example walkthrough for .nested_config.0:
// 1. .nested_config.0 level:
//    - PathRequirement initially has example: 1000000
//    - When .nested_config (parent) processes this child, it sees:
//        - Child example: 1000000
//        - Child is index 0 of a tuple variant NestedConfigEnum::Conditional
//        - Calls wrap_path_requirement_with_parent_info(1000000, "Conditional", Tuple(u32), IndexedElement{index: 0})
//        - Result: {"Conditional": 1000000} (tuple with child's value at index 0)
// 2. .nested_config level:
//    - PathRequirement now has example: {"Conditional": 1000000}
//    - When root processes this child, it sees:
//        - Child example: {"Conditional": 1000000}
//        - Child is struct field nested_config of variant TestEnumWithSerDe::Nested
//        - Calls wrap_path_requirement_with_parent_info({"Conditional": 1000000}, "Nested", Struct{nested_config, other_field}, StructField{nested_config})
//        - Result: {"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}} (struct with child's value in nested_config field, default for other_field)
//
// Each level wraps its child's PathRequirement example with its own variant context as we pop up the recursion stack.
fn wrap_path_requirement_with_parent_info(
    &self,
    child_example: &Value,
    variant_name: &str,
    variant_signature: &VariantSignature,  // Now passed in directly from enum builder
    path_kind: &PathKind  // Use PathKind which has typed information
) -> Result<Value> {
    match variant_signature {
        VariantSignature::Unit => {
            // Unit variant - just return the variant name
            Ok(json!(variant_name))
        }
        VariantSignature::Tuple(tuple_fields) => {
            // Get index directly from PathKind - no parsing needed
            let index = match path_kind {
                PathKind::IndexedElement { index, .. } => *index,
                _ => return Err(Error::InvalidState("Expected indexed element for tuple variant".to_string()).into())
            };

            // Build the tuple values
            let mut tuple_values = Vec::new();
            for (i, field_type) in tuple_fields.iter().enumerate() {
                if i == index {
                    // This is the child's position - use its example
                    tuple_values.push(child_example.clone());
                } else {
                    // Use default value for this field type
                    tuple_values.push(Self::default_value_for_type(field_type));
                }
            }

            Ok(json!({ variant_name: tuple_values }))
        }
        VariantSignature::Struct(struct_fields) => {
            // Build struct with example at the right field
            let mut field_values = serde_json::Map::new();

            // Get field name directly from PathKind - no parsing needed
            let field_name = match path_kind {
                PathKind::StructField { field_name, .. } => field_name.clone(),
                _ => return Err(Error::InvalidState("Expected struct field for struct variant".to_string()).into())
            };

            // struct_fields is Vec<(String, BrpTypeName)>
            for (field, field_type) in &struct_fields {
                if field == &field_name {
                    // Add the current field with its example
                    field_values.insert(field_name.clone(), child_example.clone());
                } else {
                    // Add other fields with default values
                    field_values.insert(field.clone(), Self::default_value_for_type(field_type));
                }
            }

            Ok(json!({ variant_name: field_values }))
        }
    }
}

}
```

### Step 4: Update PathRequirement Processing Logic

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

In the `process_all_children` method, after line 355 where we get `(child_paths, child_example)`:

```rust
// In mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
// In process_all_children, we need to iterate through items and track their PathKind
// Note: This is simplified - actual implementation needs to handle the loop properly

for item in child_items {
    // Extract variant information and PathKind from the item
    let variant_names = item.applicable_variants().map(<[String]>::to_vec);
    let variant_signature = item.variant_signature().cloned();

    // Extract the PathKind before consuming the item
    if let Some(path_kind) = item.into_path_kind() {
        // Create child_key from the path_kind for process_child
        let child_key = MutationPathDescriptor::from_path_kind(&path_kind);
        let mut child_ctx = ctx.create_recursion_context(path_kind.clone(), PathAction::Create);

        let (mut child_paths, child_example) =
            Self::process_child(&child_key, &mut child_ctx, depth)?;

        // Store the child example for later sibling field population
        child_examples.insert(child_key.clone(), child_example);

        // NEW: PathRequirement wrapping logic insertion point
        // If this parent is part of an enum that requires specific variants,
        // update all child PathRequirements with parent's context
        if let (Some(variants), Some(signature)) = (variant_names, variant_signature) {
            let variant_name = variants.first()
                .ok_or_else(|| Error::InvalidState("No variants available".to_string()))?
                .clone();

            // Create parent's variant entry
            let parent_variant_entry = VariantPathEntry {
                path: ctx.mutation_path.clone(),  // Parent's path
                variant: variant_name.clone(),
            };

            for path in &mut child_paths {
                if let Some(ref mut path_req) = path.path_requirement {
                    // Prepend parent's variant requirement to the chain
                    path_req.variant_path.insert(0, parent_variant_entry.clone());

                    // Update the description to include parent requirement
                    path_req.description = self.update_variant_description(
                        &path_req.description,
                        &parent_variant_entry
                    );

                    // Wrap PathRequirement with parent variant context
                    // Pass the PathKind directly - no string parsing needed!
                    path_req.example = self.wrap_path_requirement_with_parent_info(
                        &path_req.example,
                        &variant_name,
                        &signature,
                        &path_kind  // Pass PathKind instead of MutationPathDescriptor
                    )?;
                }
            }
        }

        all_paths.extend(child_paths);
    }
}
```

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
- **Reasoning**: The finding was initially correct - the plan described a flawed approach using sibling_examples. After investigation, we clarified that the correct approach is parent wrapping during recursion pop-back, not sibling collection. Updated the plan to remove sibling_examples parameter and use default values for non-target fields.
- **Resolution**: Renamed function to wrap_path_requirement_with_parent_info, removed sibling_examples parameter, added detailed walkthrough comment showing the step-by-step example transformation from 1000000 → {"Conditional": 1000000} → complete nested structure
- **Decision**: Plan updated with correct approach

## How It Works

Starting with `.nested_config.0`:

1. **Initial state** (at `.nested_config.0` level):
   - `description = "To use this mutation path, .nested_config must be set to NestedConfigEnum::Conditional"`
   - `example = 1000000`
   - `variant_path = [{"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}]`

2. **Pop to `.nested_config`** (NestedConfigEnum):
   - `.nested_config` is itself an enum field, but not inside another enum variant at this level
   - Sees child `.nested_config.0` has PathRequirement
   - No parent variant to prepend (`.nested_config` itself is the enum)
   - Wraps example with Conditional variant: `{"Conditional": 1000000}` (no array brackets - it's a tuple with one element)

3. **Pop to root** (TestEnumWithSerDe):
   - Root is an enum with Nested variant containing `.nested_config`
   - Sees `.nested_config` and `.nested_config.0` paths have PathRequirements
   - Prepends its variant requirement: `{"path": "", "variant": "TestEnumWithSerDe::Nested"}`
   - Updates description: `"To use this mutation path, the root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional"`
   - Now variant_path = `[{"path": "", "variant": "TestEnumWithSerDe::Nested"}, {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}]`
   - Wraps example with struct fields: `{"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}`

## Result

Each PathRequirement shows the complete context needed from root to that specific path, built naturally through the recursion pop-back process. All three fields (description, variant_path, and example) are updated consistently as we traverse back up the stack.