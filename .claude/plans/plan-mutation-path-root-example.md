# Plan: Provide Correct Root Examples for Enum Chain Mutation Paths

## Executive Summary

Fix mutation paths within enum chains to provide a single, correct root example instead of multi-step instructions with incorrect intermediate states.

## Scope

**This plan applies ONLY to mutation paths that traverse enum variant chains** - specifically paths where `enum_variant_path` is populated, indicating the path requires variant selection to be valid.

### Affected Paths
- Paths like `.middle_struct.nested_enum.name` that go through enum variants
- Any path where `enum_variant_path.len() > 0`
- Paths requiring one or more enum variant selections to reach the target field

### NOT Affected
- Direct struct field mutations (`.some_struct.field`)
- Tuple element access (`.some_tuple.0`)
- Array/Vec indexing (`.items[0]`)
- Any path that doesn't require enum variant selection

## The Core Problem

Looking at the type guide output in `TestVariantChainEnum.json` (generated from the `extras_plugin::TestVariantChainEnum` type), the mutation path `.middle_struct.nested_enum.name` demonstrates the issue:

### Current Behavior (Incorrect)
From `TestVariantChainEnum.json` lines 196-222:
```json
"enum_variant_path": [
  {
    "path": "",
    "variant_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": { "VariantA": 1000000 }  // ❌ WRONG!
        }
      }
    }
  },
  {
    "path": ".middle_struct.nested_enum",
    "variant_example": {
      "VariantB": {  // ✅ Correct, but requires second step
        "name": "Hello, World!",
        "value": 3.14
      }
    }
  }
]
```

**The Problem**: The `.name` field only exists in `VariantB`, but the root example shows `VariantA`. This forces a confusing 2-step correction process.

## Root Cause Analysis

After investigating the codebase:

1. **We correctly track variant chains** - `RecursionContext.variant_chain` accurately records the sequence of variants needed
2. **We correctly identify which paths need variants** - `enum_variant_path` is properly populated
3. **The bug**: When building the root example in `update_child_variant_paths`, we use the wrong nested enum variant

The issue is in `builder.rs` lines 567-575 where we populate `variant_example`. For root entries, we're not looking ahead to see which nested variant is actually required by the full path.

## The Solution

Replace the multi-step `enum_variant_path` with a single `root_variant_example` that contains the complete, correct variant chain from the start.

### Desired Behavior (Correct)
```json
{
  "path": ".middle_struct.nested_enum.name",
  "example": "Hello, World!",
  "path_info": {
    "enum_instructions": "Set root to the provided root_variant_example to enable this mutation",
    "root_variant_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": {
            "VariantB": {  // ✅ Correct variant from the start!
              "name": "Hello, World!",
              "value": 3.14
            }
          },
          "some_field": "test",
          "some_value": 42.0
        }
      }
    }
  }
}
```

## Implementation Strategy

### Phase 1: Build Variant Chain Lookup

Since we already track variant chains during recursion, we need to:

1. **Collect variant combinations during enum processing**
   - In `enum_path_builder::process_enum`, build examples for each variant combination
   - Store these in a lookup map: `HashMap<Vec<VariantName>, Value>`

2. **Example entries in the map** (for `TestVariantChainEnum`):
   ```
   ["WithMiddleStruct", "VariantA"] → Complete root with nested_enum as VariantA
   ["WithMiddleStruct", "VariantB"] → Complete root with nested_enum as VariantB
   ["WithMiddleStruct", "VariantC"] → Complete root with nested_enum as VariantC
   ```

### Phase 2: Propagate Correct Examples

Modify `update_child_variant_paths` in `builder.rs`:

```rust
fn update_child_variant_paths(
    paths: &mut [MutationPathInternal],
    current_path: &str,
    current_example: &Value,
    enum_examples: Option<&Vec<ExampleGroup>>,
    variant_lookup: Option<&HashMap<Vec<VariantName>, Value>>, // NEW
) {
    for child in paths.iter_mut() {
        if !child.enum_variant_path.is_empty() {
            // For root entries, use the complete variant chain to lookup correct example
            if current_path.is_empty() && variant_lookup.is_some() {
                let full_chain: Vec<VariantName> = child.enum_variant_path
                    .iter()
                    .map(|vp| vp.variant.clone())
                    .collect();

                if let Some(correct_example) = variant_lookup.get(&full_chain) {
                    child.root_variant_example = Some(correct_example.clone());
                }
            }
        }
    }
}
```

### Phase 3: Update Output Format

Add `root_variant_example` field to `MutationPathInternal`:
```rust
pub struct MutationPathInternal {
    // ... existing fields ...

    /// Complete root example with correct variant chain
    pub root_variant_example: Option<Value>,
}
```

### Phase 4: Simplify Instructions

Replace complex multi-step instructions with a single, clear instruction:
- **Old**: "Follow the 2-step process in enum_variant_path array"
- **New**: "Set root to the provided root_variant_example to enable this mutation"

## Migration Path

1. **Add new field** - Add `root_variant_example` alongside existing `enum_variant_path`
2. **Parallel operation** - Populate both fields during transition
3. **Verify correctness** - Compare that root examples have correct variants
4. **Switch consumers** - Update AI agents to use `root_variant_example`
5. **Remove old system** - Eventually deprecate `enum_variant_path` array

## Success Criteria

1. **Correctness**: Every enum chain path has a root example with the RIGHT variant chain
2. **Simplicity**: Single mutation operation instead of multi-step process
3. **Clarity**: Simple instruction instead of complex array navigation
4. **No wrong variants**: Examples never show variants that lack the target field

## Testing Strategy

1. **Unit tests**: Verify variant chain tracking and lookup
2. **Integration tests**: Test with `extras_plugin::TestVariantChainEnum` type (which generates `TestVariantChainEnum.json`)
3. **Deep nesting**: Validate with 3+ level enum chains
4. **AI agent testing**: Confirm agents can use the new format successfully

## Example: Complete Transformation

### Before (Current System)
- 2+ step process with `enum_variant_path` array
- First step has wrong variant requiring correction
- Complex instructions about following array steps
- Confusing for AI agents

### After (With This Plan)
- Single `root_variant_example` with correct structure
- One mutation to set root, then direct field access
- Simple, clear instruction
- AI agents succeed on first attempt

## Notes

- This leverages existing variant chain tracking - we're not adding new tracking, just fixing the example generation
- The core insight: we already know the full variant chain when building examples, we just need to use it
- This is a surgical fix to the specific subset of paths that traverse enum variants
