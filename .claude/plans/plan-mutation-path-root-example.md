# Plan: Fix Root Examples Using Separate Enum Descriptors for Variant Preservation

**Migration Strategy: Phased**

## Executive Summary

Fix root example assembly for mutation paths within enum chains by creating a separate `EnumFieldDescriptor` type that includes variant signature information. This allows enum processing to preserve ALL variant examples during recursion without breaking other builders that rely on `Borrow<str>` trait. The paths already exist correctly - the problem is that enum fields can only store one example in the HashMap, losing variant diversity.

## Current State

The codebase has the `EnumPathData` struct with the following fields:
- `variant_chain`: Vec<VariantPath> - Chain of enum variants from root to this path
- `applicable_variants`: Vec<VariantName> - All variants that share the same signature
- `variant_chain_root_example`: Option<Value> - Will be populated by this plan
- `enum_instructions`: Option<String> - Instructions for variant selection

The `MutationPathInternal` struct has an `enum_data: Option<EnumPathData>` field that consolidates all enum-related data.

## High-Level Implementation Plan

• **Phase 1: Create EnumFieldDescriptor type for variant tracking**
  - Add new `EnumFieldDescriptor` struct with `field_name` and `variant_signature` fields
  - Keep `MutationPathDescriptor` unchanged to preserve `Borrow<str>` for map/set/list builders
  - Enum processing uses `HashMap<EnumFieldDescriptor, Value>` internally
  - Other builders continue using `HashMap<MutationPathDescriptor, Value>` unchanged

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

### Phase 1: Add EnumFieldDescriptor Type

**Location**: `path_kind.rs`

Add the new `EnumFieldDescriptor` type as defined in "Section: Data Structure Changes". This is a simple type addition with no changes to existing code.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumFieldDescriptor {
    field_name: String,
    variant_signature: VariantSignature,
}

impl EnumFieldDescriptor {
    pub fn new(field_name: String, variant_signature: VariantSignature) -> Self {
        Self { field_name, variant_signature }
    }
}
```

**Key Point**: `MutationPathDescriptor` remains completely unchanged. No impact on existing builders.

### Phase 2: Update Enum Processing to Create Multiple HashMap Entries

**Location**: `enum_path_builder.rs`

**THE CRITICAL CHANGE**: Update enum processing to use `EnumFieldDescriptor` internally:

```rust
// In enum_path_builder.rs
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(HashMap<EnumFieldDescriptor, Value>, Vec<MutationPathInternal>)> {
    //                   ^^^^^^^^^^^^^^^^^^^^ Changed from MutationPathDescriptor
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

            // KEY CHANGE: Create EnumFieldDescriptor WITH variant signature
            let field_name = path.to_mutation_path_descriptor().to_string();
            let descriptor = EnumFieldDescriptor::new(field_name, signature.clone());

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
- Field "nested_enum" now has multiple entries (one per variant signature)
- All variant examples are preserved through recursion
- Other builders don't need to change at all (they never see `EnumFieldDescriptor`)

### Phase 3: Update Parent Enum Example Assembly

**Location**: `enum_path_builder.rs` - `build_variant_example()` and `build_enum_examples()`

Update these functions to use `HashMap<EnumFieldDescriptor, Value>`:

```rust
// In enum_path_builder.rs build_variant_example()
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &str,
    children: &HashMap<EnumFieldDescriptor, Value>,  // Changed type
    enum_type: &BrpTypeName,
) -> Value {
    match signature {
        VariantSignature::Struct(fields) => {
            let mut obj = serde_json::Map::new();

            for (field_name, field_type) in fields {
                // Create descriptor to lookup this field with current signature
                let descriptor = EnumFieldDescriptor::new(
                    field_name.to_string(),
                    signature.clone()
                );
                let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                obj.insert(field_name.to_string(), value);
            }

            json!({variant_name: Value::Object(obj)})
        }
        VariantSignature::Tuple(types) => {
            let mut tuple_values = Vec::new();
            for (index, _type_name) in types.iter().enumerate() {
                let descriptor = EnumFieldDescriptor::new(
                    index.to_string(),
                    signature.clone()
                );
                let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
                tuple_values.push(value);
            }
            // Handle single-element tuples
            if tuple_values.len() == 1 {
                json!({ variant_name: tuple_values[0] })
            } else {
                json!({ variant_name: tuple_values })
            }
        }
        VariantSignature::Unit => json!(variant_name),
    }
}
```

**Key Point**: Lookups use `EnumFieldDescriptor` with the full signature, ensuring we get the correct variant's example.

### Phase 4: Root-Level Deduplication and Example Selection

**Location**: `enum_path_builder.rs`

At the root, after all recursion completes, deduplicate and select representatives:

```rust
fn deduplicate_enum_examples(
    child_examples: HashMap<EnumFieldDescriptor, Value>,
) -> HashMap<EnumFieldDescriptor, Value> {
    let mut deduplicated = HashMap::new();

    // Group by field name (ignoring variant signature for grouping)
    let mut field_groups: HashMap<String, Vec<(EnumFieldDescriptor, Value)>> = HashMap::new();

    for (descriptor, value) in child_examples {
        field_groups
            .entry(descriptor.field_name().to_string())
            .or_insert_with(Vec::new)
            .push((descriptor, value));
    }

    // For each field, pick the most informative variant signature
    for (_field_name, entries) in field_groups {
        if let Some(selected) = select_preferred_variant_signature(entries) {
            deduplicated.insert(selected.0, selected.1);
        }
        // If None returned, indicates a bug in grouping logic - skip this field
    }

    deduplicated
}

fn select_preferred_variant_signature(
    entries: Vec<(EnumFieldDescriptor, Value)>,
) -> Option<(EnumFieldDescriptor, Value)> {
    // Prefer: Struct > Tuple > Unit (most informative first)
    // This selects which variant signature to use as the representative example
    entries.iter()
        .find(|(_desc, v)| !is_unit_variant(v))
        .cloned()
        .or_else(|| entries.into_iter().next())
    // Note: entries is never empty due to construction (see grouping logic above)
    // The Option return type exists for API consistency, but None is impossible
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
    child_examples: &HashMap<EnumFieldDescriptor, Value>,
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

// Integration: How these functions connect in the main flow
pub fn process_enum(
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    let variant_groups = extract_and_group_variants(ctx)?;
    let (child_examples, child_paths) = process_children(&variant_groups, ctx, depth)?;

    // Build examples and variant chain mapping
    let (enum_examples, default_example) =
        build_enum_examples(&variant_groups, child_examples.clone(), ctx)?;

    // NEW: Build the variant chain → root example mapping BEFORE deduplication
    let variant_chain_mapping = build_root_examples_for_chains(
        &variant_groups,
        &child_examples,  // Use BEFORE deduplication to preserve all variants
        ctx,
    );

    // Deduplicate for parent consumption
    let deduplicated_examples = deduplicate_enum_examples(child_examples);

    // Create result paths
    let mut result = create_result_paths(
        ctx,
        enum_examples,
        default_example,
        child_paths,
    );

    // NEW: Update paths with correct root examples using the mapping
    update_paths_with_root_examples(&mut result, &variant_chain_mapping);

    Ok(result)
}
```

**Critical Data Flow**:
1. `child_examples` (before deduplication) is passed to `build_root_examples_for_chains()`
2. The returned `variant_chain_mapping` is stored as a local variable
3. `variant_chain_mapping` is then passed to `update_paths_with_root_examples()`
4. This ensures all paths get their correct variant-specific root examples

## Key Changes from Current System

1. **Extended Descriptor**: Add optional `variant_signature` field to `MutationPathDescriptor`
2. **Multiple HashMap Entries**: Enum fields create multiple entries (one per variant signature)
3. **Backwards Compatible**: Other builders continue using simple descriptors unchanged
4. **Preserve All Examples**: All variant examples survive through recursion
5. **Root Deduplication**: Select representative examples at root, but keep variant chain mapping
6. **Correct Root Examples**: Each path gets its specific variant chain's root example

## Data Structure Changes

### Create Separate EnumFieldDescriptor for Variant Tracking

**THE KEY CHANGE**: Create a new descriptor type specifically for enum field tracking, keeping `MutationPathDescriptor` unchanged:

**Rationale**: Extending `MutationPathDescriptor` with a second field would break the `Borrow<str>` trait implementation used by map/set/list builders (map_builder.rs:91, set_builder.rs:61, list_builder.rs:66) for efficient HashMap lookups using `SchemaField::Key.as_ref()` and similar string slice patterns. A separate type maintains backwards compatibility while enabling enum-specific variant tracking.

```rust
// In path_kind.rs - Keep MutationPathDescriptor unchanged
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MutationPathDescriptor(String);  // Unchanged - preserves Borrow<str>

impl Deref for MutationPathDescriptor {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for MutationPathDescriptor {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// NEW: Separate type for enum-specific field tracking with variant signature
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumFieldDescriptor {
    field_name: String,
    variant_signature: VariantSignature,
}

impl EnumFieldDescriptor {
    pub fn new(field_name: String, variant_signature: VariantSignature) -> Self {
        Self { field_name, variant_signature }
    }

    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    pub fn variant_signature(&self) -> &VariantSignature {
        &self.variant_signature
    }
}

// Note: EnumFieldDescriptor does NOT implement Borrow<str> - it doesn't need to!
// Enum processing never uses string slice lookups (see build_variant_example lines 344, 360)
```

**Scope Impact**: This change is isolated to only 2 files:
- `path_kind.rs`: Add `EnumFieldDescriptor` type (~15 lines)
- `enum_path_builder.rs`: Use `HashMap<EnumFieldDescriptor, Value>` internally (~15 lines changed across 3 functions)

**Why this works**:
- Enum processing (enum_path_builder.rs) never uses string slice lookups - always uses `children.get(&descriptor)`
- Map/set/list builders use string slices (`SchemaField::Key.as_ref()`) but never see enum descriptors
- Complete isolation: enum uses `HashMap<EnumFieldDescriptor, Value>`, others use `HashMap<MutationPathDescriptor, Value>`

**No changes needed to**:
- Other builders (struct, tuple, map, set, list) - continue using `MutationPathDescriptor`
- `builder.rs` central dispatch - interface unchanged
- `PathBuilder` trait - no modifications required

### EnumPathData Structure

The existing `EnumPathData` struct has the following structure:

```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantPath>,
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

### Step 1: Add EnumFieldDescriptor Type
**File**: `path_kind.rs`
1. Add `EnumFieldDescriptor` struct with `field_name` and `variant_signature` fields
2. Implement `new()` constructor and accessor methods
3. Derive `Debug, Clone, PartialEq, Eq, Hash` for HashMap compatibility
4. No changes to existing `MutationPathDescriptor` - preserves `Borrow<str>`

### Step 2: Update Enum Processing
**File**: `enum_path_builder.rs`
1. Modify `process_children()` to use `HashMap<EnumFieldDescriptor, Value>`
2. Create `EnumFieldDescriptor::new(field_name, signature)` for each variant
3. Each variant signature gets its own HashMap entry
4. No changes to path creation logic

### Step 3: Update Example Assembly
**File**: `enum_path_builder.rs`
1. Modify `build_variant_example()` to accept `HashMap<EnumFieldDescriptor, Value>`
2. Modify `build_enum_examples()` to accept `HashMap<EnumFieldDescriptor, Value>`
3. Update lookups to create `EnumFieldDescriptor` with appropriate signature
4. Remove obsolete helper functions

### Step 4: Root Processing
**File**: `enum_path_builder.rs`
1. Add deduplication function to group by field name (ignoring signature)
2. Select preferred variant signature per field
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

## TYPE-SYSTEM-3: EnumFieldDescriptor Type Lacks Display and ToString Implementations - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Phase 1: Add EnumFieldDescriptor Type
- **Issue**: The proposed `EnumFieldDescriptor` type lacks `Display` and `ToString` trait implementations, creating inconsistency with `MutationPathDescriptor`
- **Reasoning**: Investigation confirmed that `EnumFieldDescriptor` is used exclusively as an internal HashMap key within enum processing. No code in the plan formats, displays, or converts the entire descriptor to string. Component access uses accessor methods (`.field_name()`, `.variant_signature()`) not whole descriptor display. The derived `Debug` trait provides sufficient debugging capability. Adding `Display` would violate YAGNI principle.
- **Decision**: User elected to skip this recommendation

## DESIGN-4: Phase 2 Example Code References Non-Existent create_paths_for_signature Function - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Phase 2: Update Enum Processing to Create Multiple HashMap Entries
- **Issue**: Phase 2 implementation code calls `create_paths_for_signature(signature, ctx)` which doesn't exist yet in the plan's proposed changes, creating implementation uncertainty
- **Reasoning**: The finding mischaracterizes the plan's intent. Phase 2 code (line 227) explicitly calls `create_paths_for_signature`, showing the plan author IS AWARE of this function. Implementation plans focus on what changes, not documenting all unchanged code. The function isn't mentioned in the changes because it doesn't need modification. Adding an explicit "requires NO changes" note would add unnecessary verbosity without improving clarity. This follows standard documentation practice for architectural planning.
- **Decision**: User elected to skip this recommendation

## DESIGN-6: Phase 3 build_variant_example Signature Change Missing Return Type Documentation - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Phase 3: Update Parent Enum Example Assembly
- **Issue**: The signature change from `HashMap<MutationPathDescriptor, Value>` to `HashMap<EnumFieldDescriptor, Value>` lacks documentation about return type and integration points
- **Reasoning**: The function signature explicitly shows `-> Value` as the return type. The current implementation has the same minimal documentation style. The plan's "Key Point" section (line 319) already explains integration: "Lookups use `EnumFieldDescriptor` with the full signature, ensuring we get the correct variant's example." This is an internal function refactor within a comprehensive multi-phase plan that provides adequate context. The finding applies higher documentation standards than the existing codebase maintains.
- **Decision**: User elected to skip this recommendation

## DESIGN-7: Phase 4 Missing Error Handling for Empty Variant Groups - **Verdict**: REJECTED (with compilation fix applied)
- **Status**: SKIPPED - Error logging rejected, but compilation error fixed
- **Location**: Section: Phase 4: Root-Level Deduplication and Example Selection
- **Issue**: The `select_preferred_variant_signature` function silently drops fields when returning None, without error logging
- **Reasoning**: Empty variant groups are mathematically impossible by construction - the grouping logic ensures every field_group entry is non-empty. The real issue was a compilation error (line 361 used moved value `entries`). Fixed by using `.iter().find(...).cloned()` pattern followed by `.or_else(|| entries.into_iter().next())`. Adding error logging would be dead code that misleads maintainers. The comment was updated to clarify that None is impossible due to construction invariant.
- **Decision**: User elected to fix compilation error but skip error logging proposal

## IMPLEMENTATION-1: Missing Implementation: Variant Chain to Root Example Mapping Storage - **Verdict**: CONFIRMED
- **Status**: APPROVED - Integration point specification added
- **Location**: Section: Phase 5: Update Paths with Correct Root Examples
- **Issue**: Phase 4 describes building variant chain mapping and Phase 5 describes updating paths, but no specification of WHERE the mapping is stored or HOW it's passed between phases
- **Reasoning**: Valid implementation gap. The plan showed functions that build and use the mapping but didn't show the data flow through the system. Functions `build_root_examples_for_chains()` and `update_paths_with_root_examples()` exist in the plan but no integration point was specified.
- **Resolution**: Added complete `process_enum()` function specification showing:
  - Where `variant_chain_mapping` is created (from `build_root_examples_for_chains()`)
  - How it's stored (as local variable)
  - How it's passed to `update_paths_with_root_examples()`
  - Critical data flow documentation explaining the connection between phases
- **Decision**: User agreed to add integration point specification

## TYPE-SYSTEM-1: Standalone Functions Should Be Methods on Owning Types - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Phase 2: Update Enum Processing
- **Issue**: Multiple standalone functions operate on data structures they don't own, violating encapsulation principles. Functions like process_children, find_field_example, deduplicate_enum_examples, and others manipulate HashMap and variant data without clear ownership.
- **Reasoning**: The finding incorrectly applied OOP and functional programming principles. The current in-place mutation in `update_child_variant_paths` is more efficient and appropriate for an internal helper function. The codebase correctly uses functional architecture at module boundaries while allowing efficient in-place mutation within functions. The mentioned functions `find_field_example` and `deduplicate_enum_examples` don't even exist in the codebase.
- **Decision**: User elected to skip this recommendation

## TYPE-SYSTEM-2: Builder Pattern Opportunity for Complex MutationPathDescriptor Construction - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Data Structure Changes - MutationPathDescriptor Extension
- **Issue**: The proposed MutationPathDescriptor extension adds complexity with optional variant signatures but lacks a structured construction pattern. Multiple constructors (From<String>, From<&str>, with_variant) suggest need for a builder pattern.
- **Reasoning**: The finding misidentifies simple, appropriate construction patterns as complex scenarios requiring a builder pattern. A struct with only 2 fields and 2 distinct construction patterns doesn't need a builder. The plan's approach is correct: From traits for the common case (backwards compatibility) and one with_variant method for the specialized case. This is exactly the right level of abstraction without unnecessary complexity.
- **Decision**: User elected to skip this recommendation

## DESIGN-3: Standalone Functions Should Be Methods on Owning Types - **Verdict**: REJECTED
- **Status**: REJECTED - Finding was incorrect
- **Location**: Phase 4: Update output format section
- **Issue**: Plan proposes standalone functions like `finalize_mutation_paths` and `prepare_output` that operate on data structures they don't own, violating encapsulation principles
- **Reasoning**: The finding incorrectly applies "functions should be methods" too broadly. The current standalone function design follows clean functional pipeline architecture and is superior for this domain. Converting to methods would violate Rust's "don't implement on foreign types" principle and make the code feel unnatural. Functions like `update_child_variant_paths` operate on slices and coordinate multiple types - this is orchestration logic, not natural object behavior. The functional pipeline design (Input → Processing → Transformation → Output) is architecturally appropriate and should be preserved.
