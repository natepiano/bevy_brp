# Bug Report: wrap_nested_example Mixed IndexedElement/StructField Navigation Error

## Status
**ACTIVE** - Blocks `brp_all_type_guides` from completing

## Symptom
When running `brp_all_type_guides`, the following error occurs:

```
Internal error: Invalid state: Expected object while navigating to field 'handle', found Null
```

## Affected Path
Path: `.target.0.handle.0`
- Segments: `["target", "0", "handle", "0"]`
- PathKind: `IndexedElement { index: 0, ... }`
- This is a MIXED path with both struct fields AND tuple indices

## Root Cause
The current fix for `wrap_nested_example` at enum_path_builder.rs:920 only handles pure IndexedElement paths where ALL non-terminal segments are struct field names:

```rust
if *index == 0 && path_to_parent.len() > 0 {
    // Single-element tuple (index 0) with parent struct fields in path
    // Just replace the variant content directly
    nested_partial_root.clone()
}
```

**The Problem:** This assumes `path_to_parent` contains ONLY struct field names. But for paths like `.target.0.handle.0`:
- `path_to_parent = ["target", "0", "handle"]`
- This contains BOTH a struct field ("target", "handle") AND a tuple index ("0")
- The code tries to replace directly, but doesn't handle the mixed navigation

## Example from Trace Log
```
wrap_nested_example called:
  child_path.full_mutation_path: .target.0.handle.0
  child_path.path_kind: IndexedElement { index: 0, ... }
  parent_example: {
    "Weak": {
      "Index": { ... }
    }
  }
  segments: ["target", "0", "handle", "0"]
  path_to_parent: ["target", "0", "handle"]
```

The path has:
1. `target` - struct field
2. `0` - tuple index (from some enum variant)
3. `handle` - struct field WITHIN that tuple element
4. `0` - final tuple index we're trying to replace

## The Real Issue
The current fix conflates two different scenarios:

### Scenario A: Pure struct nesting (current fix handles this)
- Path: `.color_lut.0` or `.analog.axis_data.key.0`
- All non-terminal segments are struct fields
- Solution: Direct replacement works

### Scenario B: Mixed struct/tuple nesting (BROKEN)
- Path: `.target.0.handle.0`
- Contains both struct fields AND tuple indices before the final element
- The middle `.0` is from a tuple variant, creating a structural boundary
- Solution: Need actual navigation through the mixed path

## Required Fix
The logic needs to distinguish between:
1. **Terminal tuple in struct fields**: All path segments except last are struct field names → direct replace
2. **Nested tuple in tuple**: Path contains other tuple indices → need actual navigation

Possible approaches:
- Check if `path_to_parent` contains numeric indices (indicating tuple elements)
- If yes, use `navigate_and_replace_index()`
- If no (all are field names), use direct replacement

## Related Code
- Location: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:920-941`
- Function: `wrap_nested_example()`
- Previous fix: Handled pure struct nesting but not mixed paths

## Workaround
None - this blocks all type guide generation for types with these nested patterns.

## Impact
**Severity: HIGH**
- Blocks `brp_all_type_guides` from completing
- Affects any type with Handle<T> nested in tuple variants
- Prevents mutation test creation workflow
