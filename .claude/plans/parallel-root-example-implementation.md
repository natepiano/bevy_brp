# Parallel Root Example Implementation Plan

## Problem Statement

The current `wrap_nested_example` approach is complex and has bugs with mixed paths like `.target.0.handle.0`. Every attempt to simplify by "building during ascent" has failed in ways we haven't documented.

**Key Insight:** We keep having the same conversation because we don't write down what fails and why.

## Solution: Parallel Implementation

Instead of redesigning the entire system, implement a PARALLEL path that builds `root_example` simply, while keeping the existing complex approach. This allows us to:

1. Compare outputs to understand where they differ
2. Document exactly what breaks with the simple approach
3. Incrementally fix issues without breaking working types
4. Eventually replace the complex approach once the simple one works

## Architecture Overview

### Current Flow (Complex)
```
process_children()
  ├─ Builds enum_examples (for "" root path)
  └─ Returns child_paths

build_partial_roots()  ← Called AFTER process_children
  ├─ For each variant chain
  │   └─ build_partial_root_for_chain()
  │       ├─ Gets base_example from enum_examples (HAS WRONG NESTED VARIANTS!)
  │       └─ Calls wrap_nested_example() to PATCH IT
  │           └─ Complex navigation to find and replace nested enum values
  └─ Returns Map<chain → wrapped_example>

populate_root_example()
  └─ Sets root_example on paths from partial_roots map
```

### New Flow (Simple - Parallel)
```
process_children()
  ├─ Builds enum_examples (for "" root path) [KEEP AS-IS]
  │
  └─ NEW: When child returns partial_root_examples_new:
      ├─ For each chain in child's partial_root_examples_new
      │   └─ Wrap with our variant → store in OUR partial_root_examples_new
      └─ Pass up the chain

populate_root_example_new()
  └─ Sets root_example_new on paths from partial_root_examples_new
```

**Key Difference:** Wrapping happens DURING ascent in `process_children`, not later in a separate phase.

## Implementation Steps

### Step 1: Add New Fields

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Add to `MutationPathInternal`:
```rust
/// NEW: Simple approach - root example built during ascent (for comparison)
pub root_example_new: Option<Value>,

/// NEW: Partial roots built during ascent (for comparison)
pub partial_root_examples_new: Option<BTreeMap<Vec<VariantName>, Value>>,
```

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Initialize both fields to `None` in `build_mutation_path_internal`.

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Initialize both fields to `None` in `create_result_paths`.

### Step 2: Implement New Building During Ascent

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Add new function:
```rust
/// New wrapping during ascent - builds partial_root_examples_new
///
/// Unlike the complex approach, this wraps child partial roots IMMEDIATELY
/// during recursion, not in a separate phase.
fn build_partial_roots_new(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> BTreeMap<Vec<VariantName>, Value> {
    let mut partial_roots = BTreeMap::new();

    // For each variant at THIS level
    for (_, variants) in variant_groups {
        for variant in variants {
            let our_variant = variant.variant_name().clone();

            // Build our variant chain by extending parent's chain
            let mut our_chain = ctx.variant_chain.iter()
                .map(|vp| vp.variant.clone())
                .collect::<Vec<_>>();
            our_chain.push(our_variant.clone());

            // Get base example for this variant
            let base_example = enum_examples.iter()
                .find(|ex| ex.applicable_variants.contains(&our_variant))
                .map(|ex| ex.example.clone())
                .unwrap_or(json!(null));

            // Check if base_example contains nested enums that need wrapping
            let wrapped = wrap_child_partial_roots_new(
                &base_example,
                &our_chain,
                child_paths,
            );

            partial_roots.insert(our_chain, wrapped.unwrap_or(base_example));
        }
    }

    partial_roots
}

/// New wrapper - just replaces child enum fields with their partial roots
fn wrap_child_partial_roots_new(
    base_example: &Value,
    our_chain: &[VariantName],
    child_paths: &[MutationPathInternal],
) -> Option<Value> {
    // Look for child enums that have partial_root_examples_new
    for child in child_paths {
        if let Some(child_partials) = &child.partial_root_examples_new {
            // Does child have a partial root for our chain?
            if let Some(child_root) = child_partials.get(our_chain) {
                // Simple replace: use child's path to find where to insert
                // For now, just try direct field replacement
                return replace_field_new(base_example, child, child_root);
            }
        }
    }

    None
}

/// Simplest possible replacement - just replace the field named in child.path_kind
fn replace_field_new(
    parent: &Value,
    child: &MutationPathInternal,
    new_value: &Value,
) -> Option<Value> {
    match &child.path_kind {
        PathKind::StructField { field_name, .. } => {
            let mut obj = parent.as_object()?.clone();
            obj.insert(field_name.to_string(), new_value.clone());
            Some(Value::Object(obj))
        }
        PathKind::IndexedElement { index, .. } if *index == 0 => {
            // Single-element tuple - just return the value
            Some(new_value.clone())
        }
        _ => None, // Not supported yet
    }
}
```

Call from `process_children`:
```rust
// After building enum_examples
let partial_roots_new = build_partial_roots_new(
    variant_groups,
    &all_examples,
    &all_child_paths,
    ctx,
);
```

Store in root path:
```rust
root_mutation_path.partial_root_examples_new = Some(partial_roots_new.clone());
```

Populate paths if at root level:
```rust
if ctx.variant_chain.is_empty() {
    populate_root_example(&mut child_paths, &partial_roots);  // OLD
    populate_root_example_new(&mut child_paths, &partial_roots_new);  // NEW
}
```

### Step 3: Add root_example_new to Output

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Add to `MutationPath` (the public output struct):
```rust
/// NEW: Root example built with new approach (for comparison)
#[serde(skip_serializing_if = "Option::is_none")]
pub root_example_new: Option<Value>,
```

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Update the conversion from `MutationPathInternal` to `MutationPath` to include `root_example_new`:
```rust
MutationPath {
    // ... existing fields ...
    root_example_new: internal.root_example_new,
}
```

This allows direct comparison in the tool output without needing to check logs.

### Step 4: Testing Strategy

#### Test 1: TestVariantChainEnum (Should Match)
```bash
cargo install --path mcp
# reconnect
mcp__brp__brp_type_guide types=["extras_plugin::TestVariantChainEnum"]
```

Check output for:
- ✅ Compare `root_example` vs `root_example_new` fields in each mutation path
- ✅ All paths should have matching values
- This validates the new approach works for basic cases

#### Test 2: Camera (Will Show Differences)
```bash
mcp__brp__brp_type_guide types=["bevy_render::camera::camera::Camera"]
```

Check output for:
- ❌ Compare `root_example` vs `root_example_new` for paths like `.target.0.handle.0`
- Document exactly WHAT differs (which nested enum variant is different)
- Use jq or visual diff to identify mismatches

#### Test 3: Full Type Guide (Measure Coverage)
```bash
mcp__brp__brp_all_type_guides
```

Analyze output:
- Count paths where `root_example == root_example_new`
- Count paths where they differ
- Identify which types cause mismatches
- Can use script to parse JSON and compare fields

### Step 5: Analysis & Iteration

For each MISMATCH:
1. **Document the pattern** - What path structure causes it?
2. **Understand why** - What does the simple approach do wrong?
3. **Enhance simple approach** - Add handling for that pattern
4. **Re-test** - Did the fix work? Did it break anything else?

Expected patterns to handle:
- ✅ Direct struct fields with nested enums (`.middle_struct.nested_enum`)
- ❌ Tuple indices with nested enums (`.color_lut.0`) - May need refinement
- ❌ Mixed paths (`.target.0.handle.0`) - Likely needs special handling
- ❓ Multiple levels of nesting - Unknown

### Step 6: Replacement (Future)

Once `root_example_new` matches `root_example` for all types:
1. Rename `root_example_new` → `root_example`
2. Remove old complex implementation
3. Delete `wrap_nested_example`, `build_partial_root_for_chain`, etc.

## Success Criteria

**Phase 1 (Validation):**
- ✅ TestVariantChainEnum: `root_example` == `root_example_new` for all paths
- ✅ No errors/panics during parallel building
- ✅ Can see exact differences in Camera type output

**Phase 2 (Iteration):**
- ✅ Fix at least one mismatch pattern
- ✅ Document why the fix was needed
- ✅ Verify fix doesn't break matching cases

**Phase 3 (Completion - Long Term):**
- ✅ All types: `root_example` == `root_example_new` for all paths
- ✅ Remove old implementation
- ✅ Simpler, more maintainable code

## Notes & Observations

### Why Previous "Simple" Attempts Failed

**UNKNOWN** - This is what we'll discover through comparison logging!

Document failures here as we encounter them:
- Pattern: ...
- Why it failed: ...
- How to handle: ...

### Current Bugs That May Be Fixed

If the simple approach works correctly, it should automatically fix:
- `.target.0.handle.0` mixed path navigation error
- Any other path navigation failures in `wrap_nested_example`

### Constraints We Discover

Document any fundamental constraints that prevent simple building:
- Constraint: ...
- Why it exists: ...
- How complex approach handles it: ...
