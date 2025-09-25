# Plan: Fix Root Examples Using Variant Chain as Lookup Key

## Executive Summary

Fix mutation paths within enum chains by using the variant chain as a lookup key for correct root examples, propagating all paths during recursion and only deduplicating at output time.

## The Core Insight

The `variant_chain` that we already track (e.g., `["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantB"]`) should be the key to lookup the correct root example. The current system deduplicates too early, losing the ability to build all necessary variant combinations.

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

## The Problem with Current Flow

Looking at the type guide output in `TestVariantChainEnum.json` (generated from the `extras_plugin::TestVariantChainEnum` type):

### Current (Broken) Flow
1. `BottomEnum` processes variants, groups by signature
2. Returns ONE path per signature group to parent
3. Parent (`MiddleStruct`) only gets one example per signature
4. Root (`TestVariantChainEnum`) can only build one combination
5. Result: Wrong variant in root example

### Example of the Problem
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

**The Problem**: The `.name` field only exists in `VariantB`, but the root example shows `VariantA`. This happens because when `BottomEnum` returns to its parent, it only returns one example per signature group.

## The Solution: Variant Chain as Lookup Key

### Needed Flow
1. `BottomEnum` processes ALL variants individually
2. Returns ALL paths to parent (no deduplication yet)
3. Parent gets all variant examples
4. Root builds ALL possible variant chain combinations
5. Store each in map: `VariantChain → RootExample`
6. At output, deduplicate paths but lookup correct root via chain

## Implementation Strategy

### Phase 1: Propagate All Paths During Recursion

Change enum processing to return all paths, not deduplicated:

```rust
// In enum_path_builder.rs
fn process_children() -> Result<(HashMap<MutationPathDescriptor, Value>, Vec<MutationPathInternal>)> {
    // IMPORTANT: We still need signature grouping to prevent HashMap key collisions
    // (e.g., VariantA(i32) and VariantB(i32) both create descriptor "0")
    // But we process ALL variants, not just representatives

    let variant_groups = group_variants_by_signature(all_variants);

    for (signature, variants_in_group) in variant_groups {
        // Process EACH variant in the group, not just one representative
        for variant in variants_in_group {
            // Build path for THIS specific variant
            // Add variant to chain
            // Recurse
            // Collect ALL paths (don't deduplicate)
        }
    }

    // Return ALL paths to parent
    return (child_examples, all_child_paths);  // No deduplication!
}
```

**Key Insight**: We must maintain signature grouping to avoid HashMap collisions where multiple variants would create the same `MutationPathDescriptor`. But within each signature group, we process every variant individually to build all variant chains.

### Phase 2: Build Variant Chain Map at Root

At root level assembly, build a map of variant chains to complete root examples:

```rust
/// Maps variant chains to complete root examples
type VariantChainMap = HashMap<Vec<VariantName>, Value>;

fn finalize_mutation_paths(
    paths: &mut Vec<MutationPathInternal>,
    variant_map: VariantChainMap,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_data {
            // Use the variant chain to lookup the correct root example
            if !enum_data.variant_chain.is_empty() {
                enum_data.variant_chain_root_example =
                    variant_map.get(&enum_data.variant_chain).cloned();
            }
        }
    }
}
```

### Phase 3: Deduplicate at Output Only

Move all deduplication to the output stage:

```rust
fn prepare_output(all_paths: Vec<MutationPathInternal>) -> Vec<MutationPath> {
    // Group paths by (full_mutation_path, signature)
    let mut groups: HashMap<(FullMutationPath, Signature), Vec<MutationPathInternal>> = HashMap::new();

    for path in all_paths {
        let key = (path.full_mutation_path.clone(), path.signature());
        groups.entry(key).or_default().push(path);
    }

    // For each group, pick ONE representative but preserve ALL variant info
    let mut output_paths = vec![];
    for ((full_path, sig), paths_in_group) in groups {
        let representative = paths_in_group.first().unwrap();

        // The representative already has applicable_variants from enum processing
        // It contains ALL variants with this signature, not just processed ones
        let applicable_variants = representative.enum_data
            .as_ref()
            .map(|d| d.applicable_variants.clone())
            .unwrap_or_default();

        // Use the variant_chain_root_example from the representative's enum_data
        let root_example = representative.enum_data
            .as_ref()
            .and_then(|d| d.variant_chain_root_example.clone());

        output_paths.push(MutationPath {
            path: full_path,
            example: representative.example,
            path_info: PathInfo {
                applicable_variants,  // Shows ALL variants that work here
                root_variant_example: root_example,  // Correct for THIS chain
                // ... other fields ...
            }
        });
    }

    output_paths
}
```

## Key Changes from Current System

1. **No Early Deduplication**: Enums return ALL paths during recursion
2. **Variant Chain as Key**: Use complete variant chain to lookup root examples
3. **Late Deduplication**: Only deduplicate at final output stage
4. **Preserve Variant Info**: Track applicable_variants for each output path

## Data Structure Changes

### Group Enum-Related Fields for Ergonomics

Create a dedicated struct to group all enum-related data:

```rust
pub struct EnumPathData {
    /// The complete variant chain from root (e.g., ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantB"])
    pub variant_chain: Vec<VariantName>,

    /// ALL variants that share the EXACT same signature at this path's level
    /// (same field names AND types - e.g., VariantB and VariantD both have name: String, value: f32)
    pub applicable_variants: Vec<VariantName>,

    /// The correct root example for THIS specific variant chain
    pub variant_chain_root_example: Option<Value>,

    /// Instructions for setting variants (temporary during migration)
    pub enum_instructions: Option<String>,
}

pub struct MutationPathInternal {
    // ... existing non-enum fields ...
    pub example: Value,
    pub full_mutation_path: FullMutationPath,
    pub type_name: BrpTypeName,
    pub path_kind: PathKind,
    pub mutation_status: MutationStatus,
    pub mutation_status_reason: Option<Value>,

    /// All enum-related data grouped together
    /// None for paths that don't involve enums
    pub enum_data: Option<EnumPathData>,
}
```

This grouping provides better ergonomics:
- Path builders that don't handle enums can ignore `enum_data` entirely
- Enum-aware code can check `if let Some(enum_data) = path.enum_data`
- All enum logic is cleanly separated

### How applicable_variants Works

During enum processing, track ALL variants with the EXACT same signature:

```rust
// When processing a signature group with multiple variants
// These must have IDENTICAL field names, not just same types
let signature_group = vec!["BottomEnum::VariantB", "BottomEnum::VariantD"];

// Even if we only process one path for efficiency
let representative_path = process_variant("VariantB");

// Store ALL variants that could use this path
representative_path.enum_data.as_mut().unwrap().applicable_variants = signature_group;
```

### Output Format Updates
Add `applicable_variants` to path_info and use correct root example:
```json
{
  "path": ".middle_struct.nested_enum.name",
  "example": "Hello, World!",
  "path_info": {
    "applicable_variants": ["BottomEnum::VariantB"],  // NEW
    "root_variant_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": {
            "VariantB": {  // ✅ Correct variant from the start!
              "name": "Hello, World!",
              "value": 3.14
            }
          }
        }
      }
    }
  }
}
```

## Why This Works

- Each mutation path carries its complete variant chain during recursion
- We build root examples for ALL variant chains, not just signature representatives
- The variant chain becomes the key to lookup the correct root example
- Output deduplication preserves all necessary information while avoiding redundancy

## Success Criteria

1. **Correct Variants**: `.middle_struct.nested_enum.name` has root example with `VariantB`, not `VariantA`
2. **Complete Chain Map**: All variant chain combinations have corresponding root examples
3. **No Early Loss**: All paths propagate up during recursion
4. **Smart Deduplication**: Output shows one path per signature with correct root and applicable variants
5. **Variant Transparency**: `applicable_variants` clearly shows which variants support each path

## Testing Strategy

1. **Unit tests**: Verify variant chain map building and lookup
2. **Integration tests**: Test with `extras_plugin::TestVariantChainEnum` type (which generates `TestVariantChainEnum.json`)
3. **Verification**:
   - Path `.middle_struct.nested_enum.name` has root with `BottomEnum::VariantB`
   - Path `.middle_struct.nested_enum.0` has root with `BottomEnum::VariantA`
   - Path `.middle_struct.nested_enum` shows all three variants in examples array
4. **Deep nesting**: Validate with 3+ level enum chains

## Example: Complete Transformation

### Before (Current System)
- Enum returns one path per signature group
- Parent gets limited examples
- Root example has wrong variant
- 2-step correction process needed

### After (With This Plan)
- Enum returns all paths during recursion
- Parent gets all variant examples
- Variant chain map provides correct root
- Single mutation operation works

## Migration Path

1. **Add variant chain map**: Build map during recursion without breaking current flow
2. **Parallel operation**: Populate both old and new fields
3. **Verify correctness**: Compare variant chains produce correct roots
4. **Switch output**: Use variant_chain_root_example instead of enum_variant_path
5. **Remove old system**: Eventually deprecate multi-step array

## Applicable Variants Tracking

The `applicable_variants` field serves a critical purpose:

### What It Contains
- ALL variants that share the EXACT same signature at a given mutation path
- Same field names AND same types (not just same types)
- Example: If `VariantB` and `VariantD` both have fields `name: String, value: f32`, both appear
- Counter-example: `Color::Srgba` (fields: red, green, blue, alpha) and `Color::LinearRgba` (fields: r, g, b, a) would NOT group together

### When It's Populated
- During enum processing when we group variants by signature
- Signature comparison MUST include field names to avoid incorrect grouping
- Stored in `EnumPathData` during recursion
- Preserved through deduplication to final output

### Why It Matters
- Tells AI agents which variants support a mutation path
- Enables proper variant selection without trial and error
- Documents the complete API surface for each path
- Prevents field name confusion bugs

## Important Implementation Details

### HashMap Collision Prevention
When processing enum variants, we must maintain signature grouping to prevent HashMap key collisions. For example:
- `VariantA(i32)` creates `MutationPathDescriptor("0")`
- `VariantB(i32)` also creates `MutationPathDescriptor("0")`
- Without grouping, these would collide in the HashMap

Solution: Group by signature first, then process all variants within each group individually.

### Structured Data Types
Consider using structured types instead of raw JSON during processing:

```rust
#[derive(Debug, Clone)]
struct VariantExampleData {
    variant_name: VariantName,
    signature: VariantSignature,
    example: Value,
}
```

This provides type safety and clearer code than working with JSON values directly.

## Notes

- This leverages existing variant chain tracking (`enum_variant_path` contains the chain)
- The core insight: variant chain should be the lookup key for root examples
- Signatures must match EXACTLY including field names to group variants
- We've confirmed the signature deduplication already includes field names (safe from Color bug)
- Performance impact: More paths during recursion, but same output size
- This is a surgical fix to the specific subset of paths that traverse enum variants