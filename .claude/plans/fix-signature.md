# Fix HashMap Collision via Field-Level Variant Grouping

## Problem

Color enum variants create duplicate mutation paths with identical string keys, causing HashMap collision:

```rust
// All 10 Color variants have .alpha field:
Srgba { red, green, blue, alpha }
Hsla { hue, saturation, lightness, alpha }
LinearRgba { red, green, blue, alpha }
// ... 7 more variants

// Current behavior generates SEPARATE paths for each variant:
Srgba generates: ".0.alpha" → path entry 1
Hsla generates: ".0.alpha" → path entry 2 (overwrites entry 1!)
LinearRgba generates: ".0.alpha" → path entry 3 (overwrites entry 2!)

// HashMap.collect() keeps only the LAST variant's path
// Result: 9 variants silently lost
```

## Current Working Behavior: Same-Signature Consolidation

The system ALREADY consolidates variants with identical signatures:

```rust
enum BottomEnum {
    VariantA(u32),  // Tuple signature: (u32)
    VariantD(u32),  // Same signature!
}

// Result: ONE path entry with BOTH variants:
".0": {
    "applicable_variants": ["BottomEnum::VariantA", "BottomEnum::VariantD"],
    "root_example": {"VariantA": 1}
}
```

**How it works:**
- `group_variants_by_signature()` (enum_path_builder.rs:197-222) groups by complete signature
- Tuple variants with same types → same signature → grouped together
- ONE path generated, `applicable_variants` lists all variants in group
- No HashMap collision because only one path entry created

## The Gap: Struct Variants with Shared Fields

Color variants have **different complete signatures** but **overlapping fields**:

```rust
Srgba signature:  Struct([("red", f32), ("green", f32), ("blue", f32), ("alpha", f32)])
Hsla signature:   Struct([("hue", f32), ("saturation", f32), ("lightness", f32), ("alpha", f32)])
```

These are **different signatures**, so they're processed separately. But they share the `alpha` field!

**Conceptual question:** Why not treat shared fields like shared tuple signatures?

- `VariantA(u32)` and `VariantB(u32)` → ONE `.0` path
- `Srgba { alpha }` and `Hsla { alpha }` → ONE `.0.alpha` path

## Proposed Solution: Field-Level Grouping for Struct Variants

Extend the consolidation strategy from signature-level to **field-level** for struct variants.

### Algorithm Change

**Current (enum_path_builder.rs:396-461):**
```rust
fn process_signature_groups(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    ctx: &RecursionContext,
) {
    // For each SIGNATURE group (complete signature match)
    for (signature, variants) in variant_groups.sorted() {
        // All variants in this group share COMPLETE signature
        let applicable_variants = variants.clone();

        // Generate paths for this signature
        for path_kind in create_paths_for_signature(signature, ctx) {
            process_signature_path(path_kind, &applicable_variants, ...);
        }
    }
}
```

**Proposed:**
```rust
fn process_signature_groups(...) {
    // For TUPLE variants: keep signature-level grouping (already works)
    // For STRUCT variants: group by individual field compatibility

    for (signature, variants) in variant_groups.sorted() {
        match signature {
            VariantSignature::Tuple(_) => {
                // Existing behavior: all variants in group share signature
                let applicable_variants = variants.clone();
                process_tuple_signature(signature, applicable_variants, ctx);
            }
            VariantSignature::Struct(fields) => {
                // NEW: group variants by FIELD compatibility
                process_struct_fields_by_compatibility(fields, variants, variant_groups, ctx);
            }
            VariantSignature::Unit => {
                // Unit variants have no fields
            }
        }
    }
}

fn process_struct_fields_by_compatibility(
    fields: &[(StructFieldName, BrpTypeName)],
    variants_with_this_signature: &[VariantName],
    all_variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    ctx: &RecursionContext,
) {
    // For each field in this signature
    for (field_name, field_type) in fields {
        // Find ALL variants (across ALL signatures) that have this field
        let variants_with_this_field = find_variants_with_field(
            field_name,
            field_type,
            all_variant_groups, // Passed as explicit parameter
        );

        // Generate ONE path for this field with ALL compatible variants
        let path_kind = PathKind::StructField {
            field_name: field_name.clone(),
            type_name: field_type.clone(),
            parent_type: ctx.type_name().clone(),
        };

        process_signature_path(
            path_kind,
            &variants_with_this_field, // List of all variants with this field
            signature,
            ctx,
            &mut child_examples,
        );
    }
}

fn find_variants_with_field(
    field_name: &StructFieldName,
    field_type: &BrpTypeName,
    all_variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
) -> Vec<VariantName> {
    let mut matching_variants = Vec::new();

    for (signature, variants) in all_variant_groups {
        if let VariantSignature::Struct(fields) = signature {
            // Check if this signature has the field
            if fields.iter().any(|(name, ty)| name == field_name && ty == field_type) {
                matching_variants.extend(variants.clone());
            }
        }
    }

    matching_variants
}
```

### Result for Color Example

```rust
// Field "alpha" (type f32) found in 10 variants:
".0.alpha": {
    "applicable_variants": [
        "Color::Srgba", "Color::Hsla", "Color::LinearRgba",
        // ... all 10 variants with alpha field
    ],
    "root_example": {"Srgba": {"red": 1.0, "green": 1.0, "blue": 1.0, "alpha": 1.0}},
    "example": 1.0
}

// Field "red" (type f32) found in 3 variants:
".0.red": {
    "applicable_variants": ["Color::Srgba", "Color::LinearRgba", "Color::Xyza"],
    "root_example": {"Srgba": {"red": 1.0, "green": 1.0, "blue": 1.0, "alpha": 1.0}},
    "example": 1.0
}

// Field "hue" (type f32) found in 2 variants:
".0.hue": {
    "applicable_variants": ["Color::Hsla", "Color::Hsva"],
    "root_example": {"Hsla": {"hue": 1.0, "saturation": 1.0, "lightness": 1.0, "alpha": 1.0}},
    "example": 1.0
}
```

**Benefits:**
- ✅ No HashMap collision (each unique field name → one entry)
- ✅ Shows which variants support each field mutation
- ✅ Conceptually consistent with tuple variant behavior
- ✅ Keeps existing data structure (HashMap works fine now)
- ✅ More informative for users (see all variants that support a field)

## Implementation Changes

### 1. Refactor `process_signature_groups` (enum_path_builder.rs:396-461)

Split into two paths:
- Tuple/Unit variants: keep current signature-level grouping
- Struct variants: new field-level grouping

### 2. Add `find_variants_with_field` helper

Cross-references all variant groups to find field compatibility.

### 3. Update `process_signature_path` signature (lines 224-280)

May need access to all variant groups, not just current signature's variants.

### 4. Handle deduplication

If multiple signatures have the same field, ensure we only create ONE path entry:
- Use HashMap/HashSet to track processed fields
- Key: `(field_name, field_type)` tuple

### 5. Fix instruction text

Change from:
```
"See 'applicable_variants' for which variants support this field."
```

To:
```
"See 'applicable_variants' for which variants support the '{path}' mutation path."
```

Where `{path}` is the actual mutation path like `.0.alpha`.

**Location:** Likely in `mutation_path_internal.rs` where `enum_instructions` is generated.

## Files to Modify

1. **mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs**
   - Refactor `process_signature_groups` (lines 396-461) to dispatch to field-level processing for struct variants
   - Add `process_struct_fields_by_compatibility` function with signature:
     ```rust
     fn process_struct_fields_by_compatibility(
         fields: &[(StructFieldName, BrpTypeName)],
         variants_with_this_signature: &[VariantName],
         all_variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
         ctx: &RecursionContext,
     )
     ```
   - Add `find_variants_with_field` helper function (signature already shown in plan)
   - Pass `variant_groups` as explicit parameter through the call chain (not via RecursionContext)

2. **mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs**
   - Update `enum_instructions` text generation to include actual path
   - Lines around where `enum_instructions` is populated in `into_mutation_path_external`

## Edge Cases

### 1. Field Name Collision with Different Types

```rust
enum WeirdEnum {
    VariantA { value: f32 },
    VariantB { value: String }, // Same field name, different type!
}
```

**Solution:** Group by `(field_name, field_type)` tuple, not just field name.
- `.0.value` for f32 → `applicable_variants: ["VariantA"]`
- `.0.value` for String → Wait, this still collides in HashMap!

**Resolution:** BRP mutation paths already include type information in the path construction. Check `path_kind_to_segment` (recursion_context.rs:172-179) to see how types are encoded.

If types differ, the **mutation path strings should differ** (BRP protocol requirement). If they don't, this is a BRP protocol limitation, not our bug.

### 2. Nested Struct Fields

```rust
enum ComplexEnum {
    VariantA { nested: Transform },
    VariantB { nested: Transform },
}
```

Field-level grouping would give:
- `.0.nested` → `applicable_variants: ["VariantA", "VariantB"]`
- `.0.nested.translation.x` → `applicable_variants: ["VariantA", "VariantB"]`

This is CORRECT behavior - both variants support these nested paths.

## Testing Strategy

### 1. Existing Test: Tuple Variants
Verify that `BottomEnum` with `VariantA(u32)` and `VariantD(u32)` still consolidates correctly.

### 2. New Test: Struct Variants with Shared Fields

Add test enum:
```rust
#[derive(Component, Reflect)]
#[reflect(Component)]
enum TestSharedFieldEnum {
    Alpha { x: f32, y: f32, alpha: f32 },
    Beta { z: f32, alpha: f32 },
    Gamma { alpha: f32 },
}
```

Expected behavior:
- `.0.alpha` → ONE entry with `applicable_variants: ["Alpha", "Beta", "Gamma"]`
- `.0.x` → ONE entry with `applicable_variants: ["Alpha"]`
- `.0.y` → ONE entry with `applicable_variants: ["Alpha"]`
- `.0.z` → ONE entry with `applicable_variants: ["Beta"]`

### 3. Color Enum Test

Test with actual `bevy::color::Color` enum:
- Verify `.0.alpha` has all 10 variants listed
- Verify no HashMap collision (all 10 variants appear in type guide)
- Verify root_example is valid for spawning

## Migration Notes

**Breaking Change:** No

This change is purely additive - it fills in missing `applicable_variants` data that was previously lost to HashMap collision. The structure of the output remains the same.

**User Impact:** Positive

Users will see:
- More accurate `applicable_variants` lists showing field compatibility
- No more silent data loss for struct variants with shared fields
- Better documentation of which variants support which mutation paths

## Alternative: Why Not Just Use Vec?

The `root-example-fix.md` plan proposes switching from `HashMap<String, MutationPathExternal>` to `Vec<MutationPathExternal>`.

**Downsides of Vec approach:**
- Keeps duplicate paths (multiple `.0.alpha` entries, one per variant)
- Doesn't leverage consolidation like tuple variants do
- Less informative (can't see which variants share a field)
- Wastes space (10 identical `.0.alpha` entries instead of 1 consolidated entry)

**Field-level grouping is conceptually superior:**
- Consolidates like tuple variants do
- Shows field compatibility explicitly
- No data structure change needed
- More informative for users and AI agents

## Open Questions

1. **Signature vs Field Grouping Performance:** Does cross-referencing all variant groups for each field cause performance issues with large enums?

2. **root_example Selection:** When multiple variants support a field, which variant's example should be used for `root_example`?
   - Current logic: first variant in the group
   - Proposed: same logic, but now group contains all field-compatible variants

3. **Should we ONLY use field-level grouping for structs, or hybrid?**
   - Option A: Always use field-level (proposed above)
   - Option B: Use signature-level first, then merge paths with same field name
   - Recommendation: Option A for consistency with tuple behavior

## Summary

**Current state:**
- Tuple variants: signature-level consolidation ✅ works
- Struct variants: signature-level only ❌ causes collision

**Proposed change:**
- Tuple variants: signature-level consolidation (no change)
- Struct variants: field-level consolidation (NEW)

**Result:**
- No HashMap collision
- Consistent consolidation behavior across variant types
- Better information for users
- No breaking changes
