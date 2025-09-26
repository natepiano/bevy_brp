# Plan: Fix Root Examples Using Extended Descriptors for Variant Preservation

**Migration Strategy: Incremental**

## Executive Summary

Fix root example assembly for mutation paths within enum chains by extending `MutationPathDescriptor` to include an optional variant signature. This allows the existing HashMap structure to preserve ALL variant examples during recursion without breaking other builders. The paths already exist correctly - the problem is that enum fields can only store one example in the HashMap, losing variant diversity.

## High-Level Implementation Plan

• **Phase 1: Extend MutationPathDescriptor with optional variant signature**
  - Add `variant_signature: Option<VariantSignature>` field to descriptor
  - Implement `From<String>` and `From<&str>` for backwards compatibility
  - Non-enum builders continue creating simple descriptors unchanged
  - Enum builders create descriptors with both base and variant signature

• **Phase 2: Update enum processing to create multiple HashMap entries**
  - For each variant signature, create a descriptor with that signature
  - Insert each variant's example with its unique descriptor
  - Example: "nested_enum" + VariantA → `{"VariantA": 123}`
  - Example: "nested_enum" + VariantB → `{"VariantB": {"name": "Hello"}}`
  - This preserves ALL variant examples in the existing HashMap structure

• **Phase 3: Update parent enum's variant path updates**
  - When enum gets child paths back, it updates ALL variant examples
  - Each path's variant chain determines which descriptor to use
  - Parent enum wraps each variant's example correctly
  - Result: Complete root examples for all variant combinations

• **Phase 4: Add root-level deduplication**
  - At the root, group descriptors by base field name
  - Select one representative example per field (prefer non-unit variants)
  - Store the variant chain → root example mapping
  - Update all paths with their correct root examples

• **Phase 5: Clean up output**
  - Use the stored root examples for each path
  - Add `applicable_variants` field showing variant support
  - Single-step root example replaces multi-step array

## Design Considerations

### Memory Implications

Processing all variant paths without early deduplication could theoretically cause exponential memory growth. However, the existing recursion depth limit already protects against unbounded growth by capping the maximum nesting level.

### Phase Ordering

Data restructuring (Phase 1) is intentionally placed before logic changes (Phase 2) to avoid implementing new functionality on old data structures, reducing refactoring and technical debt.

### Performance and Regression Testing

The existing integration test suite is sufficient for performance and regression testing. The tests already validate complex nested enum scenarios and will catch any performance degradation or behavioral changes. See `.claude/commands/mutation_test.md` for the comprehensive validation framework.

### Backwards Compatibility

Backwards compatibility is not a concern for this implementation. This is a feature enhancement that fixes incorrect behavior in nested enum scenarios.

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

## The Core Problem: HashMap Can Only Hold One Example Per Field

**CRITICAL UNDERSTANDING**: The mutation paths already exist correctly for all signature groups. The problem is the HashMap structure `HashMap<MutationPathDescriptor, Value>` can only hold ONE value per key.

### Why Examples Get Lost

**The HashMap Bottleneck**
```
When MiddleStruct has field "nested_enum" of type BottomEnum:
→ BottomEnum has 3 variants with different signatures
→ But can only return ONE example for key "nested_enum"
→ Other variant examples are lost forever
→ Parent builds root with only one variant available
```

**Concrete Example**
```rust
// Current: HashMap can only hold one entry
HashMap {
  "nested_enum" → {"VariantA": 123}  // Only ONE variant preserved!
}

// Needed: Multiple entries for different variants
HashMap {
  "nested_enum" + VariantA → {"VariantA": 123},
  "nested_enum" + VariantB → {"VariantB": {"name": "Hello"}},
  "nested_enum" + VariantC → "VariantC"
}
```

### Example of the Problem
From `TestVariantChainEnum.json` in the enum_variant_path section:
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

## The Solution: Preserve All Variant Chain Examples

### What Needs to Change
1. **NOT path creation** - paths already exist for all signature groups ✓
2. **Example assembly** - preserve ALL variant chain examples during recursion
3. **Root mapping** - build `VariantChain → RootExample` for ALL combinations
4. **Path lookup** - each existing path uses its variant chain to find correct root

### Visual Flow Diagram

```
Current (BROKEN) Example Flow:
================================
BottomEnum [VariantA, VariantB, VariantC]
    ↓ (returns to parent)
HashMap {
  "nested_enum" → {"VariantA": 123}  // Only ONE variant!
}
    ↓ (parent receives)
MiddleStruct builds example with ONLY VariantA
    ↓ (returns to grandparent)
TestVariantChainEnum wraps example that only has VariantA
    ↓ (result)
Path ".middle_struct.nested_enum.name" needs VariantB
→ But root example has VariantA (WRONG!)


Fixed Example Flow with Extended Descriptors:
================================
BottomEnum [VariantA, VariantB, VariantC]
    ↓ (returns to parent with EXTENDED descriptors)
HashMap {
  {"nested_enum", Tuple} → {"VariantA": 123},
  {"nested_enum", Struct} → {"VariantB": {"name": "Hello"}},
  {"nested_enum", Unit} → "VariantC"
}
    ↓ (parent receives ALL variants)
MiddleStruct can build examples with ANY variant
    ↓ (returns to grandparent)
TestVariantChainEnum builds complete examples:
  - With VariantA for paths needing it
  - With VariantB for paths needing it
  - With VariantC for paths needing it
    ↓ (result)
Path ".middle_struct.nested_enum.name" with chain [WithMiddleStruct, VariantB]
→ Gets root example with VariantB (CORRECT!)
```

## Implementation Strategy

### Phase 1: Restructure Data with EnumPathData

Create a dedicated struct to group all enum-related data before implementing new logic. This provides better ergonomics and cleaner separation of concerns. Move the data structure definition from later in the document to be implemented first.

### Phase 2: Update Enum Processing to Create Multiple HashMap Entries

**THE CRITICAL CHANGE**: Create multiple HashMap entries with extended descriptors:

```rust
// In enum_path_builder.rs
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(HashMap<MutationPathDescriptor, Value>, Vec<MutationPathInternal>)> {
    let mut child_examples = HashMap::new();
    let mut all_child_paths = Vec::new();

    for (signature, variants_in_group) in variant_groups {
        // Create paths for this signature
        let paths = create_paths_for_signature(signature, ctx);

        for path in paths {
            // Process with first variant for path generation
            let representative = variants_in_group.first().unwrap();
            let mut child_ctx = ctx.create_recursion_context(path.clone(), PathAction::Create);
            child_ctx.variant_chain.push(VariantPath {
                full_mutation_path: ctx.full_mutation_path.clone(),
                variant: representative.variant_name().clone(),
                // ...
            });

            // Recurse to get paths and example
            let child_type_kind = get_type_kind(&child_ctx)?;
            let child_paths = recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

            // Get the example from first path
            let child_example = child_paths.first()
                .map_or(json!(null), |p| p.example.clone());

            // KEY CHANGE: Create descriptor WITH variant signature
            let descriptor = MutationPathDescriptor::with_variant(
                path.to_mutation_path_descriptor().base,
                signature.clone()
            );

            // This preserves THIS variant's example specifically
            child_examples.insert(descriptor, child_example);
            all_child_paths.extend(child_paths);
        }
    }

    Ok((child_examples, all_child_paths))
}
```

**What This Achieves**:
- Each variant signature gets its own HashMap entry
- Field "nested_enum" now has multiple entries (one per variant)
- All variant examples are preserved through recursion
- Other builders don't need to change at all

### Phase 3: Update Parent Enum Example Assembly

When parent enums assemble their examples, they need to handle child enum fields:

```rust
// In enum_path_builder.rs build_variant_example()
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &str,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
    current_variant_chain: &[VariantName],  // NEW: Track current chain
) -> Value {
    match signature {
        VariantSignature::Struct(fields) => {
            let mut obj = serde_json::Map::new();

            for field in fields {
                // NEW: Find the right variant's example for enum fields
                let example = find_field_example(
                    children,
                    &field.field_name,
                    current_variant_chain
                );
                obj.insert(field.field_name.to_string(), example);
            }

            json!({variant_name: Value::Object(obj)})
        }
        // Similar for Tuple, Unit...
    }
}

// Helper to find the right example for a field
fn find_field_example(
    children: &HashMap<MutationPathDescriptor, Value>,
    field_name: &str,
    variant_chain: &[VariantName],
) -> Value {
    // Try to find entry with matching variant chain
    // This is where we'd match based on the variant signature
    for (descriptor, value) in children {
        if descriptor.matches_field(field_name) {
            // Could check variant_chain compatibility here
            return value.clone();
        }
    }
    json!(null)
}
```

### Phase 4: Root-Level Deduplication and Example Selection

At the root, after all recursion completes, deduplicate and select representatives:

```rust
fn deduplicate_enum_examples(
    child_examples: HashMap<MutationPathDescriptor, Value>,
) -> HashMap<MutationPathDescriptor, Value> {
    let mut deduplicated = HashMap::new();

    // Group by base field name
    let mut field_groups: HashMap<String, Vec<(MutationPathDescriptor, Value)>> = HashMap::new();

    for (descriptor, value) in child_examples {
        field_groups
            .entry(descriptor.base.clone())
            .or_insert_with(Vec::new)
            .push((descriptor, value));
    }

    // For each field, pick the "coolest" variant
    for (field_name, variants) in field_groups {
        let selected = select_preferred_variant(variants);
        deduplicated.insert(selected.0, selected.1);
    }

    deduplicated
}

fn select_preferred_variant(
    variants: Vec<(MutationPathDescriptor, Value)>,
) -> (MutationPathDescriptor, Value) {
    // Prefer: Struct > Tuple > Unit
    // Or: Non-unit > Unit
    // Or: First non-null
    variants.into_iter()
        .find(|(_, v)| !is_unit_variant(v))
        .or_else(|| variants.first().cloned())
        .unwrap_or_else(|| (MutationPathDescriptor::from("error"), json!(null)))
}
```

**Critical**: Before deduplication, store the variant chain → example mapping so we can update paths with correct roots.

### Phase 5: Update Paths with Correct Root Examples

After building all examples, update paths with their correct root examples:

```rust
fn update_paths_with_root_examples(
    paths: &mut Vec<MutationPathInternal>,
    root_examples: &HashMap<Vec<VariantName>, Value>,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_data {
            if !enum_data.variant_chain.is_empty() {
                // Look up the correct root example for this variant chain
                enum_data.variant_chain_root_example =
                    root_examples.get(&enum_data.variant_chain).cloned();
            }
        }
    }
}

// Build the root examples during enum processing
fn build_root_examples_for_chains(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    child_examples: &HashMap<MutationPathDescriptor, Value>,
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, Value> {
    let mut root_examples = HashMap::new();

    // For each variant signature that has examples
    for (signature, variants) in variant_groups {
        for variant in variants {
            let mut chain = ctx.variant_chain.clone();
            chain.push(variant.variant_name().clone());

            // Build complete example for this chain
            let example = build_variant_example(
                signature,
                variant.name(),
                child_examples,
                ctx.type_name(),
                &chain
            );

            root_examples.insert(chain, example);
        }
    }

    root_examples
}
```

## Key Changes from Current System

1. **Extended Descriptor**: Add optional `variant_signature` field to `MutationPathDescriptor`
2. **Multiple HashMap Entries**: Enum fields create multiple entries (one per variant signature)
3. **Backwards Compatible**: Other builders continue using simple descriptors unchanged
4. **Preserve All Examples**: All variant examples survive through recursion
5. **Root Deduplication**: Select representative examples at root, but keep variant chain mapping
6. **Correct Root Examples**: Each path gets its specific variant chain's root example

## Data Structure Changes

### Extend MutationPathDescriptor to Preserve Variant Information

**THE KEY CHANGE**: Extend the descriptor to optionally include variant signature:

```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MutationPathDescriptor {
    /// The base descriptor (field name, index, etc.) - what it is today
    base: String,

    /// Optional variant signature - ONLY populated for enum fields
    /// This allows multiple HashMap entries for the same field
    variant_signature: Option<VariantSignature>,
}

// Backwards compatibility - other builders use this unchanged
impl From<String> for MutationPathDescriptor {
    fn from(s: String) -> Self {
        Self {
            base: s,
            variant_signature: None
        }
    }
}

impl From<&str> for MutationPathDescriptor {
    fn from(s: &str) -> Self {
        Self {
            base: s.to_string(),
            variant_signature: None
        }
    }
}

// For enum fields specifically
impl MutationPathDescriptor {
    pub fn with_variant(base: String, variant: VariantSignature) -> Self {
        Self {
            base,
            variant_signature: Some(variant),
        }
    }

    /// Get just the base for field lookups
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Check if this descriptor matches a base field name
    pub fn matches_field(&self, field_name: &str) -> bool {
        self.base == field_name
    }
}
```

### Keep EnumPathData for Organization (Optional)

```rust
/// Still useful for organizing enum-specific data on paths
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub variant_chain_root_example: Option<Value>,
    pub enum_instructions: Option<String>,
}
```

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

- **Minimal Change**: Only `MutationPathDescriptor` and enum processing change
- **Other Builders Unchanged**: Struct, tuple, array builders continue working as-is
- **HashMap Preserved**: Still using `HashMap<MutationPathDescriptor, Value>`
- **All Examples Preserved**: Each variant gets its own HashMap entry
- **Correct Assembly**: Parent enums can build complete examples for all variant combinations
- **Clean Deduplication**: Root level picks best examples while preserving all variant chains

## Success Criteria

1. **Correct Variants**: `.middle_struct.nested_enum.name` has root example with `VariantB`, not `VariantA`
2. **Complete Chain Map**: All variant chain combinations have corresponding root examples
3. **No Early Loss**: All paths propagate up during recursion
4. **Smart Deduplication**: Output shows one path per signature with correct root and applicable variants
5. **Variant Transparency**: `applicable_variants` clearly shows which variants support each path

## Testing Strategy

1. **Unit tests**:
   - Verify extended descriptor creation and equality
   - Test `matches_field()` functionality
   - Verify HashMap can hold multiple entries per field

2. **Integration tests**: Test with `extras_plugin::TestVariantChainEnum`
   - Before fix: `.middle_struct.nested_enum.name` has wrong root (VariantA)
   - After fix: `.middle_struct.nested_enum.name` has correct root (VariantB)

3. **Verification Points**:
   - HashMap has multiple entries for enum fields
   - Each variant signature preserved separately
   - Root deduplication selects appropriate examples
   - All paths updated with correct variant chain roots

4. **Edge Cases**:
   - Multiple variants with same signature
   - Deep nesting (3+ enum levels)
   - Mix of enum and non-enum fields
   - Empty variants and unit variants

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

## Implementation Strategy

### Step 1: Extend MutationPathDescriptor
1. Add `variant_signature: Option<VariantSignature>` field
2. Implement `From` traits for backwards compatibility
3. Add `with_variant()` constructor for enum use
4. Add `matches_field()` helper for field lookups

### Step 2: Update Enum Processing
1. Modify `process_children()` to create extended descriptors
2. Each variant signature gets its own HashMap entry
3. No changes to path creation logic

### Step 3: Update Example Assembly
1. Modify `build_variant_example()` to find right variant's example
2. Add helper to match descriptors by base field name
3. Thread variant chain through for context

### Step 4: Root Processing
1. Add deduplication function to group by base field
2. Select preferred variant per field
3. Build variant chain → example mapping
4. Update all paths with correct root examples

### Step 5: Testing
1. Verify with `TestVariantChainEnum`
2. Check `.middle_struct.nested_enum.name` has VariantB root
3. Validate all variant chains have correct examples
4. Performance test with deep nesting

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

## Critical Implementation Details

### The HashMap Bottleneck Explained

The core issue is that `HashMap<MutationPathDescriptor, Value>` enforces uniqueness by key:

```rust
// When BottomEnum returns to MiddleStruct:
child_examples.insert("nested_enum", example_A);  // First insert
child_examples.insert("nested_enum", example_B);  // OVERWRITES example_A!
```

With extended descriptors:
```rust
// Each variant gets its own entry:
child_examples.insert({"nested_enum", SignatureA}, example_A);
child_examples.insert({"nested_enum", SignatureB}, example_B);
// Both preserved!
```

### Why Other Builders Don't Need Changes

Other builders use `From<String>` which creates simple descriptors:
```rust
// struct_builder.rs - unchanged!
let descriptor = MutationPathDescriptor::from("field_name");
// Automatically gets variant_signature: None
```

Only enum processing explicitly uses the new constructor:
```rust
// enum_path_builder.rs - the only change!
let descriptor = MutationPathDescriptor::with_variant(
    "field_name".to_string(),
    variant_signature
);
```

## Important Implementation Details

### How This Solves the HashMap Collision Problem

The extended descriptor naturally prevents collisions:
- `VariantA(i32)` creates `MutationPathDescriptor { base: "0", variant_signature: Some(Tuple([i32])) }`
- `VariantB(i32)` creates `MutationPathDescriptor { base: "0", variant_signature: Some(Tuple([i32])) }`
- If they have the same signature, we only need one example anyway
- If they have different signatures, they get different descriptors

The variant signature in the descriptor ensures uniqueness while preserving all information needed for correct example assembly.

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

- This is a surgical fix - only `MutationPathDescriptor` and enum processing change
- The extended descriptor preserves backward compatibility perfectly
- HashMap structure remains unchanged, just more entries for enum fields
- Parent types naturally build correct examples without special logic
- Root deduplication ensures clean output despite multiple internal examples
- Performance impact: Slightly more HashMap entries during recursion, same final output size

## Design Review Skip Notes

## DESIGN-3: Standalone Functions Should Be Methods on Owning Types - **Verdict**: REJECTED
- **Status**: REJECTED - Finding was incorrect
- **Location**: Phase 4: Update output format section
- **Issue**: Plan proposes standalone functions like `finalize_mutation_paths` and `prepare_output` that operate on data structures they don't own, violating encapsulation principles
- **Reasoning**: The finding incorrectly applies "functions should be methods" too broadly. The current standalone function design follows clean functional pipeline architecture and is superior for this domain. Converting to methods would violate Rust's "don't implement on foreign types" principle and make the code feel unnatural. Functions like `update_child_variant_paths` operate on slices and coordinate multiple types - this is orchestration logic, not natural object behavior. The functional pipeline design (Input → Processing → Transformation → Output) is architecturally appropriate and should be preserved.