# Implementation Plan: root_example_unavailable_reason

## Dependencies

**IMPORTANT:** This plan depends on `mutability-reason-type-safety.md` being completed first.

That plan refactors `MutationPathInternal.mutability_reason` from `Option<Value>` (JSON) to `Option<NotMutableReason>` (typed enum), which makes Phase 2.1 of this plan significantly simpler and type-safe.

**Implementation order:**
1. Complete `mutability-reason-type-safety.md` first
2. Then implement this plan

---

## Problem Statement

Enum variants that are PartiallyMutable or NotMutable cannot be constructed via BRP, but their mutable fields should still be documented for entities already in that variant. Currently, these paths show `root_example: "None"` (fallback to wrong variant) causing:

1. **Misleading documentation** - shows wrong variant structure
2. **Mutation test failures** - tries to mutate fields on wrong variant
3. **User confusion** - instructions don't match reality

### Root Cause

Lines 565-570 in `enum_path_builder.rs::build_partial_root_examples`:
```rust
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone())           // Returns None for PartiallyMutable
    .or_else(|| select_preferred_example(...))    // BUG: Falls back to wrong variant!
    .unwrap_or(json!(null));
```

For `TestMixedMutabilityEnum::Multiple` (PartiallyMutable due to Arc fields):
- Its `example` is `None` (line 332 - no spawn example generated)
- Falls back to `select_preferred_example()` which picks `None` (Unit variant - Mutable)
- Result: Paths like `.value` (only exist on Multiple) get `root_example: "None"`

### Example Issue

**Current output:**
```json
{
  "path": ".value",
  "applicable_variants": ["TestMixedMutabilityEnum::Multiple"],
  "root_example": "None",  // WRONG - Unit variant, not Multiple!
  "enum_instructions": "First, set root to 'root_example'..."
}
```

**Expected output:**
```json
{
  "path": ".value",
  "applicable_variants": ["TestMixedMutabilityEnum::Multiple"],
  "root_example": {
    "Multiple": {
      "name": "Hello, World!",
      "mixed": {"mutable_string": "test", "mutable_float": 1.0},
      "value": 1.0
    }
  },
  "root_example_unavailable_reason": "Cannot construct Multiple variant via BRP due to non-mutable fields: .mixed.not_mutable_arc (Arc<String>): Type bevy_platform::sync::Arc<alloc::string::String> is a leaf type registered in the schema but has no hardcoded example value available for mutations. This variant's mutable fields can only be mutated if the entity is already set to this variant by game code."
}
```

---

## Solution Overview

1. **Remove fallback logic** - Always build variant-specific root_example
2. **Add new field** - `root_example_unavailable_reason` explaining why variant can't be constructed
3. **Collect actual reasons** - Extract from NotMutable child fields (not assume "Arc")
4. **Filter mutation tests** - Skip unconstructible paths in prepare.py

---

## Phase 1: Core Type System Changes

### 1.1 Update `EnumPathData` struct
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:213-226`

```rust
#[derive(Debug, Clone)]
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<Value>,
    /// Explanation for why root_example cannot be used to construct this variant via BRP.
    /// Only populated for PartiallyMutable/NotMutable variants.
    pub root_example_unavailable_reason: Option<String>,  // NEW
}
```

### 1.2 Update `PathInfo` struct
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:172-197`

Add after `root_example` field (line 196):
```rust
/// Explanation for why root_example cannot be used to construct the required variant
#[serde(skip_serializing_if = "Option::is_none")]
pub root_example_unavailable_reason: Option<String>,  // NEW
```

**Architecture Note:** The field exists in both structs following the existing pattern for `root_example`:
- **EnumPathData** (1.1): Internal representation during type guide generation
- **PathInfo** (1.2): External API representation serialized to JSON

The field is populated in `EnumPathData` during processing (Phase 4), then mapped to `PathInfo` during serialization (Phase 5). This separation allows internal processing logic to remain strongly typed while the external API provides the JSON schema expected by MCP clients.

---

## Phase 2: Variant Constructibility Analysis

### 2.1 Create analysis function
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Add after `build_variant_example_for_chain` (around line 628):

```rust
/// Analyze if a variant can be constructed via BRP and build detailed reason if not
///
/// Returns `Ok(())` if variant is constructible (Mutable variants, Unit variants)
/// Returns `Err(reason)` if variant cannot be constructed, with human-readable explanation
///
/// For PartiallyMutable variants, collects actual reasons from NotMutable child fields.
/// For NotMutable variants, indicates all fields are problematic.
fn analyze_variant_constructibility(
    variant_name: &VariantName,
    signature: &VariantSignature,
    mutability: Mutability,
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Result<(), String> {
    // Unit variants are always constructible (no fields to serialize)
    if matches!(signature, VariantSignature::Unit) {
        return Ok(());
    }

    // Fully Mutable variants are constructible
    if matches!(mutability, Mutability::Mutable) {
        return Ok(());
    }

    // NotMutable variants - all fields are problematic
    if matches!(mutability, Mutability::NotMutable) {
        return Err(format!(
            "Cannot construct {} variant via BRP - all fields are non-mutable. \
            This variant cannot be mutated via BRP.",
            variant_name.short_name()
        ));
    }

    // PartiallyMutable variants - collect NotMutable field reasons
    // NOTE: This assumes mutability-reason-type-safety.md has been completed,
    // so mutability_reason is Option<NotMutableReason> (typed enum), not Option<Value> (JSON).
    let not_mutable_details: Vec<String> = child_paths
        .iter()
        .filter(|p| p.is_direct_child_at_depth(*ctx.depth))
        .filter(|p| matches!(p.mutability, Mutability::NotMutable))
        .map(|p| {
            let descriptor = p.path_kind.to_mutation_path_descriptor();
            let type_name = p.type_name.short_name();

            // Extract the actual reason from mutability_reason if available
            // With typed enum, we can simply format it directly
            let reason_detail = p.mutability_reason
                .as_ref()
                .map(|reason| format!("{reason}"))
                .unwrap_or_else(|| "unknown reason".to_string());

            format!("{descriptor} ({type_name}): {reason_detail}")
        })
        .collect();

    if not_mutable_details.is_empty() {
        // Shouldn't happen for PartiallyMutable, but handle gracefully
        return Ok(());
    }

    let field_list = not_mutable_details.join("; ");

    Err(format!(
        "Cannot construct {} variant via BRP due to non-mutable fields: {}. \
        This variant's mutable fields can only be mutated if the entity is \
        already set to this variant by game code.",
        variant_name.short_name(),
        field_list
    ))
}
```

**Rationale:**
- Collects actual NotMutable reasons from child paths
- Handles all mutability cases: Unit, Mutable, PartiallyMutable, NotMutable
- Provides detailed, actionable error messages
- No assumptions about Arc fields - uses actual mutability_reason
- **Depends on mutability-reason-type-safety.md** - uses typed `NotMutableReason` enum for type-safe extraction

---

## Phase 3: Remove Fallback and Build Reasons

### 3.0 Add PartialRootExample struct

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Add this struct definition near the top of the file, after the type aliases (around line 76-80):

```rust
/// Data for a partial root example including construction feasibility
///
/// Stores both the JSON example for a variant chain and an optional explanation
/// for why that variant cannot be constructed via BRP spawn/insert operations.
#[derive(Debug, Clone)]
struct PartialRootExample {
    /// Complete root example for this variant chain
    example: Value,
    /// Explanation for why this variant cannot be constructed via BRP.
    /// Only populated for PartiallyMutable/NotMutable variants.
    unavailable_reason: Option<String>,
}
```

**Rationale:** Grouping related data (example + its unavailability reason) in a struct is more idiomatic than two parallel HashMaps. This ensures keys cannot diverge, simplifies call sites (single hash lookup instead of two), and makes the code more maintainable.

### 3.1 Update `build_partial_root_examples` signature
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs:549-609`

Change return type (line 549):
```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, PartialRootExample>
```

### 3.2 Remove fallback and always build variant-specific examples
**File:** Same file

**Understanding the loop structure** (lines 558-606):
```rust
for (signature, variants) in variant_groups.sorted() {      // Line 558 - OUTER LOOP
    for variant_name in variants {                          // Line 559 - INNER LOOP
        let mut this_variant_chain = ctx.variant_chain.clone();
        this_variant_chain.push(variant_name.clone());

        // Lines 565-570: BUGGY FALLBACK (DELETE THIS)
        // Lines 575-604: Nested chain logic (REPLACE THIS)

    } // End inner loop
} // End outer loop
```

**DELETE lines 565-570** (the incorrect fallback - inside inner loop):
```rust
// REMOVE THIS:
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone())
    .or_else(|| select_preferred_example(enum_examples))
    .unwrap_or(json!(null));
```

**REPLACE lines 565-604** (all code after `this_variant_chain` creation) with:
```rust
// Find this variant's mutability status
let variant_mutability = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .map(|ex| ex.mutability)
    .unwrap_or(Mutability::NotMutable);

// Determine if this variant can be constructed via BRP
let unavailable_reason = analyze_variant_constructibility(
    variant_name,
    signature,
    variant_mutability,
    child_mutation_paths,
    ctx,
).err();

// Find all deeper nested chains that extend this variant
let nested_enum_chains =
    collect_child_chains_to_wrap(child_mutation_paths, &this_variant_chain, ctx);

// Build root examples for each nested enum chain
for nested_chain in &nested_enum_chains {
    let example = build_variant_example_for_chain(
        signature,
        variant_name,
        child_mutation_paths,
        nested_chain,
        ctx,
    );

    // Propagate reason to nested chains
    // RATIONALE: When a parent variant is unconstructible (e.g., TestMixedMutabilityEnum::Multiple
    // has Arc fields), ALL nested mutations on that variant's fields are also unconstructible
    // via BRP spawn operations. Even if a nested enum field itself is mutable, you cannot
    // reach it through spawn/insert because the parent variant cannot be constructed.
    // Game code could set the parent variant at runtime, but mutation tests that rely on
    // spawn/insert operations cannot test these nested paths. This propagation ensures
    // prepare.py correctly filters these paths from mutation testing.
    partial_root_examples.insert(
        nested_chain.clone(),
        PartialRootExample {
            example,
            unavailable_reason: unavailable_reason.clone(),
        },
    );
}

// Build root example for this variant's chain itself
let example = build_variant_example_for_chain(
    signature,
    variant_name,
    child_mutation_paths,
    &this_variant_chain,
    ctx,
);

partial_root_examples.insert(
    this_variant_chain,
    PartialRootExample {
        example,
        unavailable_reason,
    },
);
```

Initialize HashMap at start (line 555):
```rust
let mut partial_root_examples = HashMap::new();
```

Return at end (line 607):
```rust
partial_root_examples
```

**Rationale:**
- Removes incorrect fallback causing the bug
- Always builds variant-specific root_example
- Analyzes each variant's constructibility
- Stores both examples and reasons in single struct (type-safe, cannot diverge)
- Propagates reasons to nested chains
- Uses idiomatic Rust pattern (grouped data in struct, not parallel collections)

---

## Phase 4: Propagation Through Call Stack

### 4.1 Update `ProcessChildrenResult` type
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs:76-80`

```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED: struct instead of tuple
);
```

### 4.2 Update `process_signature_groups`
**File:** Same file, lines 400-460

Change line 456:
```rust
let partial_root_examples =
    build_partial_root_examples(variant_groups, &examples, &child_mutation_paths, ctx);
```

Change return (line 459):
```rust
Ok((examples, child_mutation_paths, partial_root_examples))
```

### 4.3 Update `process_enum`
**File:** Same file, lines 87-128

Change line 101:
```rust
let (enum_examples, child_mutation_paths, partial_root_examples) =
    process_signature_groups(&variants_grouped_by_signature, ctx)?;
```

Change line 121:
```rust
Ok(create_enum_mutation_paths(
    ctx,
    enum_examples,
    default_example,
    child_mutation_paths,
    partial_root_examples,
))
```

### 4.4 Update `create_enum_mutation_paths`
**File:** Same file, lines 724-766

Update parameter (line 724):
```rust
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED
) -> Vec<MutationPathInternal>
```

Update call (line 756):
```rust
propagate_partial_root_examples_to_children(
    &mut child_mutation_paths,
    &partial_root_examples,
    ctx,
);
```

### 4.5 Update `propagate_partial_root_examples_to_children`
**File:** Same file, lines 707-721

Update parameter:
```rust
fn propagate_partial_root_examples_to_children(
    child_paths: &mut [MutationPathInternal],
    partial_root_examples: &HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED
    ctx: &RecursionContext,
)
```

Update call (line 719):
```rust
support::populate_root_examples_from_partials(
    child_paths,
    partial_root_examples,
);
```

### 4.6 Update `support::populate_root_examples_from_partials`
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs:158-176`

```rust
pub fn populate_root_examples_from_partials(
    paths: &mut [MutationPathInternal],
    partials: &HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
        {
            // Populate both fields from the struct (single lookup!)
            if let Some(data) = partials.get(&enum_data.variant_chain) {
                enum_data.root_example = Some(data.example.clone());
                enum_data.root_example_unavailable_reason = data.unavailable_reason.clone();
            }
        }
    }
}
```

---

## Phase 5: JSON Serialization

### 5.1 Update `resolve_enum_data_mut`
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs:179-205`

Change return type (line 181):
```rust
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<Value>,            // root_example
    Option<String>,           // root_example_unavailable_reason (NEW)
)
```

Change early return (line 186):
```rust
return (None, None, None, None);
```

Change map_or (line 189-204):
```rust
self.enum_path_data
    .take()
    .map_or((None, None, None, None), |enum_data| {
        let instructions = Some(format!(
            "First, set the root mutation path to 'root_example', then you can mutate the '{}' path. See 'applicable_variants' for which variants support this field.",
            &self.mutation_path
        ));

        let variants = if enum_data.applicable_variants.is_empty() {
            None
        } else {
            Some(enum_data.applicable_variants)
        };

        (
            instructions,
            variants,
            enum_data.root_example,
            enum_data.root_example_unavailable_reason,  // NEW
        )
    })
```

### 5.2 Update `into_mutation_path_external`
**File:** Same file, lines 76-110

Update extraction (line 94):
```rust
let (enum_instructions, applicable_variants, root_example, root_example_unavailable_reason) =
    self.resolve_enum_data_mut();
```

Update struct creation (lines 96-109):
```rust
MutationPathExternal {
    description,
    path_info: PathInfo {
        path_kind: self.path_kind,
        type_name: self.type_name,
        type_kind,
        mutability: self.mutability,
        mutability_reason: self.mutability_reason,
        enum_instructions,
        applicable_variants,
        root_example,
        root_example_unavailable_reason,  // NEW
    },
    path_example,
}
```

---

## Phase 6: Initialization

### 6.1 Initialize new field in EnumPathData construction

**IMPORTANT:** There are TWO construction sites that must be updated.

#### Site 1: enum_path_builder.rs
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Find `build_enum_root_path` (around line 679-687):
```rust
let enum_path_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain:       ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        root_example:        None,
        root_example_unavailable_reason: None,  // NEW
    })
};
```

#### Site 2: path_builder.rs
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Find `build_mutation_path_internal` (around line 416-424):
```rust
let enum_path_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain:       ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        root_example:        None,
        root_example_unavailable_reason: None,  // NEW
    })
};
```

**Rationale:** Both functions construct `EnumPathData` for nested enum scenarios. Site 1 handles enum-within-enum cases, Site 2 handles all other types nested within enums (structs, tuples, etc.). Both must initialize the new field or compilation will fail after Phase 1.1.

---

## Phase 7: Mutation Test Integration

### 7.0 Update `PathInfo` TypedDict
**File:** `.claude/scripts/mutation_test/prepare.py:51-55`

Add new field to TypedDict:
```python
class PathInfo(TypedDict, total=False):
    """Path metadata including mutability and root examples."""

    mutability: str
    root_example: object
    root_example_unavailable_reason: str  # NEW
```

**Rationale:** TypedDict must include all fields that will be accessed in the filtering code. Without this, basedpyright will report type errors when accessing the new field at line 609/611.

### 7.1 Add path filtering logic
**File:** `.claude/scripts/mutation_test/prepare.py`

Add filtering after excluded types removal (after line 1022):

```python
# Filter out paths with unavailable root examples from mutation testing
print("Filtering paths with unavailable root examples...", file=sys.stderr)

for type_name, type_data in list(data["type_guide"].items()):
    mutation_paths: dict[str, MutationPathData] = type_data.get("mutation_paths", {})
    available_paths: dict[str, MutationPathData] = {}
    excluded_count: int = 0

    for path, path_data in mutation_paths.items():
        path_info: PathInfo = path_data.get("path_info", {})

        # Check if root_example is unavailable
        # This filters ANY path with unconstructibility marker, including nested variant
        # chains that inherit unconstructibility from their parent. This is conservative
        # but correct: we exclude nested mutations even if they would work once game code
        # sets the parent variant, because mutation tests rely on spawn/insert operations.
        if "root_example_unavailable_reason" in path_info:
            excluded_count += 1
            reason_preview = path_info["root_example_unavailable_reason"][:80]
            print(
                f"  Excluding {type_name}{path}: {reason_preview}...",
                file=sys.stderr
            )
        else:
            available_paths[path] = path_data

    # Update type's mutation paths
    if available_paths:
        type_data["mutation_paths"] = available_paths
        if excluded_count > 0:
            print(
                f"  Kept {len(available_paths)} paths, excluded {excluded_count} for {type_name}",
                file=sys.stderr
            )
    else:
        # No testable paths remain - remove entire type
        print(
            f"  Removing {type_name} - no constructible paths remain",
            file=sys.stderr
        )
        del data["type_guide"][type_name]
```

---

## Phase 8: Testing and Validation

### 8.1 Manual verification checklist

1. **Build and verify compilation:**
   ```bash
   cargo build
   ```

2. **Launch test app:**
   ```bash
   mcp__brp__brp_launch_bevy_example --target=extras_plugin --profile=debug
   ```

3. **Get type guide:**
   ```bash
   mcp__brp__brp_type_guide --types='["extras_plugin::TestMixedMutabilityEnum"]'
   ```

4. **Verify `.value` path shows:**
   ```json
   {
     "path": ".value",
     "applicable_variants": ["TestMixedMutabilityEnum::Multiple"],
     "root_example": {
       "Multiple": {
         "name": "Hello, World!",
         "mixed": {
           "mutable_string": "Hello, World!",
           "mutable_float": 1.0,
           "partially_mutable_nested": {"nested_mutable_value": 1.0}
         },
         "value": 1.0
       }
     },
     "root_example_unavailable_reason": "Cannot construct Multiple variant via BRP due to non-mutable fields: .mixed.not_mutable_arc (Arc<String>): Type bevy_platform::sync::Arc<alloc::string::String> is a leaf type registered in the schema but has no hardcoded example value available for mutations. This variant's mutable fields can only be mutated if the entity is already set to this variant by game code."
   }
   ```

5. **Verify WithMixed variant similarly** - paths like `.0.mutable_float` should have variant-specific root_example

### 8.2 Python type checking

Verify TypedDict changes pass type checking:
```bash
~/.local/bin/basedpyright .claude/scripts/mutation_test/prepare.py
```

Expected: Zero errors, zero warnings. If you see `reportAny` errors about `PathInfo` or `root_example_unavailable_reason`, the TypedDict update in Phase 7.0 was not applied correctly.

### 8.3 Mutation test validation

1. Run `/create_mutation_test_json` to regenerate test plans
2. Run prepare.py and verify filtering output:
   ```bash
   python3 .claude/scripts/mutation_test/prepare.py
   ```

   Expected output should include:
   ```
   Filtering paths with unavailable root examples...
     Excluding extras_plugin::TestMixedMutabilityEnum.value: Cannot construct Multiple variant via BRP...
     Kept 3 paths, excluded 2 for extras_plugin::TestMixedMutabilityEnum
   ```

3. Verify TestMixedMutabilityEnum paths for Multiple/WithMixed are filtered
4. Run batch 15 mutation tests:
   ```bash
   .claude/commands/mutation_test.sh
   ```
5. Verify no failures related to variant construction

### 8.4 Regression testing

Test with other enum types to ensure no regressions:
- `Option` (Mutable variants)
- `Result` (Mutable variants)
- `Handle` (may have PartiallyMutable variants)
- Regular enums without Arc fields

---

## Expected Outcomes

### Type Guide Output
1. **root_example** shows correct variant structure (not fallback to wrong variant)
2. **root_example_unavailable_reason** explains why with actual field reasons extracted from `mutability_reason`
3. **Users understand** which fields are problematic and why (e.g., Arc, recursion limit, missing trait)

### Mutation Testing
1. **Unconstructible paths filtered** during prepare.py execution
2. **No test failures** from trying to construct PartiallyMutable/NotMutable variants
3. **Clear logs** showing what was excluded and why

### Documentation
1. **Manual users** can see partial structure even if unconstructible
2. **Clear guidance** on when paths are usable (entity already in variant)
3. **Accurate information** about field-level mutability issues

---

## Implementation Order

1. **Phase 1** - Type system changes (EnumPathData, PathInfo)
2. **Phase 2** - Analysis function (analyze_variant_constructibility)
3. **Phase 3** - Remove fallback and build reasons (update build_partial_root_examples)
4. **Phase 4** - Propagation (thread reasons through all functions)
5. **Phase 5** - Serialization (expose in JSON)
6. **Phase 6** - Initialization (EnumPathData construction)
7. **Build and compile** - `cargo build`, verify no errors
8. **Phase 7** - Mutation test integration (prepare.py)
9. **Phase 8** - Testing and validation

---

## Success Criteria

- [ ] Type guide shows variant-specific root_example for all variants
- [ ] root_example_unavailable_reason explains unconstructible variants with actual reasons
- [ ] Mutation tests skip unconstructible paths
- [ ] No regression in existing enum handling (Option, Result, Handle, etc.)
- [ ] Batch 15 completes without variant construction failures
- [ ] Documentation is clear and actionable for manual users

---

## Rollback Plan

If issues arise:
1. Revert `enum_path_builder.rs` changes (restore fallback at lines 565-570)
2. Remove new fields from `EnumPathData` and `PathInfo`
3. Restore original function signatures
4. Keep prepare.py changes (defensive, won't break existing code)

---

## Files Modified Summary

### Rust (7 files):
1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Add fields
2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` - Major changes (analysis function, remove fallback, propagation)
3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` - Update serialization
4. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` - Update propagation helper

### Python (1 file):
5. `.claude/scripts/mutation_test/prepare.py` - Filter unconstructible paths

### Total Estimate:
- Rust implementation: 4-6 hours
- Python integration: 1 hour
- Testing/validation: 2-3 hours
- **Total: 7-10 hours**
