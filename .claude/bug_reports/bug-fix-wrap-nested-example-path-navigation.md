# Fix: wrap_nested_example Path Navigation for Nested Enums in Tuples

## Problem Analysis

From trace log (lines 49-69):
```
wrap_nested_example called:
  child_path.full_mutation_path: .color_lut.0
  child_path.path_kind: IndexedElement { index: 0, type_name: BrpTypeName("bevy_asset::id::AssetId<bevy_image::image::Image>"), parent_type: BrpTypeName("bevy_asset::handle::Handle<bevy_image::image::Image>") }
  parent_example: {"Weak": {"Index": {"index": {...}}}}
  nested_partial_root: {"Index": {"index": {...}}}
```

**Error:** `Field 'color_lut' not found while navigating to index 0`

## Root Cause

The path `.color_lut.0` has two components:
1. `.color_lut` - StructField in ChromaticAberration
2. `.0` - IndexedElement in Handle<Image> enum

When wrapping the AssetId enum's partial root into the Handle enum's example:
- Current code parses `.color_lut.0` → segments = `["color_lut", "0"]`
- path_to_parent = `["color_lut"]`
- Tries to navigate: `variant_content["color_lut"][0]`
- **ERROR:** variant_content is `{"Index": {"index": {...}}}` - no "color_lut" field!

**The Confusion:**
- The parent enum (Handle) doesn't know about the struct field name (`color_lut`)
- That field name is from the PARENT STRUCT (ChromaticAberration), not the enum
- We're wrapping within the Handle enum's variants, not navigating through ChromaticAberration

## The Real Hierarchy

```
ChromaticAberration (struct)
  └─ color_lut: Handle<Image> (enum, path: .color_lut)
       ├─ Weak variant (tuple with 1 element)
       │   └─ 0: AssetId<Image> (enum, path: .color_lut.0)
       │        ├─ Index variant (struct)
       │        │   └─ index: AssetIndex
       │        └─ Uuid variant (struct)
       │            └─ uuid: Uuid
       └─ Strong variant (tuple with 1 element)
           └─ 0: Arc<StrongHandle> (not an enum)
```

When `wrap_nested_example` is called:
- **Parent:** Handle's Weak variant example: `{"Weak": {"Index": {"index": {...}}}}`
- **Child:** AssetId's partial root: `{"Index": {"index": {...}}}`
- **Goal:** Replace the AssetId example in Weak variant with AssetId's partial root

## The Fix

**DON'T navigate through the full mutation path!**

The `full_mutation_path` includes ancestors that aren't relevant when wrapping within an enum.

For `IndexedElement` paths, the wrapping should be:
1. Extract index from PathKind (not from parsing the full path)
2. NO navigation - replace directly at the index in the parent variant

For `StructField` paths nested in enums, similar logic applies - the field name in PathKind is relative to the immediate parent, not the full path.

### Proposed Solution

Change `wrap_nested_example` to:
1. Use `PathKind` directly to determine what to replace
2. For `IndexedElement`: Replace at index in the variant content (no navigation)
3. For `StructField`: Replace the field in the variant content (no navigation)
4. Only navigate if there are intermediate struct levels WITHIN the enum variant

But wait - how do we know if there are intermediate levels?

Actually, the current implementation is CORRECT for cases like `.middle_struct.nested_enum` where:
- Path: `.middle_struct.nested_enum`
- Segments: `["middle_struct", "nested_enum"]`
- Navigation: `variant_content["middle_struct"]` exists
- Replace: the `nested_enum` field within `middle_struct`

The bug is specific to `IndexedElement` paths where the segments include a parent struct field name.

### Root Issue Identification

The problem is that for `IndexedElement { index: 0, parent_type: Handle }`, the `full_mutation_path` is `.color_lut.0`:
- This path includes the parent STRUCT's field name (`color_lut`)
- But we're wrapping WITHIN the Handle enum, not at the struct level

**Solution:** For `IndexedElement` and `ArrayElement`, extract ONLY the index-based suffix from the path, ignoring struct field prefixes.

## Implementation

Change `wrap_nested_example` logic:

```rust
fn wrap_nested_example(
    parent_example: &Value,
    nested_partial_root: &Value,
    child_path: &MutationPathInternal,
) -> Result<Value> {
    // Unwrap variant wrapper
    let (variant_name, variant_content) = ...;

    // For IndexedElement/ArrayElement: NO navigation, replace directly at index
    let new_content = match &child_path.path_kind {
        PathKind::IndexedElement { index, .. } | PathKind::ArrayElement { index, .. } => {
            // The parent is already at the enum variant level
            // Just replace the tuple element at this index
            replace_tuple_element(variant_content, *index, nested_partial_root)?
        }

        PathKind::StructField { field_name, .. } => {
            // For struct fields, parse the path to find intermediate navigation
            // But exclude the field name itself (it's the target, not part of navigation)
            let path_str = child_path.full_mutation_path.trim_start_matches('.');
            let segments: Vec<&str> = path_str.split('.').collect();

            // All segments except the last are navigation path
            let path_to_parent = &segments[..segments.len() - 1];

            navigate_and_replace_field(
                variant_content,
                path_to_parent,
                field_name.as_str(),
                nested_partial_root,
            )?
        }

        PathKind::RootValue { .. } => {
            return Err(...);
        }
    };

    Ok(json!({ variant_name: new_content }))
}
```

### New Helper: `replace_tuple_element`

```rust
fn replace_tuple_element(
    variant_content: &Value,
    index: usize,
    new_value: &Value,
) -> Result<Value> {
    // The variant_content for a tuple variant is the nested partial root itself
    // Just return the new value directly
    Ok(new_value.clone())
}
```

Wait, that's not right either. Let me re-examine the structure.

Looking at the trace log line 37:
```
Result: Object {"Weak": Object {"Index": Object {"index": Object {"generation": Number(1000000), "index": Number(1000000)}}}}
```

The Handle enum's Weak variant example is:
```json
{
  "Weak": {
    "Index": {
      "index": {"generation": 1000000, "index": 1000000}
    }
  }
}
```

This is WRONG. The Handle<Image> enum should look like:
```json
{
  "Weak": <AssetId value>
}
```

But instead it has `{"Index": ...}` which is the AssetId enum's example nested inside.

OH! The nested_partial_root being passed IS the correct value to replace the tuple element with. The parent example `{"Weak": {...}}` has the old AssetId example, and we want to replace it with the AssetId's partial root.

So for tuple variants, the content after unwrapping the variant name IS the value at index 0. We just need to replace it entirely!

## Correct Fix

For `IndexedElement` in tuple variants:
- The variant_content after unwrapping IS the tuple element value
- For single-element tuples (common for newtypes), just replace the entire content
- For multi-element tuples, need to handle array indexing

```rust
PathKind::IndexedElement { index, .. } => {
    // For single-element tuple newtypes, the variant content IS the element
    // Just return the new value
    nested_partial_root.clone()
}
```

Let me verify this understanding with the actual structure from the trace.

Actually, re-reading line 49-69, the parent_example after wrapping should be:
```json
{
  "Weak": <nested_partial_root>
}
```

Where `<nested_partial_root>` is the AssetId partial root.

So the fix is simple: for `IndexedElement` when index is 0 and it's a single-element tuple, just replace the variant content entirely with the nested partial root!
