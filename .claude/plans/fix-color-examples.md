# Fix Color Enum Field Name Bug

## Problem Statement

Enum variants with the same structural signature but different field names have their examples generated with incorrect field names. This causes mutation operations to fail with "missing field" errors.

### Example

For `bevy_color::color::Color`:

**What SHOULD be generated:**
```json
{
  "Srgba": {"red": 3.14, "green": 3.14, "blue": 3.14, "alpha": 3.14}
}
```

**What IS ACTUALLY generated:**
```json
{
  "Srgba": {"x": 3.14, "y": 3.14, "z": 3.14, "alpha": 3.14}
}
```

All variants (`Srgba`, `Hsla`, `Xyza`, etc.) get the same field names (`x`, `y`, `z`, `alpha`) instead of their correct field names.

## Root Cause

The bug is in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`.

### Current Flow (Broken)

1. **`process_children()` creates a SHARED `child_examples` HashMap** (line 111)
2. For each variant group:
   - Recurse into the wrapped struct type (`Srgba`, `Hsla`, `Xyza`)
   - Get the struct example
   - Insert into shared HashMap: `child_examples["0"] = example`
3. **Problem:** All variants use descriptor `"0"` (the tuple index), so each insertion OVERWRITES the previous one
4. **Result:** Only the LAST variant's example survives in the HashMap

### Why All Variants Use Same Descriptor

Color variants are newtype wrappers:
```rust
Color::Srgba(Srgba)  // Tuple variant with 1 element at index 0
Color::Hsla(Hsla)    // Tuple variant with 1 element at index 0
Color::Xyza(Xyza)    // Tuple variant with 1 element at index 0
```

All have signature `Tuple([inner_type])`, and all create child path at descriptor `"0"`.

### Example Trace

Processing Color enum:

1. **Process Srgba group:**
   - Recurse into `Srgba` struct → example: `{"red": 3.14, "green": 3.14, "blue": 3.14, "alpha": 3.14}`
   - Insert: `child_examples["0"] = {"red": ..., "green": ..., "blue": ..., "alpha": ...}`

2. **Process Hsla group:**
   - Recurse into `Hsla` struct → example: `{"hue": 3.14, "saturation": 3.14, "lightness": 3.14, "alpha": 3.14}`
   - Insert: `child_examples["0"] = {"hue": ..., "saturation": ..., "lightness": ..., "alpha": ...}` ← **OVERWRITES Srgba!**

3. **Process Xyza group:**
   - Recurse into `Xyza` struct → example: `{"x": 3.14, "y": 3.14, "z": 3.14, "alpha": 3.14}`
   - Insert: `child_examples["0"] = {"x": ..., "y": ..., "z": ..., "alpha": ...}` ← **OVERWRITES Hsla!**

4. **Build examples for ALL variants:**
   - All variants call `build_variant_example(signature, ..., &child_examples)`
   - All look up `child_examples["0"]`
   - All get the LAST inserted value: `{"x": ..., "y": ..., "z": ..., "alpha": ...}`

## Solution: Build Examples Inline

Instead of collecting all child examples in a shared HashMap then building examples later, **build each variant's example immediately** while we have the correct child examples.

### Key Changes

#### 1. Change `process_children` Return Type

**Before:**
```rust
fn process_children() -> (HashMap<MutationPathDescriptor, Value>, Vec<MutationPathInternal>)
```

**After:**
```rust
fn process_children() -> (Vec<ExampleGroup>, Vec<MutationPathInternal>)
```

#### 2. Move Example Building Into `process_children`

**Before (lines 103-211):**
```rust
fn process_children() {
    let mut child_examples = HashMap::new();  // ← SHARED across all variants

    for (signature, variants_in_group) in variant_groups {
        for path in paths {
            // Recurse to get struct example
            let child_paths = recurse(...);
            let child_example = child_paths.first().example;

            child_examples.insert("0", child_example);  // ← OVERWRITES previous!
        }
    }

    Ok((child_examples, all_child_paths))
}
```

**After:**
```rust
fn process_children() {
    let mut all_examples = Vec::new();     // ← Collect ExampleGroup directly
    let mut all_child_paths = Vec::new();

    for (signature, variants_in_group) in variant_groups {
        let mut child_examples = HashMap::new();  // ← FRESH for each group!

        for path in paths {
            // Recurse to get struct example
            let child_paths = recurse(...);
            let child_example = child_paths.first().example;

            child_examples.insert(descriptor, child_example);  // ← No collision
            all_child_paths.extend(child_paths);
        }

        // BUILD EXAMPLE IMMEDIATELY while we have the correct child_examples
        let example = build_variant_example(
            signature,
            representative.name(),
            &child_examples,  // ← Uses THIS variant's examples
            ctx.type_name()
        );

        all_examples.push(ExampleGroup {
            applicable_variants: variants_in_group.iter().map(...).collect(),
            signature: signature.to_string(),
            example,  // ← Correct example for THIS variant
        });
    }

    Ok((all_examples, all_child_paths))  // ← Return examples directly
}
```

#### 3. Simplify `process_enum`

**Before (lines 168-186):**
```rust
pub fn process_enum() {
    let variant_groups = extract_and_group_variants(ctx)?;
    let (child_examples, child_paths) = process_children(&variant_groups, ctx, depth)?;
    let (enum_examples, default_example) = build_enum_examples(&variant_groups, child_examples, ctx)?;
    create_result_paths(ctx, enum_examples, default_example, child_paths)
}
```

**After:**
```rust
pub fn process_enum() {
    let variant_groups = extract_and_group_variants(ctx)?;
    let (enum_examples, child_paths) = process_children(&variant_groups, ctx, depth)?;
    let default_example = select_preferred_example(&enum_examples).unwrap_or(json!(null));
    create_result_paths(ctx, enum_examples, default_example, child_paths)
}
```

#### 4. Remove or Simplify `build_enum_examples`

The function at lines 415-450 becomes redundant. Its logic moves into `process_children`, and only the `select_preferred_example` call remains.

## Why This Fix Works

### For Color::Srgba
1. Create **fresh** `child_examples = {}`
2. Recurse into `Srgba` struct → get `{"red": 3.14, "green": 3.14, "blue": 3.14, "alpha": 3.14}`
3. Insert `child_examples["0"] = <Srgba example>`
4. **Build example immediately:** `build_variant_example(Tuple([Srgba]), "Srgba", &child_examples)`
5. Result: `{"Srgba": {"red": 3.14, "green": 3.14, "blue": 3.14, "alpha": 3.14}}` ✅

### For Color::Hsla
1. Create **new fresh** `child_examples = {}` ← No shared state!
2. Recurse into `Hsla` struct → get `{"hue": 3.14, "saturation": 3.14, "lightness": 3.14, "alpha": 3.14}`
3. Insert `child_examples["0"] = <Hsla example>`
4. **Build example immediately:** `build_variant_example(Tuple([Hsla]), "Hsla", &child_examples)`
5. Result: `{"Hsla": {"hue": 3.14, "saturation": 3.14, "lightness": 3.14, "alpha": 3.14}}` ✅

### For Color::Xyza
1. Create **new fresh** `child_examples = {}` ← No shared state!
2. Recurse into `Xyza` struct → get `{"x": 3.14, "y": 3.14, "z": 3.14, "alpha": 3.14}`
3. Insert `child_examples["0"] = <Xyza example>`
4. **Build example immediately:** `build_variant_example(Tuple([Xyza]), "Xyza", &child_examples)`
5. Result: `{"Xyza": {"x": 3.14, "y": 3.14, "z": 3.14, "alpha": 3.14}}` ✅

## Why This Won't Break Anything

### 1. No API Changes from Caller's Perspective
- `process_enum` still returns `Vec<MutationPathInternal>`
- `create_result_paths` signature unchanged
- Return values match expected types

### 2. Same Logic, Different Timing
- `build_variant_example()` is still called with same parameters
- Each variant group still gets its `ExampleGroup` created with same fields
- Only difference is WHEN it's called (immediately vs deferred)

### 3. No Shared State Issues
- **Current bug:** Shared `child_examples` HashMap causes overwrites
- **Fixed:** Each variant group gets fresh HashMap
- No possibility of collision or overwriting

### 4. Edge Cases Handled

**Unit variants** (no children):
- `create_paths_for_signature(Unit)` returns `None`
- Loop doesn't run, `child_examples` stays empty
- Example built correctly ✅

**Struct variants** (with named fields):
- Each field gets unique descriptor (field name, not index)
- No collision possible even with shared HashMap
- After fix: Still works, just builds immediately ✅

**Multiple tuple elements** (e.g., `Result<T, E>`):
- Descriptors are "0", "1" - can collide if different groups have different types at same index
- **This is ALSO a bug!** Our fix solves this too ✅

**Nested enums**:
- Child paths extend correctly - no change in that logic
- `applicable_variants` population (lines 148-164) - no change needed
- Partial root building uses pre-built examples - no change needed ✅

**Empty enum**:
- `variant_groups` would be empty
- Loop doesn't run, returns `(vec![], vec![])`
- Works correctly ✅

## Implementation Steps

1. **Modify `process_children` function (lines 103-211):**
   - Change return type to `(Vec<ExampleGroup>, Vec<MutationPathInternal>)`
   - Move `child_examples` HashMap inside the variant group loop (make it fresh for each group)
   - Move example building logic from `build_enum_examples` into the loop
   - Build `ExampleGroup` immediately after processing each variant group

2. **Update `process_enum` function (lines 168-186):**
   - Change destructuring to `(enum_examples, child_paths)`
   - Remove `build_enum_examples` call
   - Add direct call to `select_preferred_example`

3. **Remove or simplify `build_enum_examples` (lines 415-450):**
   - Can be deleted entirely, or
   - Simplified to just the `select_preferred_example` logic if needed elsewhere

4. **Update debug tracing:**
   - Move debug logs from `build_enum_examples` into `process_children`
   - Ensure trace output shows when each variant's example is built

## Testing

After implementing the fix:

1. **Launch extras_plugin example:**
   ```bash
   mcp__brp__brp_launch_bevy_example extras_plugin
   ```

2. **Get type guide for Color:**
   ```bash
   mcp__brp__brp_type_guide ["bevy_color::color::Color"]
   ```

3. **Verify examples have correct field names:**
   - Srgba: `{"red", "green", "blue", "alpha"}`
   - Hsla: `{"hue", "saturation", "lightness", "alpha"}`
   - Xyza: `{"x", "y", "z", "alpha"}`
   - Each variant should have its OWN field names, not shared ones

4. **Test mutation:**
   ```rust
   mcp__brp__bevy_mutate_component {
       "entity": entity_id,
       "component": "bevy_pbr::fog::DistanceFog",
       "path": ".color",
       "value": {
           "Srgba": {
               "red": 1.0,
               "green": 0.0,
               "blue": 0.0,
               "alpha": 1.0
           }
       }
   }
   ```
   This should succeed, not fail with "missing field" errors.

## Related Files

- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs` - Main file to modify
- `.claude/bug_reports/bug-report-color-enum-fields.md` - Original bug report

## Benefits

1. ✅ Fixes Color enum field name bug
2. ✅ Fixes similar bugs for ANY enum with tuple variants wrapping different struct types
3. ✅ Eliminates shared mutable state
4. ✅ Makes code more maintainable (example building happens immediately, not deferred)
5. ✅ No performance impact (same number of operations, just different ordering)
