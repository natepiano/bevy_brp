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
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
Lines: 395-404 and 451-460

### Root Cause
`MutationExample::EnumChild` is wrapping the example with `applicable_variants` metadata. This metadata is already passed through the `MaybeVariants` trait during `collect_children`, so the wrapper is redundant and pollutes the JSON output.

### Proposed Fix
Remove `MutationExample::EnumChild` entirely and use `MutationExample::Simple` instead:

1. **Remove the `EnumChild` variant** from `MutationExample` enum (lines ~34-37)
2. **Change `EnumContext::Child` case** (lines 395-404) to return `MutationExample::Simple(example)` instead of `EnumChild`
3. **Remove the `EnumChild` match arm** (lines 451-460) in the JSON conversion
4. **Remove `flatten_variant_chain` function** (lines 268-280) as it's only used by `EnumChild`

The `applicable_variants` information flows through `MaybeVariants` for use in building both `ExampleGroup` objects and `variant_path` entries, so `EnumChild` is unnecessary.

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
Lines: 380-385

### Root Cause
This is a consequence of Issue 2. The wrapped example from `MutationExample::EnumChild` prevents proper enum handling. Once Issue 2 is fixed, this should resolve automatically.

### Proposed Fix
After fixing Issue 2, verify that IndexedElement paths pointing to enums get `EnumContext::Root` set correctly (already happens at lines 380-385 for StructField).

## Issue 5: Example Wrapper in Mutation Path

### Problem
Same as Issue 2 - the mutation path's example is being wrapped with `applicable_variants` metadata.

### Root Cause
This is the same issue as Issue 2. `MutationExample::EnumChild` creates the wrapper.

### Proposed Fix
Fixed by Issue 2's solution - removing `MutationExample::EnumChild`.

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
