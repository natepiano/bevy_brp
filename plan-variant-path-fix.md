# Plan: Fix Variant Path Implementation Issues
fix for ./plan-variant-path.md issues that we see in production

## Target Structure Reference
See TestEnumWithSerde_mutation_paths.json for the correct target structure we're trying to achieve.

## Issue 1: Variant Name Format

### Problem
Variant names are being duplicated with the type name appearing twice.

### Current Output
```json
"variant": "Handle<Image>::Handle<Image>::Weak"
```

### Expected Output
```json
"variant": "Handle<Image>::Weak"
```

### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
Function: `process_all_children()` method
Lines: 272 and 283 where `ctx.type_name().variant_name(&variant)` is called
Note: The `variant_name()` method exists and works correctly in `brp_type_name.rs`

### Root Cause
The enum_builder.rs already provides fully qualified variant names (e.g., "Handle<Image>::Weak") in the `applicable_variants` field. The builder.rs then incorrectly calls `variant_name()` again on these already-formatted names, causing duplication.

### Proposed Fix
Remove the redundant `variant_name()` calls in builder.rs since the variants are already properly formatted:

```rust
// builder.rs line 272 - change from:
variant: ctx.type_name().variant_name(&variant),
// to:
variant: variant.clone(),

// builder.rs line 283 - change from:
variant: ctx.type_name().variant_name(variant),
// to:
variant: variant.clone(),
```

## Issue 2: PathRequirement.example Structure

### Problem
The PathRequirement.example is wrapped in an incorrect structure with `applicable_variants` and `value` fields.

### Current Output
```json
"example": {
  "applicable_variants": ["Handle<Image>::Handle<Image>::Weak"],
  "value": {
    "Index": {
      "index": {
        "generation": 1000000,
        "index": 1000000
      }
    }
  }
}
```

### Expected Output
```json
"example": {
  "Weak": [
    {
      "Index": {
        "index": {
          "generation": 1000000,
          "index": 1000000
        }
      }
    }
  ]
}
```

### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
Function: `generate_path_internal()` method
Approximate lines: ~530-540 where `PathRequirement` struct is constructed with description, example, and variant_path fields
Note: Line numbers are approximate and may shift during implementation of other fixes

### Proposed Fix
The example should show the complete setup value for the parent path, not wrapped in metadata.

## Issue 3: PathRequirement.description Path Reference

### Problem
The description incorrectly refers to "the root" when it should reference the specific parent path.

### Current Output
```json
"description": "To use this mutation path, the root must be set to Handle<Image>::Handle<Image>::Weak"
```

### Expected Output
```json
"description": "To use this mutation path, .color_lut must be set to Handle<Image>::Weak"
```

### Current Code
Location: Need to find the `generate_variant_description` function or equivalent

### Proposed Fix
Update the description generation to properly reference the parent path from the variant_path entry.

```rust
fn generate_variant_description(variant_chain: &[VariantPathEntry]) -> String {
    if variant_chain.len() == 1 {
        let entry = &variant_chain[0];
        if entry.path.is_empty() {
            format!("To use this mutation path, the root must be set to {}", entry.variant)
        } else {
            format!("To use this mutation path, {} must be set to {}", entry.path, entry.variant)
        }
    } else {
        // Handle multiple requirements...
    }
}
```

## Issue 4: Missing Examples Array for Enum Paths

### Problem
Enum paths like `.color_lut.0` should have an `examples` array showing all variants, but instead have a single malformed `example`.

### Current Output
```json
".color_lut.0": {
  "example": {
    "applicable_variants": ["Handle<Image>::Handle<Image>::Weak"],
    "value": {
      "Index": {
        "index": {
          "generation": 1000000,
          "index": 1000000
        }
      }
    }
  }
}
```

### Expected Output
```json
".color_lut.0": {
  "examples": [
    {
      "applicable_variants": ["AssetId<Image>::Index"],
      "example": {
        "Index": {
          "index": {
            "generation": 1000000,
            "index": 1000000
          }
        }
      },
      "signature": "struct{index: AssetIndex}"
    },
    {
      "applicable_variants": ["AssetId<Image>::Uuid"],
      "example": {
        "Uuid": {
          "uuid": "550e8400-e29b-41d4-a716-446655440000"
        }
      },
      "signature": "struct{uuid: Uuid}"
    }
  ]
}
```

### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
Function: Logic that sets `enum_context` for child contexts
Approximate lines: 380-385 (same location as Issue 7) - need to verify IndexedElement handling
Note: Line numbers are approximate and may shift during implementation of other fixes

### Proposed Fix
When an IndexedElement's type is an enum, it should get `EnumContext::Root` to generate proper examples array.

## Issue 5: Example Wrapper in Mutation Path

### Problem
The mutation path's own example is being wrapped with `applicable_variants` which should not be there.

### Current Output
```json
"example": {
  "applicable_variants": ["..."],
  "value": {...}
}
```

### Expected Output
```json
"example": {...}
```
Or for enum types:
```json
"examples": [
  {
    "applicable_variants": ["..."],
    "example": {...},
    "signature": "..."
  }
]
```

### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
Function: `from_mutation_path_internal()` method
Approximate lines: 194-205 where `examples` and `example` fields are determined from `enum_root_examples`
Note: Line numbers are approximate and may shift during implementation of other fixes

### Proposed Fix
Ensure the example extraction doesn't wrap non-enum examples in metadata structures.

## Issue 7: IndexedElement Enum Handling

### Problem
When an IndexedElement (like `.0`) points to an enum type within a tuple variant, it's not generating proper enum examples.

### Current Code
File: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` lines 380-385
```rust
if matches!(child_kind, TypeKind::Enum)
    && child_ctx.enum_context.is_none()
    && matches!(child_ctx.path_kind, PathKind::StructField { .. })
{
    child_ctx.enum_context = Some(super::recursion_context::EnumContext::Root);
}
```

### Proposed Fix
Also check for IndexedElement and ArrayElement paths that point to enum types:

```rust
// If child is an enum and we're building a non-root path for it, set EnumContext::Root
// This ensures the enum generates proper examples for its mutation path
if matches!(child_kind, TypeKind::Enum)
    && child_ctx.enum_context.is_none()
    && (matches!(child_ctx.path_kind, PathKind::StructField { .. })
        || matches!(child_ctx.path_kind, PathKind::IndexedElement { .. })
        || matches!(child_ctx.path_kind, PathKind::ArrayElement { .. }))
{
    child_ctx.enum_context = Some(super::recursion_context::EnumContext::Root);
}
```
