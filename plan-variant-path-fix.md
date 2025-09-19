# Plan: Fix Variant Path Implementation Issues
fix for ./plan-variant-path.md issues that we see in production

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
   cargo build
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

### Step 1: Fix enum handling for IndexedElement and ArrayElement ✅ COMPLETED
**Objective**: Extend enum context handling to tuple and array elements
**Files**: mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
**Build**: `cargo build`
**Type**: SAFE - Additive change

### Step 2: Fix variant description path references ✅ COMPLETED
**Objective**: Update generate_variant_description to properly reference parent paths
**Files**: mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
**Build**: `cargo build`
**Type**: SAFE - Local function modification

### Step 3: Fix variant name duplication ✅ COMPLETED
**Objective**: Remove redundant variant_name() calls that cause duplication
**Files**: mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
**Build**: `cargo build`
**Type**: SAFE - Fixes formatting bug

### Step 4: Fix signature group variant redundancy ✅ COMPLETED
**Objective**: Only use one representative variant per signature group in variant_path
**Files**: mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
**Build**: `cargo build`
**Type**: SAFE - Reduces redundancy in variant chains

### Step 5: Remove EnumChild wrapper ✅ COMPLETED
**Objective**: Eliminate redundant EnumChild variant and its dependencies
**Files**: mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs
**Build**: `cargo build`
**Type**: ATOMIC GROUP - All EnumChild references must be updated together

### Final Validation ⏳ PENDING
**Objective**: Run complete test suite and verify all fixes work together
**Build**: `cargo test` and `cargo nextest run`
**Validation**: Check JSON output matches expected format

## Target Structure Reference
See TestEnumWithSerde_mutation_paths.json for the correct target structure we're trying to achieve.

## Implementation Details

### Issue 7: IndexedElement Enum Handling

#### Problem
When an IndexedElement (like `.0`) points to an enum type within a tuple variant, it's not generating proper enum examples.

#### Current Code
File: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` lines 380-385
```rust
if matches!(child_kind, TypeKind::Enum)
    && child_ctx.enum_context.is_none()
    && matches!(child_ctx.path_kind, PathKind::StructField { .. })
{
    child_ctx.enum_context = Some(super::recursion_context::EnumContext::Root);
}
```

#### Proposed Fix
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

### Issue 3: PathRequirement.description Path Reference

#### Problem
The description incorrectly refers to "the root" when it should reference the specific parent path.

#### Current Output
```json
"description": "To use this mutation path, the root must be set to Handle<Image>::Handle<Image>::Weak"
```

#### Expected Output
```json
"description": "To use this mutation path, .color_lut must be set to Handle<Image>::Weak"
```

#### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` line 475
Function: `generate_variant_description`

#### Proposed Fix
Update the existing `generate_variant_description` function at line 475 in builder.rs to properly reference the parent path from the variant_path entry.

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

### Issue 1: Variant Name Format

#### Problem
Variant names are being duplicated with the type name appearing twice.

#### Current Output
```json
"variant": "Handle<Image>::Handle<Image>::Weak"
```

#### Expected Output
```json
"variant": "Handle<Image>::Weak"
```

#### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
Function: `process_all_children()` method
Lines: 272 and 283 where `ctx.type_name().variant_name(&variant)` is called
Note: The `variant_name()` method exists and works correctly in `brp_type_name.rs`

#### Root Cause
The enum_builder.rs already provides fully qualified variant names (e.g., "Handle<Image>::Weak") in the `applicable_variants` field. The builder.rs then incorrectly calls `variant_name()` again on these already-formatted names, causing duplication.

#### Proposed Fix
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

### Issue 2: PathRequirement.example Structure

#### Problem
The PathRequirement.example is wrapped in an incorrect structure with `applicable_variants` and `value` fields.

#### Current Output
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

#### Expected Output
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

#### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
Lines: 395-404 and 451-460

#### Root Cause
`MutationExample::EnumChild` is wrapping the example with `applicable_variants` metadata. This metadata is already passed through the `MaybeVariants` trait during `collect_children`, so the wrapper is redundant and pollutes the JSON output.

#### Proposed Fix
Remove `MutationExample::EnumChild` entirely and use `MutationExample::Simple` instead:

1. **Remove the `EnumChild` variant** from `MutationExample` enum (lines ~34-37)
2. **Change `EnumContext::Child` case** (lines 395-404) to return `MutationExample::Simple(example)` instead of `EnumChild`
3. **Remove the `EnumChild` match arm** (lines 451-460) in the JSON conversion
4. **Remove `flatten_variant_chain` function** (lines 272-280) - verified it has only one caller at line 398 in the `EnumContext::Child` case which is being removed

The `applicable_variants` information flows through `MaybeVariants` for use in building both `ExampleGroup` objects and `variant_path` entries, so `EnumChild` is unnecessary.

### Issue 4: Missing Examples Array for Enum Paths

#### Problem
Enum paths like `.color_lut.0` should have an `examples` array showing all variants, but instead have a single malformed `example`.

#### Current Output
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

#### Expected Output
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

#### Root Cause
This is a consequence of Issue 2. The wrapped example from `MutationExample::EnumChild` prevents proper enum handling. Once Issue 2 is fixed, this should resolve automatically.

#### Proposed Fix
After fixing Issue 2, verify that IndexedElement paths pointing to enums get `EnumContext::Root` set correctly (already happens at lines 380-385 for StructField).

### Issue 5: Example Wrapper in Mutation Path

#### Problem
Same as Issue 2 - the mutation path's example is being wrapped with `applicable_variants` metadata.

#### Root Cause
This is the same issue as Issue 2. `MutationExample::EnumChild` creates the wrapper.

#### Proposed Fix
Fixed by Issue 2's solution - removing `MutationExample::EnumChild`.

### Issue 6: Redundant Signature Group Variants in variant_path

#### Problem
When multiple enum variants share the same signature (e.g., `Special` and `AlsoSpecial` both containing a String), ALL variants are included in the `variant_path` array and description, creating redundancy.

#### Current Output
```json
"path_requirement": {
  "description": "To use this mutation path, root must be set to TestEnumWithSerDe::TestEnumWithSerDe::Special and root must be set to TestEnumWithSerDe::TestEnumWithSerDe::AlsoSpecial",
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::TestEnumWithSerDe::Special"},
    {"path": "", "variant": "TestEnumWithSerDe::TestEnumWithSerDe::AlsoSpecial"}
  ]
}
```

#### Expected Output
```json
"path_requirement": {
  "description": "To use this mutation path, root must be set to TestEnumWithSerDe::TestEnumWithSerDe::Special",
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::TestEnumWithSerDe::Special"}
  ]
}
```

#### Current Code
Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` lines 268-284
The code iterates through ALL variants from `MaybeVariants::applicable_variants()`:
```rust
for variant in variants {
    extended.push(super::types::VariantPathEntry {
        path:    ctx.mutation_path.clone(),
        variant: ctx.type_name().variant_name(&variant),
    });
}
```

#### Root Cause
When `enum_builder.rs` groups variants by signature, it passes ALL variants in the group via `applicable_variants()`. The builder.rs then adds ALL of them to the variant_chain, but for signature groups, only one representative variant is needed since they all have identical mutation paths.

#### Proposed Fix
Only use the first (representative) variant from each signature group when building the variant_chain:

```rust
// builder.rs lines 268-274 - change from iterating all variants:
for variant in variants {
    extended.push(super::types::VariantPathEntry {
        path:    ctx.mutation_path.clone(),
        variant: ctx.type_name().variant_name(&variant),
    });
}

// to using only the first (representative) variant:
if let Some(representative_variant) = variants.first() {
    extended.push(super::types::VariantPathEntry {
        path:    ctx.mutation_path.clone(),
        variant: ctx.type_name().variant_name(representative_variant),
    });
}

// Similarly for lines 279-284 - change from:
variants
    .iter()
    .map(|variant| super::types::VariantPathEntry {
        path:    ctx.mutation_path.clone(),
        variant: ctx.type_name().variant_name(variant),
    })
    .collect()

// to:
variants
    .first()
    .map(|variant| super::types::VariantPathEntry {
        path:    ctx.mutation_path.clone(),
        variant: ctx.type_name().variant_name(variant),
    })
    .into_iter()
    .collect()
```

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Design Review Skip Notes

### ⚠️ PREJUDICE WARNING - TYPE-SYSTEM-1: String-based variant handling instead of type-driven design
- **Status**: PERMANENTLY REJECTED
- **Location**: Section: Issue 1: Variant Name Format
- **Issue**: The plan fixes variant name duplication by removing redundant calls, but doesn't address the root cause: dual formatting paths that handle variant names inconsistently. The enum_builder.rs already formats variants with variant_name(), then builder.rs calls variant_name() again, indicating a design failure where string manipulation is scattered across multiple locations instead of using a single, type-driven approach.
- **Reasoning**: The plan already has the correct simple fix - builder.rs should clone already-formatted variants from enum_builder.rs. The suggested VariantFormatter type would be over-engineering. The plan explicitly states this approach in lines 32-44.
- **Critical Note**: DO NOT SUGGEST THIS AGAIN - Permanently rejected by user

### TYPE-SYSTEM-2: Magic string conditions instead of enum-driven pattern matching - **Verdict**: REJECTED ✅
- **Status**: SKIPPED
- **Location**: Section: Issue 7: IndexedElement Enum Handling
- **Issue**: The plan adds more string-based PathKind matching instead of using proper enum pattern matching. The condition `matches!(child_ctx.path_kind, PathKind::StructField { .. })` is extended to include more variants, but this approach perpetuates conditional logic instead of using type-driven design.
- **Reasoning**: The finding is incorrect because the code already uses proper Rust enum pattern matching with the `matches!` macro, which is idiomatic Rust, not "magic string conditions." Creating an EnumContextResolver for this simple three-condition check would be over-engineering.
- **Decision**: User elected to skip this recommendation

### ⚠️ PREJUDICE WARNING - DESIGN-1: Incomplete architectural solution for variant handling inconsistency
- **Status**: PERMANENTLY REJECTED
- **Location**: Section: Issue 1: Variant Name Format
- **Issue**: The plan addresses the symptom (duplicate formatting) but doesn't solve the architectural problem: enum_builder.rs and builder.rs both handle variant formatting independently. This creates tight coupling between these components and makes the system fragile to future changes.
- **Reasoning**: This is the same issue as TYPE-SYSTEM-1 which was already rejected with prejudice. The plan already has the correct approach (lines 32-44) where enum_builder.rs formats variants and builder.rs just clones them.
- **Critical Note**: DO NOT SUGGEST THIS AGAIN - Permanently rejected by user

### DESIGN-2: Inconsistent enum handling across different path types - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Issue 7: IndexedElement Enum Handling
- **Issue**: The plan reveals that enum handling is inconsistent across PathKind variants (StructField gets special treatment, but IndexedElement and ArrayElement don't). This suggests a design gap where enum context logic is scattered rather than centralized.
- **Existing Implementation**: The plan already contains this exact fix in Issue 7 "Proposed Fix" section (lines 246-258) with identical code to extend enum context handling to IndexedElement and ArrayElement paths
- **Plan Section**: Section: Issue 7: IndexedElement Enum Handling - Proposed Fix
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### IMPLEMENTATION-GAP-2: Missing analysis of EnumChild metadata dependencies - **Verdict**: REJECTED ✅
- **Status**: SKIPPED
- **Location**: Section: Issue 2: PathRequirement.example Structure
- **Issue**: The plan proposes removing MutationExample::EnumChild entirely but doesn't analyze what other code might depend on the applicable_variants metadata it provides. The plan states the metadata 'flows through MaybeVariants' but doesn't verify that all consumers of this metadata will continue to work after the removal.
- **Reasoning**: The plan correctly identifies that applicable_variants metadata flows through MaybeVariants trait (lines 87, 97). The metadata is preserved through the trait system, not the EnumChild wrapper, so the removal is safe.
- **Decision**: User elected to skip this recommendation

### ⚠️ PREJUDICE WARNING - SIMPLIFICATION-1: Complex dual-path architecture for variant handling could be simplified
- **Status**: PERMANENTLY REJECTED
- **Location**: Section: Issue 1: Variant Name Format
- **Issue**: The current architecture has two separate components (enum_builder.rs and builder.rs) both handling variant formatting in different ways. This creates unnecessary complexity and the duplication issues the plan is trying to fix. A simpler approach would centralize all variant handling in one location.
- **Reasoning**: This is the same issue as TYPE-SYSTEM-1 and DESIGN-1 already rejected with prejudice. Variant formatting is already centralized in BrpTypeName::variant_name(). The suggested VariantFormatter would add complexity, not reduce it.
- **Critical Note**: DO NOT SUGGEST THIS AGAIN - Permanently rejected by user