# Fix PathRequirement Context Examples

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

### Key Insight

In `process_child` (line 377 of builder.rs), after recursion returns:
- We get back `Vec<MutationPathInternal>` containing all descendant paths
- These paths have PathRequirements with `variant_chain` but wrong examples
- The parent can wrap these examples with its context

### Code Location

After line 355 in `process_all_children` where we get `(child_paths, child_example)`:

```rust
// In process_all_children, after line 355:
let (mut child_paths, child_example) =
    Self::process_child(&child_key, &mut child_ctx, depth)?;

// If this parent is part of an enum that requires specific variants,
// update all child PathRequirements with parent's context
if let Some(variants) = &applicable_variants {
    // Determine which variant entry to prepend to variant_path
    let parent_variant_entry = VariantPathEntry {
        path: ctx.mutation_path.clone(),  // Parent's path (e.g., "" for root, ".nested_config" for field)
        variant: variants.first().unwrap().clone(),  // The variant needed at parent level
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

            // Wrap the example with this parent's variant context
            path_req.example = self.wrap_with_variant_context(
                &path_req.example,
                variants,
                &child_key,
                &child_examples
            )?;
        }
    }
}

child_examples.insert(child_key, child_example);
all_paths.extend(child_paths.clone());
```

### Helper Methods

```rust
fn update_variant_description(
    &self,
    current_description: &str,
    parent_entry: &VariantPathEntry
) -> String {
    if parent_entry.path.is_empty() {
        // Root level requirement
        format!("To use this mutation path, the root must be set to {} and {}",
            parent_entry.variant,
            current_description.trim_start_matches("To use this mutation path, ")
        )
    } else {
        // Field level requirement - just prepend
        format!("To use this mutation path, {} must be set to {} and {}",
            parent_entry.path,
            parent_entry.variant,
            current_description.trim_start_matches("To use this mutation path, ")
        )
    }
}

fn wrap_with_variant_context(
    &self,
    example: &Value,
    variants: &[String],  // The applicable variants at this level
    field_descriptor: &MutationPathDescriptor,
    sibling_examples: &HashMap<MutationPathDescriptor, Value>
) -> Result<Value> {
    // Determine which variant to use (could be from variant_chain or default)
    let variant_name = variants.first()
        .ok_or_else(|| Error::InvalidState("No variants available"))?;

    // Build the wrapped structure based on the variant type
    match self.determine_variant_type(variant_name)? {
        VariantType::Unit => {
            // Unit variant - just return the variant name
            Ok(json!(variant_name))
        }
        VariantType::Tuple(size) => {
            // Build tuple with example at the right position
            let mut tuple_values = vec![json!(null); size];
            let index = field_descriptor.to_index()?;
            tuple_values[index] = example.clone();
            Ok(json!({ variant_name: tuple_values }))
        }
        VariantType::Struct(fields) => {
            // Build struct with example at the right field
            let mut field_values = serde_json::Map::new();

            // Add the current field
            let field_name = field_descriptor.to_field_name()?;
            field_values.insert(field_name, example.clone());

            // Add sibling fields with their examples
            for (sibling_key, sibling_example) in sibling_examples {
                if sibling_key != field_descriptor {
                    let sibling_name = sibling_key.to_field_name()?;
                    field_values.insert(sibling_name, sibling_example.clone());
                }
            }

            Ok(json!({ variant_name: field_values }))
        }
    }
}
```

## How It Works

Starting with `.nested_config.0`:

1. **Initial state**:
   - `description = "To use this mutation path, .nested_config must be set to NestedConfigEnum::Conditional"`
   - `example = 1000000`
   - `variant_path = [{"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}]`

2. **Pop to `.nested_config`** (NestedConfigEnum):
   - Sees child has PathRequirement
   - Already has correct variant_path for its level (no parent enum to add)
   - Description stays the same (no parent enum context to add)
   - Wraps example: `{"Conditional": [1000000]}`

3. **Pop to root** (TestEnumWithSerDe):
   - Sees `.nested_config` and `.nested_config.0` paths have PathRequirements
   - Prepends its variant requirement: `{"path": "", "variant": "TestEnumWithSerDe::Nested"}`
   - Updates description: `"To use this mutation path, the root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional"`
   - Now variant_path = `[{"path": "", "variant": "TestEnumWithSerDe::Nested"}, {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}]`
   - Wraps examples: `{"Nested": {"nested_config": {"Conditional": [1000000]}, "other_field": "..."}}`

## Result

Each PathRequirement.example shows the complete context needed from root to that specific path, built naturally through the recursion pop-back process.