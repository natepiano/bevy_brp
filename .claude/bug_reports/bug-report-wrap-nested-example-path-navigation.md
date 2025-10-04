# Bug Report: wrap_nested_example Path Navigation Error [FIXED]

## Executive Summary
The `wrap_nested_example` function failed when wrapping nested enum partial roots for fields like `color_lut` that contain `Handle<T>` enums. The function incorrectly tried to navigate through the parent struct's field name before inserting the wrapped value, causing "Field not found" errors.

**Status:** ✅ FIXED in enum_path_builder.rs:909

## Symptom

When running `brp_all_type_guides` or querying `ChromaticAberration` type:

```
Internal error: Invalid state: Field 'color_lut' not found while navigating to index 0
```

## Affected Type

**`bevy_core_pipeline::post_process::ChromaticAberration`**

The type has a field:
```rust
color_lut: Handle<Image>
```

Which creates nested paths like:
- `.color_lut` - The Handle enum itself
- `.color_lut.0` - The tuple element (AssetId or Arc)
- `.color_lut.0.index` - Fields within AssetId

## Root Cause Analysis

### The Problem

In `wrap_nested_example` (enum_path_builder.rs:~826-890), when processing a child path like `.color_lut.0`:

1. **Line 849**: Parse path `.color_lut.0` into segments: `["color_lut", "0"]`
2. **Line 855**: Calculate `path_to_parent` as everything except last segment: `["color_lut"]`
3. **Line 885**: Call `navigate_and_replace_index(variant_content, ["color_lut"], index=0, ...)`

The function tries to navigate through "color_lut" field in `variant_content` before replacing at index 0.

**But:** The `variant_content` doesn't HAVE a `color_lut` field yet - that's the field we're trying to INSERT the wrapped value into!

### Why This Happens

The path `.color_lut.0` represents:
- `color_lut` - A field in ChromaticAberration struct
- `0` - Tuple index in the Handle<Image> enum

The confusion is:
- `wrap_nested_example` receives the **child's** `full_mutation_path` which is `.color_lut` (the Handle enum)
- When wrapping the Handle enum's partial root, we need to INSERT it at the `color_lut` field
- But the code treats `.color_lut` as a navigation path through the parent, not as the insertion target

### Expected vs Actual Behavior

**What SHOULD happen:**
- For child path `.color_lut` (Handle enum):
  - Navigate: nowhere (we're at variant root)
  - Insert at: field "color_lut"
  - Result: `{"VariantName": {"color_lut": <wrapped_handle_value>, ...}}`

**What ACTUALLY happens:**
- For child path `.color_lut`:
  - Parse segments: `["color_lut"]`
  - path_to_parent: `[]` (empty - correct!)
  - Navigate: works (empty path)
  - Insert at: field "color_lut"
  - Result: Should work...

Wait - if the path is `.color_lut`, then segments = `["color_lut"]`, and path_to_parent = `[]` (empty).

Let me reconsider... The error says "navigating to index 0", which means it's going through the IndexedElement branch. This suggests the child_path we're wrapping has a PathKind::IndexedElement, not StructField.

**Ah!** The child being wrapped is probably `.color_lut.0` (the tuple element), not `.color_lut` (the Handle enum itself).

For path `.color_lut.0`:
- Segments: `["color_lut", "0"]`
- path_to_parent: `["color_lut"]`
- PathKind: IndexedElement { index: 0 }
- Calls: `navigate_and_replace_index(variant_content, ["color_lut"], index=0, ...)`

**The Issue:** This tries to navigate through `variant_content["color_lut"]` to reach the location where index 0 should be replaced. But `variant_content` is the ChromaticAberration struct data, which doesn't have a `color_lut` key yet in the parent example we're building.

### The Core Misunderstanding

`wrap_nested_example` is designed to wrap a child enum's partial root INTO a parent enum's example. It navigates through the parent's variant content to find where to insert the child's partial root.

**For struct fields** (like `.middle_struct.nested_enum`):
- Parent has `middle_struct` field
- Child enum is at `nested_enum` within `middle_struct`
- Navigation works: variant_content["middle_struct"]["nested_enum"] = child_partial_root

**For tuple/indexed paths** (like `.color_lut.0`):
- Parent struct (ChromaticAberration) has `color_lut` field
- Child is the Handle enum AT `.color_lut`
- But we're trying to wrap the partial root of `.color_lut.0` (tuple element)

Wait, that's wrong too. Let me think about what's being wrapped.

### What's Actually Being Wrapped

The bottom-up algorithm builds partial roots at each enum level:
1. **Handle<Image>** enum (at path `.color_lut`) builds partial roots for its variants
2. **ChromaticAberration** (root) tries to wrap Handle's partial roots

When ChromaticAberration wraps the Handle partial root:
- It looks for child paths with `partial_root_examples`
- It finds the Handle enum at path `.color_lut`
- The Handle's PathKind is `StructField { field_name: "color_lut" }`
- So it should use the StructField branch, not IndexedElement!

But the error says "navigating to index 0", which means IndexedElement branch.

**Hypothesis:** The child_path being wrapped is NOT the Handle enum itself (`.color_lut`), but one of its descendants (`.color_lut.0`).

But that doesn't make sense - we should only be wrapping enum root paths (paths with `partial_root_examples`), and `.color_lut.0` is not an enum root path.

## Investigation Needed

Need to add debug tracing to determine:
1. What is the `child_path.full_mutation_path` when the error occurs?
2. What is the `child_path.path_kind`?
3. What partial root are we trying to wrap?
4. What does the parent example look like?

## Temporary Workaround

None - this blocks generation of type guides for any type containing Handle<T> fields in struct variants.

## Impact

**Severity: HIGH**
- Blocks `brp_all_type_guides` from completing
- Affects any type with `Handle<T>` fields: ChromaticAberration, many rendering types
- Breaks the mutation test creation workflow

## Related Code

- `wrap_nested_example`: enum_path_builder.rs:~820-900
- `navigate_and_replace_index`: enum_path_builder.rs:~950-1010
- `build_partial_root_for_chain`: enum_path_builder.rs:~760-850

## Solution Implemented

Changed the `IndexedElement` case in `wrap_nested_example` (enum_path_builder.rs:909):

**Before:**
```rust
if *index == 0 && path_to_parent.is_empty() {
    nested_partial_root.clone()
} else {
    navigate_and_replace_index(variant_content, path_to_parent, *index, nested_partial_root)?
}
```

**After:**
```rust
if *index == 0 && segments.len() == 2 {
    // Path is ".field_name.index" where field_name is parent struct field
    // When wrapping within enum, ignore the struct field prefix
    nested_partial_root.clone()
} else {
    // Multi-element tuples or more complex nesting
    navigate_and_replace_index(variant_content, path_to_parent, *index, nested_partial_root)?
}
```

**Key Insight:** For paths like `.color_lut.0`:
- `color_lut` is the parent **struct's** field (ChromaticAberration)
- `0` is the tuple index **within** the Handle enum
- When wrapping at the enum level, we should ignore the struct field prefix
- Check `segments.len() == 2` instead of `path_to_parent.is_empty()`

## Verification

Tested with `ChromaticAberration` type - now generates correctly:
- `.color_lut.0.uuid` has `root_example: {"Weak": {"Uuid": {"uuid": "..."}}}`
- `.color_lut.0.index` has `root_example: {"Weak": {"Index": {"index": {...}}}}`

The AssetId enum's partial roots are correctly wrapped into Handle enum's variants.
