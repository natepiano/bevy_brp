# Plan: Enable Enum Recursion for Nested Mutation Paths

## Problem Statement

After removing wrapper_types.rs, enum types (including Option) no longer generate nested mutation paths. The `EnumMutationBuilder` treats enums as atomic units that can only be replaced entirely, unlike `StructMutationBuilder` which recurses into struct fields to generate nested paths.

### Specific Bugs from Sprite Mutation Paths

#### Bug 1: Missing Nested Mutation Paths for Option<Vec2> ❌
**Current**: Option<Vec2> fields only generate a single path:
```json
".custom_size": {
  "example": "None",
  "type": "core::option::Option<glam::Vec2>"
}
```

**Expected**: Should generate nested component paths:
```json
".custom_size": { /* base field */ },
".custom_size.x": {
  "description": "Mutate the x component of custom_size (type: f32)",
  "example": { "none": null, "some": 1.0 },
  "path_kind": "NestedPath",
  "type": "f32"
},
".custom_size.y": {
  "description": "Mutate the y component of custom_size (type: f32)",
  "example": { "none": null, "some": 2.0 },
  "path_kind": "NestedPath", 
  "type": "f32"
}
```

#### Bug 2: Option Fields Show String "None" Instead of Dual Examples ❌
**Current**: All Option fields show:
```json
"example": "None"  // Just a string!
```

**Expected**: Should show dual-format examples:
```json
"example_none": null,
"example_some": [1.0, 2.0],  // Or appropriate structure for the inner type
"note": "For Option fields: pass the value directly to set Some, null to set None"
```

#### Bug 3: Option<Rect> Doesn't Show Inner Structure ❌
**Current**: 
```json
".rect": {
  "example": "None"
}
```

**Expected**:
```json
".rect": {
  "example_none": null,
  "example_some": {
    "max": [100.0, 100.0],
    "min": [0.0, 0.0]
  },
  "note": "For Option fields: pass the value directly to set Some, null to set None"
}
```

#### Bug 4: Option<TextureAtlas> Shows Wrong Example Format ❌
**Current**:
```json
".texture_atlas": {
  "example": "None"
}
```

**Expected** (from old_sprite.json):
```json
".texture_atlas": {
  "enum_variants": ["None", "Some"],
  "example_none": null,
  "example_some": "None",  // Shows the enum variant name for complex types
  "note": "For Option fields: pass the value directly to set Some, null to set None"
}
```

### Root Cause Summary

1. **Missing Nested Paths**: EnumMutationBuilder doesn't recurse into variants
2. **Wrong Example Format**: Option fields need dual examples, not single enum variant strings
3. **Lost Inner Structure**: Option<Complex> types don't show the structure of their inner types
4. **Missing Metadata**: Complex Option types need enum_variants field

### Current Behavior vs Expected

**Current**: EnumMutationBuilder generates only one path:
```rust
// For Option<Vec2> field named "custom_size"
paths = vec![
    MutationPath { path: ".custom_size", type: "core::option::Option<glam::Vec2>" }
]
```

**Expected**: Should generate paths through variants:
```rust
paths = vec![
    MutationPath { path: ".custom_size", type: "core::option::Option<glam::Vec2>" },
    MutationPath { path: ".custom_size.x", type: "f32" },  // Through Some variant
    MutationPath { path: ".custom_size.y", type: "f32" }   // Through Some variant
]
```

## Root Cause Analysis

The `EnumMutationBuilder::build_paths` method (line 518-566 in mutation_path_builders.rs) only creates a single mutation path for the entire enum. It doesn't recurse into enum variants like `StructMutationBuilder` recurses into struct fields.

Key differences:
- **StructMutationBuilder**: Iterates through fields, recurses into each field's type
- **EnumMutationBuilder**: Builds example but doesn't recurse for paths

## Solution: Make Enums Recurse Like Structs

### Core Design Principle

**EnumMutationBuilder should work exactly like StructMutationBuilder** - iterate through its "children" and recurse into their types to generate nested paths.

**Analogy**:
- **StructMutationBuilder**: Iterates through struct fields → recurses into each field's type
- **EnumMutationBuilder**: Should iterate through enum variants → recurse into each variant's inner types

### The Generic Approach

Instead of special-casing Option, make ALL enums with data variants generate nested paths:

- **Option<Vec2>**: Some(Vec2) → recurse into Vec2 → generate `.field.x`, `.field.y`
- **Result<String, Error>**: Ok(String) → recurse into String, Err(Error) → recurse into Error  
- **Handle<Mesh>**: Strong(AssetId<Mesh>) → recurse into AssetId → generate nested paths
- **Custom enums**: Any Tuple/Struct variant → recurse into inner types

### Implementation Strategy

#### Step 1: Extract All Variant Inner Types

Make EnumMutationBuilder iterate through variants like StructMutationBuilder iterates through fields:

```rust
impl EnumMutationBuilder {
    /// Extract inner types from enum variants for recursion
    /// This is the enum equivalent of StructMutationBuilder's field iteration
    fn extract_variant_inner_types(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Vec<BrpTypeName> {
        let mut inner_types = Vec::new();
        
        let variants = extract_enum_variants(schema, registry, 0);
        
        for variant in variants {
            match variant {
                EnumVariantInfo::Unit(_) => {
                    // Unit variants have no inner types to recurse into
                }
                EnumVariantInfo::Tuple(_name, types) => {
                    // Tuple variants: recurse into each inner type
                    inner_types.extend(types);
                }
                EnumVariantInfo::Struct(_name, fields) => {
                    // Struct variants: recurse into each field type
                    for field in fields {
                        inner_types.push(field.type_name);
                    }
                }
            }
        }
        
        // Remove duplicates (multiple variants might have same inner type)
        inner_types.sort();
        inner_types.dedup();
        inner_types
    }
}
```

#### Step 2: Make EnumMutationBuilder Recurse Like StructMutationBuilder

**Current StructMutationBuilder pattern**:
```rust
// StructMutationBuilder iterates through fields
for (field_name, field_info) in properties {
    let field_type = extract_field_type(field_info);
    let field_ctx = ctx.create_field_context(&field_name, &field_type);
    
    // Recurse into the field's type  
    let field_paths = field_kind.build_paths(&field_ctx, depth)?;
    paths.extend(field_paths);
}
```

**New EnumMutationBuilder pattern** (mirror the struct approach):
```rust
impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        };

        let mut paths = Vec::new();
        
        // Step 1: Add the base enum path (existing logic)
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(schema, &ctx.registry, Some(ctx.type_name()), depth.increment());

        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal { /* base enum path */ });
            }
            RootOrField::Field { field_name, field_type, parent_type } => {
                paths.push(MutationPathInternal { /* base enum field path */ });
            }
        }

        // Step 2: NEW - Recurse into variant inner types (like struct fields)
        let variant_inner_types = Self::extract_variant_inner_types(schema, &ctx.registry);
        
        for inner_type in variant_inner_types {
            // Get the schema for the inner type
            let Some(inner_schema) = ctx.get_type_schema(&inner_type) else {
                continue; // Skip if we can't find the schema
            };
            
            let inner_kind = TypeKind::from_schema(inner_schema, &inner_type);
            
            // Create field context for recursion (treat variant inner type like a field)
            let current_field_name = match &ctx.location {
                RootOrField::Root { .. } => "", // Root level
                RootOrField::Field { field_name, .. } => field_name,
            };
            let variant_ctx = ctx.create_field_context(current_field_name, &inner_type);
            
            // Recurse just like StructMutationBuilder does
            let nested_paths = inner_kind.build_paths(&variant_ctx, depth)?;
            paths.extend(nested_paths);
        }

        Ok(paths)
    }
}
```

**Key Insight**: This approach works for ANY enum:
- **Option<Vec2>**: Some(Vec2) → extracts Vec2 → Vec2 generates `.x`, `.y` paths
- **Result<Transform, Error>**: Ok(Transform) → extracts Transform → Transform generates `.translation.x`, etc.
- **Handle<Mesh>**: Strong(AssetId) → extracts AssetId → AssetId generates its paths

#### Step 3: Special Handling for Option Paths

Since Option is special (None has no data, Some has data), we need special handling:

```rust
fn is_option_type(type_name: &BrpTypeName) -> bool {
    type_name.as_str().starts_with("core::option::Option<")
}

fn extract_option_inner_type(type_name: &str) -> Option<String> {
    if type_name.starts_with("core::option::Option<") && type_name.ends_with(">") {
        let inner = &type_name[22..type_name.len()-1];
        Some(inner.to_string())
    } else {
        None
    }
}

// When generating nested paths through Option, wrap examples appropriately
fn wrap_option_example(inner_example: Value) -> Value {
    json!({
        "none": null,
        "some": inner_example
    })
}
```

#### Step 4: Handle Complex Cases

For struct types that have known component paths (like Vec2, Vec3), we need to ensure the recursion generates the right paths:

```rust
// In the recursion logic, check for types with known components
if inner_type.as_str() == "glam::Vec2" {
    // Vec2 should generate .x and .y paths
    // The StructMutationBuilder will handle this when we recurse
}
```

## Alternative Approach: Targeted Option Handling

If full enum recursion is too complex or has unwanted side effects, we could special-case Option:

```rust
impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(...) -> Result<Vec<MutationPathInternal>> {
        // ... existing code ...
        
        // Special handling for Option types
        if ctx.type_name().as_str().starts_with("core::option::Option<") {
            if let Some(inner_type_str) = extract_option_inner_type(ctx.type_name().as_str()) {
                // Generate nested paths specifically for Option
                let nested = generate_option_nested_paths(ctx, &inner_type_str, &ctx.registry);
                paths.extend(nested);
            }
        }
        
        Ok(paths)
    }
}

fn generate_option_nested_paths(
    ctx: &MutationPathContext,
    inner_type_str: &str,
    registry: &HashMap<BrpTypeName, Value>,
) -> Vec<MutationPathInternal> {
    // Generate paths based on known types
    match inner_type_str {
        "glam::Vec2" => {
            vec![
                create_option_component_path(ctx, "x", "f32"),
                create_option_component_path(ctx, "y", "f32"),
            ]
        }
        "glam::Vec3" => {
            vec![
                create_option_component_path(ctx, "x", "f32"),
                create_option_component_path(ctx, "y", "f32"),
                create_option_component_path(ctx, "z", "f32"),
            ]
        }
        _ => {
            // For other types, try to recurse generically
            if let Some(inner_type) = BrpTypeName::try_from(inner_type_str).ok() {
                // ... generic recursion ...
            }
            vec![]
        }
    }
}
```

## Testing Strategy

### Validation Tests for Each Bug

#### Test Bug 1 Fix: Nested Paths for Option<Vec2>
```bash
# Should see .custom_size.x and .custom_size.y paths
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths | keys[] | select(startswith(".custom_size"))'
# Expected: [".custom_size", ".custom_size.x", ".custom_size.y"]
```

#### Test Bug 2 Fix: Dual-Format Examples
```bash
# Should see example_none and example_some
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths.".custom_size"'
# Expected: {"example_none": null, "example_some": [1.0, 2.0], "note": "..."}
```

#### Test Bug 3 Fix: Option<Rect> Structure
```bash
# Should see min/max in example_some
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths.".rect".example_some'
# Expected: {"min": [0.0, 0.0], "max": [100.0, 100.0]}
```

#### Test Bug 4 Fix: TextureAtlas Format
```bash
# Should see enum_variants field
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths.".texture_atlas"'
# Expected: has "enum_variants": ["None", "Some"]
```

### Regression Tests
1. **Test regular enums**: Ensure Color enum still works correctly
2. **Test Handle types**: Should generate paths through Strong/Weak variants if we extend beyond Option
3. **Test other Option types**: Option<String>, Option<bool>, etc.

## Success Criteria

- ✅ Option<Vec2> generates `.x` and `.y` nested paths
- ✅ Other enums (like Color) continue to work as before
- ✅ Nested paths have appropriate examples
- ✅ No infinite recursion issues
- ✅ Performance remains acceptable

## Risk Assessment

**Risks**:
1. **Infinite recursion**: Need depth limits (already have RecursionDepth)
2. **Path explosion**: Many enum variants could generate too many paths
3. **Behavioral changes**: Other enums might get unexpected nested paths

**Mitigations**:
1. Use existing RecursionDepth system
2. Potentially limit recursion to specific enum types (Option, Result, etc.)
3. Test thoroughly with various enum types

## Implementation Order

1. Add `is_option_type` and `extract_option_inner_type` utilities
2. Modify `EnumMutationBuilder::build_paths` to detect Option and recurse
3. Test with Option<Vec2> to verify nested paths work
4. Extend to other Option types
5. Consider whether to extend to all enums or keep Option-specific

## Decision Point

**Option A: Full Enum Recursion**
- Pros: Consistent behavior, works for all enums with data variants
- Cons: More complex, could generate many paths for complex enums

**Option B: Option-Specific Recursion**
- Pros: Targeted fix, less risk of side effects
- Cons: Special-case code, might miss other enums that could benefit

**Recommendation**: Start with Option B (Option-specific) to fix the immediate regression, then consider extending to full enum recursion if needed.