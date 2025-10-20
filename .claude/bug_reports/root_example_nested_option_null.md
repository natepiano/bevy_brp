# Bug: Incorrect `root_example` for Nested Option Types

## Status
**IDENTIFIED - FIX READY**

## Summary
For deeply nested `Option` types (e.g., `Option<Option<Option<...>>>`), the `root_example` field is incorrectly set to `null` instead of the proper nested array structure needed to access `.0` paths.

## Affected Component
`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

## Test Case
**Type**: `extras_plugin::RecursionDepthTestComponent`
```rust
struct RecursionDepthTestComponent {
    deeply_nested: Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<f32>>>>>>>>>>>
}
```

## Actual Behavior
All nested paths have `root_example: {"deeply_nested": null}`:
- `.deeply_nested.0` → `{"deeply_nested": null}` ❌
- `.deeply_nested.0.0` → `{"deeply_nested": null}` ❌
- `.deeply_nested.0.0.0` → `{"deeply_nested": null}` ❌

## Expected Behavior
Each level should have the correct nesting depth:
- `.deeply_nested.0` → `{"deeply_nested": [null]}` ✅ (Some(None))
- `.deeply_nested.0.0` → `{"deeply_nested": [[null]]}` ✅ (Some(Some(None)))
- `.deeply_nested.0.0.0` → `{"deeply_nested": [[[null]]]}` ✅ (Some(Some(Some(None))))

## Root Cause

### Call Stack
1. **Function**: `build_partial_root_examples` (line 539-638)
2. **At line 600-605**:
   ```rust
   let children = support::collect_children_for_chain(&child_refs, ctx, Some(child_chain));
   let wrapped = build_variant_example(signature, variant.name(), &children, ctx.type_name());
   ```

### The Problem
For nested `Option` types at recursion depth 10:
1. Child at depth 11 hits `MAX_TYPE_RECURSION_DEPTH` → returns `NotMutable`
2. `collect_children_for_chain` filters out `NotMutable` children → returns empty `HashMap`
3. `build_variant_example` with empty children → constructs `{"Some": null}`
4. `apply_option_transformation` unwraps → returns `null`
5. This `null` propagates up through parent levels
6. All ancestor `root_example` fields become `{"deeply_nested": null}`

### Why This Breaks Mutations
Setting `.deeply_nested` to `null` creates the `None` variant (Unit variant with no fields).
Attempting to access `.0` on a Unit variant fails:
```
Error: Expected variant index access to access a Tuple variant, found a Unit variant instead.
```

## The Fix

### Location
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`
**Function**: `build_partial_root_examples`
**Lines**: Around 573-610

### Strategy
1. **Extract duplicated logic** into helper function
2. **Apply Option::Some null fix** in single location
3. **Eliminate code duplication** between child_chain and our_chain processing

### Constants

**Note**: There are currently NO constants defined in `enum_path_builder.rs`. This will be the first one.

**Add after the use statements (after line 61):**

```rust
/// Variant name for Option::Some
const OPTION_SOME_VARIANT: &str = "Some";
```

### New Helper Function (add after line 248)

```rust
/// Build and wrap a variant example for Option types
///
/// This centralizes the common pattern used in build_partial_root_examples:
/// 1. Collect children for a variant chain
/// 2. For Option::Some, wrap each child value to add one nesting level
/// 3. Build variant example from (wrapped) children
///
/// For Option::Some variants, each child value is wrapped in an array before
/// build_variant_example processes it. This ensures proper nesting:
/// - Child has: [null]
/// - We wrap to: [[null]]
/// - build_variant_example creates: {"Some": [[null]]}
/// - apply_option_transformation unwraps to: [[null]]
/// - Result: Correct nesting depth achieved
///
/// Returns the wrapped example value ready for insertion into partial_root_examples.
fn build_and_wrap_variant(
    signature: &VariantSignature,
    variant_name: &str,
    child_mutation_paths: &[MutationPathInternal],
    variant_chain: &[VariantName],
    ctx: &RecursionContext,
) -> Value {
    use super::option_classification::OptionClassification;

    let child_refs: Vec<&MutationPathInternal> = child_mutation_paths.iter().collect();
    let mut children = support::collect_children_for_chain(&child_refs, ctx, Some(variant_chain));

    let is_option = OptionClassification::from_type_name(ctx.type_name()).is_option();

    // Special handling for Option::Some variants
    if is_option && variant_name == OPTION_SOME_VARIANT {
        if children.is_empty() {
            // No children (filtered NotMutable) - return minimal Some(None) value
            return json!([null]);
        }

        // Wrap each child value to add one level of nesting
        // This compensates for apply_option_transformation's unwrapping
        children = children
            .into_iter()
            .map(|(descriptor, value)| (descriptor, json!([value])))
            .collect();
    }

    build_variant_example(signature, variant_name, &children, ctx.type_name())
}
```

### Apply Fix in build_partial_root_examples (lines 573-610)

**Replace the duplicated code blocks with:**

```rust
let mut found_child_chains = false;
for child_chain in &child_chains_to_wrap {
    let wrapped = build_and_wrap_variant(
        signature,
        variant.name(),
        &child_mutation_paths,
        child_chain,
        ctx,
    );
    partial_root_examples.insert(child_chain.clone(), wrapped);
    found_child_chains = true;
}

// After processing all child chains, also create entry for n-variant chain
if found_child_chains {
    let wrapped = build_and_wrap_variant(
        signature,
        variant.name(),
        &child_mutation_paths,
        &our_chain,
        ctx,
    );
    partial_root_examples.insert(our_chain.clone(), wrapped);
} else {
    // No child chains found, this is a leaf variant - store base example
    partial_root_examples.insert(our_chain, base_example);
}
```

### Benefits
- **Single fix location**: Option::Some null fix in one place
- **No duplication**: Eliminates repeated `collect_children_for_chain` + `build_variant_example` pattern
- **Clearer intent**: Helper name documents what the code does
- **Easier maintenance**: Future changes only need to happen once

### Why This Works

The fix wraps child values BEFORE `build_variant_example` processes them:

**Level 10** (deepest, child is NotMutable):
- `collect_children_for_chain` returns: `{}` (empty)
- Wrap empty children: still `{}`
- `build_variant_example` with empty: `{"Some": null}`
- `apply_option_transformation` unwraps: `null`
- But empty means no child value to propagate - **this is actually fine!**
- The issue is at Level 9...

Wait, let me reconsider. If Level 10's child is NotMutable and filtered out, there IS no child value. The problem is we need to CREATE a value at the first level where we have no children.

**Actually, the fix needs to handle the empty children case differently:**
- When `children` is empty AND we're Option::Some → return `[null]` directly
- When `children` has values AND we're Option::Some → wrap each value before processing

**Updated logic:**
1. **Level 10**: No children (NotMutable) → return `[null]` directly
2. **Level 9**: Child value `[null]` → wrap to `[[null]]` → unwrap to `[[null]]`
3. **Level 8**: Child value `[[null]]` → wrap to `[[[null]]]` → unwrap to `[[[null]]]`

## Testing
After fix, verify:
1. All `.deeply_nested.0*` paths have correct `root_example` nesting
2. Mutation test for `RecursionDepthTestComponent` passes
3. Other nested Option types still work correctly

## Related Files
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (fix location)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/option_classification.rs` (helper)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` (child filtering)
- `test-app/examples/extras_plugin.rs` (test case)
