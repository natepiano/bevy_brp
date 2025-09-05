# Plan: Enable Enum Recursion for Nested Mutation Paths

**CRITICAL PATH FORMAT NOTE**: This document has been updated to reflect the CORRECT tuple-based enum variant access pattern (`.custom_size.0.x`) that works in BRP. The `old_sprite.json` file showing direct field access (`.custom_size.x`) is INCORRECT and will fail. This was confirmed through testing on BRP port 15702.

**SCOPE CLARIFICATION**: While the identified bugs are Option<T>-specific, this implementation addresses ALL enum types in Bevy comprehensively. Bevy contains numerous enum types (Result<T,E>, Handle<T>, Color variants, custom game enums, etc.) that will benefit from nested mutation path generation. We are implementing a GENERIC enum recursion solution, not an Option-only fix, to handle all current and future enum scenarios uniformly.

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
".custom_size.0.x": {
  "description": "Mutate the x component of custom_size through Some variant (type: f32)",
  "example": { "none": null, "some": 1.0 },
  "path_kind": "IndexedElement",
  "type": "f32"
},
".custom_size.0.y": {
  "description": "Mutate the y component of custom_size through Some variant (type: f32)",
  "example": { "none": null, "some": 2.0 },
  "path_kind": "IndexedElement", 
  "type": "f32"
}
```

**CRITICAL**: The above format (`.custom_size.0.x`) is the **CORRECT** format that actually works in BRP. The `old_sprite.json` file showing `.custom_size.x` format is **INCORRECT** and does not work with current BRP implementation. Testing confirmed that `.custom_size.x` fails while `.custom_size.0.x` succeeds.

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
    MutationPath { path: ".custom_size.0.x", type: "f32" },  // Through Some(Vec2) tuple variant
    MutationPath { path: ".custom_size.0.y", type: "f32" }   // Through Some(Vec2) tuple variant
]
```

**NOTE**: The `.custom_size.0.x` format is the CORRECT tuple-based enum variant access pattern that works in BRP, not the `.custom_size.x` format shown in `old_sprite.json`.

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
- **EnumMutationBuilder**: Should iterate through enum variant signatures → recurse into each signature's inner types

### Signature Deduplication Approach

**Critical Design Decision**: Process each unique variant signature exactly once, not every variant.

**Variant Signatures**:
- **Unit variants**: All unit variants share one signature (e.g., `None`, `Empty`, `Active` → process ONE)
- **Tuple variants**: Signature is the list of types (e.g., `Some(f32)` and `Ok(f32)` have same signature `[f32]` → process ONE)
- **Struct variants**: Signature is field names and types (e.g., different field structures → process each unique one)

**Example**: For an enum with `None`, `Empty`, `Some(f32)`, `Ok(f32)`, `Err(String)`:
- Process ONE unit variant (first encountered)
- Process ONE tuple `[f32]` variant (first encountered)
- Process ONE tuple `[String]` variant

This approach:
- Eliminates redundant processing
- Generates all necessary paths
- Maintains efficiency

### The Generic Approach

Instead of special-casing Option, make ALL enums with data variants generate nested paths:

- **Option<Vec2>**: Some(Vec2) → recurse into Vec2 → generate `.field.0.x`, `.field.0.y`
- **Result<String, Error>**: Ok(String) → recurse into String, Err(Error) → recurse into Error  
- **Handle<Mesh>**: Strong(AssetId<Mesh>) → recurse into AssetId → generate nested paths
- **Custom enums**: Any Tuple/Struct variant → recurse into inner types

### Implementation Strategy

#### Step 1: Replace build_enum_example to Return All Variant Examples

**CRITICAL**: We are NOT implementing new deduplication logic. We are REPLACING the current `build_enum_example` (which only returns the first variant) to call the EXISTING `build_all_enum_examples` function that already has signature deduplication.

```rust
impl EnumMutationBuilder {
    /// Build example value for an enum type
    /// CHANGED: Now returns ALL variant examples instead of just the first one
    /// by calling the existing build_all_enum_examples function
    pub fn build_enum_example(
        schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: Option<&BrpTypeName>,
        depth: RecursionDepth,
    ) -> Value {
        // Check for exact enum type knowledge first
        if let Some(enum_type) = enum_type
            && let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(enum_type.type_string()))
        {
            return knowledge.example_value().clone();
        }

        // CRITICAL: Reuse EXISTING build_all_enum_examples function
        // DO NOT reimplement the deduplication logic - it already exists!
        let all_examples = build_all_enum_examples(
            schema, 
            registry, 
            depth.current_depth(), 
            enum_type
        );
        
        // Return all variant examples as JSON
        if all_examples.is_empty() {
            json!(null)
        } else {
            json!(all_examples)
        }
    }
}

#### Step 2: Extract Unique Signature Inner Types for Recursion

Add method to `EnumVariantInfo` for extracting inner types, then implement signature deduplication:

```rust
impl EnumVariantInfo {
    /// Extract inner types and their access methods from this variant
    /// Returns empty vector for unit variants, tuple indices for tuple variants,
    /// and field names for struct variants
    pub fn inner_types(&self) -> Vec<(BrpTypeName, VariantAccess)> {
        match self {
            Self::Unit(_) => Vec::new(),
            Self::Tuple(_, types) => {
                types.iter()
                    .enumerate()
                    .map(|(index, type_name)| {
                        (type_name.clone(), VariantAccess::TupleIndex(index))
                    })
                    .collect()
            }
            Self::Struct(_, fields) => {
                fields.iter()
                    .map(|field| {
                        (field.type_name.clone(), 
                         VariantAccess::StructField(field.field_name.clone()))
                    })
                    .collect()
            }
        }
    }
    
    /// Get the signature of this variant for deduplication
    /// Unit variants return None, tuple variants return type list,
    /// struct variants return field name/type pairs
    pub fn signature(&self) -> VariantSignature {
        match self {
            Self::Unit(_) => VariantSignature::Unit,
            Self::Tuple(_, types) => VariantSignature::Tuple(types.clone()),
            Self::Struct(_, fields) => {
                let field_sig = fields.iter()
                    .map(|f| (f.field_name.clone(), f.type_name.clone()))
                    .collect();
                VariantSignature::Struct(field_sig)
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum VariantSignature {
    Unit,
    Tuple(Vec<BrpTypeName>),
    Struct(Vec<(String, BrpTypeName)>),
}

/// Deduplicate variants by signature, returning first variant of each unique signature
fn deduplicate_variant_signatures(variants: Vec<EnumVariantInfo>) -> Vec<EnumVariantInfo> {
    let mut seen_signatures = HashSet::new();
    let mut unique_variants = Vec::new();
    
    for variant in variants {
        let signature = variant.signature();
        if seen_signatures.insert(signature) {
            unique_variants.push(variant);
        }
    }
    
    unique_variants
}

#[derive(Debug, Clone)]
enum VariantAccess {
    TupleIndex(usize),        // For .0, .1, .2... paths
    StructField(String),      // For .field_name paths
}

impl VariantAccess {
    fn to_field_name(&self) -> String {
        match self {
            Self::TupleIndex(idx) => idx.to_string(),
            Self::StructField(name) => name.clone(),
        }
    }
}
```

#### Step 3: Make EnumMutationBuilder Recurse With Signature Deduplication

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

        // Check depth limit first (like StructMutationBuilder does)
        if depth.exceeds_limit() {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
            )]);
        }

        let mut paths = Vec::new();
        
        // Step 1: Add the base enum path with ALL signature examples
        let enum_variants = Self::extract_enum_variants(schema);
        let enum_example = Self::build_enum_example(
            schema, 
            &ctx.registry, 
            Some(ctx.type_name()), 
            depth  // No increment here - just pass current depth
        );

        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path: String::new(),
                    example: enum_example,  // Now contains all unique signature examples
                    enum_variants,
                    type_name: type_name.clone(),
                    path_kind: MutationPathKind::RootValue { type_name: type_name.clone() },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
            RootOrField::Field { field_name, field_type, parent_type } => {
                paths.push(MutationPathInternal {
                    path: format!(".{field_name}"),
                    example: MutationPathContext::wrap_example(enum_example),
                    enum_variants,
                    type_name: field_type.clone(),
                    path_kind: MutationPathKind::StructField { 
                        field_name: field_name.clone(), 
                        parent_type: parent_type.clone() 
                    },
                    mutation_status: MutationStatus::Mutatable,
                    error_reason: None,
                });
            }
        }

        // Step 2: Recurse into unique signature inner types
        let variants = extract_enum_variants(schema, &ctx.registry, depth.current_depth());
        let unique_variants = deduplicate_variant_signatures(variants);
        
        for variant in unique_variants {
            for (type_name, variant_access) in variant.inner_types() {
                // Get the schema for the inner type
                let Some(inner_schema) = ctx.get_type_schema(&type_name) else {
                    continue; // Skip if we can't find the schema
                };
                
                let inner_kind = TypeKind::from_schema(inner_schema, &type_name);
                
                // Create field context for recursion using existing infrastructure
                let field_name = variant_access.to_field_name();
                let variant_ctx = ctx.create_field_context(&field_name, &type_name);
                
                // Recurse with current depth (TypeKind::build_paths will increment if needed)
                let nested_paths = inner_kind.build_paths(&variant_ctx, depth)?;
                paths.extend(nested_paths);
            }
        }

        Ok(paths)
    }
}
```

**Key Benefits of This Approach**:

1. **Signature Deduplication**: Each unique variant signature is processed exactly once
   - For `Result<String, String>`: Process String type once, not twice
   - For enums with multiple unit variants: Process one unit example

2. **Complete Examples**: The refactored `build_enum_example` returns all unique signature examples
   ```json
   {
     "None": null,           // Unit variant example
     "Some": [1.0, 2.0],     // Tuple variant example
     "WithStruct": {...}     // Struct variant example
   }
   ```

3. **Efficient Recursion**: Only recurse into types from unique signatures
   - Eliminates O(n²) behavior for enums with duplicate types
   - Maintains all necessary paths without redundant processing

4. **Depth Safety**: The existing `RecursionDepth` system prevents infinite recursion
   - Deeply nested enums like `Option<Option<Option<Vec2>>>` are handled safely
   - TypeKind already manages depth increments properly

## Testing Strategy

### Validation Tests for Each Bug

#### Test Bug 1 Fix: Nested Paths for Option<Vec2>
```bash
# Should see .custom_size.0.x and .custom_size.0.y paths
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths | keys[] | select(startswith(".custom_size"))'
# Expected: [".custom_size", ".custom_size.0.x", ".custom_size.0.y"]
```

**CRITICAL**: The expected paths use `.custom_size.0.x` format (tuple variant access), NOT `.custom_size.x` as shown in `old_sprite.json`. The latter format is incorrect and will fail in BRP.

#### Test Bug 2 Fix: All Signature Examples
```bash
# Should see examples for all unique variant signatures
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths.".custom_size".example'
# Expected: {"None": null, "Some": [1.0, 2.0]}
```

#### Test Bug 3 Fix: Option<Rect> Structure
```bash
# Should see min/max in the Some variant example
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths.".rect".example.Some'
# Expected: {"min": [0.0, 0.0], "max": [100.0, 100.0]}
```

#### Test Bug 4 Fix: TextureAtlas Format
```bash
# Should see enum_variants field
brp_type_schema types=["bevy_sprite::sprite::Sprite"] | jq '.result.type_info."bevy_sprite::sprite::Sprite".mutation_paths.".texture_atlas"'
# Expected: has "enum_variants": ["None", "Some"] and example with both variants
```

### Testing Signature Deduplication

#### Test Enum with Multiple Same Signatures
```bash
# Create a test enum with multiple variants of same signature
# enum TestEnum { First(f32), Second(f32), Third(String) }
# Should generate examples for only TWO tuple signatures:
# - One for [f32] (either First or Second, whichever is first)
# - One for [String] (Third)
brp_type_schema types=["test_app::TestEnum"] | jq '.result.type_info."test_app::TestEnum".mutation_paths."".example'
# Expected: Only 2 examples, not 3
```

### Regression Tests
1. **Test regular enums**: Ensure Color enum still works correctly
2. **Test Handle types**: Should generate paths through Strong/Weak variants if we extend beyond Option
3. **Test other Option types**: Option<String>, Option<bool>, etc.

## Success Criteria

- ✅ Option<Vec2> generates `.0.x` and `.0.y` nested paths (tuple variant access format)
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

## Complete Implementation Design

### Variant Types and Their Handling

With signature deduplication, each unique variant signature is processed exactly once:

#### 1. Unit Variants
**Examples**: `None`, `Empty`, `Active`  
**Signature**: All unit variants share one signature
**Processing**: Process ONE unit variant (whichever is encountered first)
**Path Generation**: No recursion needed - only root path

#### 2. Tuple Variants  
**Examples**: 
- `Some(Vec2)` and `Ok(Vec2)` → Same signature `[Vec2]`
- `Err(String)` → Different signature `[String]`

**Signature**: List of inner types `[T1, T2, ...]`
**Processing**: Process first variant of each unique signature
**Path Generation**: Recurse into `.0`, `.1`, etc. for inner types
**Confirmed Working**: `.custom_size.0.x` tested on BRP port 15702

#### 3. Struct Variants
**Examples**: `WithStruct { position: Vec3, scale: f32 }`
**Signature**: Field names and types `[(name1, T1), (name2, T2), ...]`
**Processing**: Process first variant of each unique field structure
**Path Generation**: Recurse into `.field_name` for each field

#### 4. Nested Enums
**Examples**: `Option<Option<Vec2>>`
**Processing**: Each level processes its unique signatures
**Path Generation**: Natural recursion produces `.0.0.x`, `.0.0.y`

### Expected Path Generation Examples After Implementation

#### Option<Vec2> in Sprite
**Current**: Only `.custom_size` with single example
**After Fix**: 
```json
{
  ".custom_size": {
    "example": {
      "None": null,
      "Some": [1.0, 2.0]
    },
    "enum_variants": ["None", "Some"]
  },
  ".custom_size.0.x": {
    "description": "Mutate x component through Option<Vec2> Some variant",
    "example": 1.0,
    "type": "f32"
  },
  ".custom_size.0.y": {
    "description": "Mutate y component through Option<Vec2> Some variant",
    "example": 2.0,
    "type": "f32"
  }
}

## Architecture Decision

**Chosen Approach: Generic Enum Recursion**

Make `EnumMutationBuilder` work exactly like `StructMutationBuilder` by recursing into enum variant inner types. This is the most consistent and elegant solution.

**Why This Approach**:
- **Consistency**: Enums and structs now work the same way
- **Generality**: Fixes Option, Result, Handle, and any other enum automatically  
- **Maintainability**: No special cases or string matching needed
- **Future-proof**: Any new enum types will automatically get nested paths

**Potential Concerns Addressed**:
- **Path explosion**: Mitigated by existing depth limits and type deduplication
- **Performance**: Minimal overhead - same recursion pattern as structs
- **Backwards compatibility**: Only adds paths, doesn't remove existing ones

**Implementation Priority**: Single-pass implementation of generic recursion rather than incremental Option-only fixes.

## TYPE-SYSTEM-6: Function Should Be Method - extract_variant_inner_types Misplaced ✅
- **Category**: TYPE-SYSTEM
- **Status**: APPROVED - To be implemented
- **Location**: Section: Extract Unique Signature Inner Types for Recursion
- **Issue Identified**: The plan originally proposed extract_variant_inner_types as a standalone function in EnumMutationBuilder when this behavior belongs on the EnumVariantInfo type itself. This violates the principle that functions operating on type data should be methods on that type.
- **Verdict**: CONFIRMED
- **Reasoning**: Functions that operate on type data should be methods on that type for better encapsulation and following object-oriented design principles.

### Approved Change:
The plan has been updated to make `inner_types()` a method on `EnumVariantInfo` instead of a standalone function in `EnumMutationBuilder`. Additionally, a `signature()` method was added to `EnumVariantInfo` to support signature deduplication, and a separate `deduplicate_variant_signatures()` function handles the deduplication logic.

### Implementation Notes:
- `EnumVariantInfo::inner_types()` returns the types and access methods for recursion
- `EnumVariantInfo::signature()` returns the variant's signature for deduplication
- `deduplicate_variant_signatures()` operates on collections of variants to eliminate duplicates
- This follows the principle that types should own their behavior

## DUPLICATION-2: Example Building Logic Creates Competing Implementations ✅
- **Category**: DUPLICATION
- **Status**: APPROVED - To be implemented
- **Location**: Section: Replace build_enum_example to Return All Variant Examples
- **Issue Identified**: The original plan proposed reimplementing signature deduplication logic that already exists in build_all_enum_examples. This would create two different code paths for the same functionality.
- **Verdict**: CONFIRMED
- **Reasoning**: Code duplication violates DRY principles and creates maintenance burden. The existing build_all_enum_examples function already contains the exact signature deduplication logic needed.

### Approved Change:
Replace the current `build_enum_example` implementation to call the existing `build_all_enum_examples` function instead of reimplementing its logic. The plan has been updated to make this crystal clear with "CRITICAL" notes.

### Implementation Notes:
- DO NOT reimplement deduplication logic - it already exists in `build_all_enum_examples`
- Simply call the existing function and return its result as JSON
- This eliminates ~30 lines of duplicate code and ensures consistent behavior

## Design Review Skip Notes

### DUPLICATION-1: Path Construction Duplicates Existing Type-Safe Infrastructure
- **Status**: SKIPPED
- **Category**: DUPLICATION
- **Location**: Section: Make EnumMutationBuilder Recurse With Signature Deduplication
- **Issue**: The plan proposes manual field context creation that duplicates the existing type-safe path building infrastructure. The existing create_field_context method already handles path construction properly.
- **Proposed Change**: Use existing create_field_context infrastructure consistently
- **Verdict**: REJECTED
- **Reasoning**: This finding is a false positive. The plan's proposed code actually DOES use the existing create_field_context method properly (ctx.create_field_context(&field_name, &type_name)). There is no duplication in the planned implementation.
- **Decision**: User elected to skip this recommendation

## Design Review Skip Notes

### DESIGN-2: Inconsistent Recursion Pattern Deviates from Established Architecture
- **Status**: SKIPPED
- **Category**: DESIGN
- **Location**: Section: Make EnumMutationBuilder Recurse With Signature Deduplication
- **Issue**: The plan's recursion approach deviates from the established pattern used by StructMutationBuilder. The struct builder uses a clear pattern: extract properties -> create field context -> recurse via field_kind.build_paths(). The enum approach should follow the same pattern.
- **Proposed Change**: Follow the same recursion pattern as StructMutationBuilder
- **Verdict**: REJECTED
- **Reasoning**: The finding is based on a fundamental misunderstanding of BRP enum mutation semantics. Enums in BRP are replaced as whole units, not mutated field-by-field like structs. However, for generating nested PATHS (not mutations), the plan's approach to recurse into enum variants is correct and necessary to enable paths like `.custom_size.0.x` for Option<Vec2>.
- **Decision**: User elected to skip this recommendation

### TYPE-SYSTEM-2: Inconsistent Path Format Strategy
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: Section: Expected Path Generation Examples
- **Issue**: The plan shows inconsistent path formats. The expected example shows .custom_size.x and .custom_size.y, but earlier tuple variant examples show .custom_size.0.x format. The old_sprite.json confirms the direct .field.component format is expected, not tuple-indexed paths.
- **Proposed Change**: Clarify whether Option<T> should generate direct field access (.field.x) or tuple-indexed access (.field.0.x) and ensure consistency throughout.
- **Verdict**: MODIFIED
- **Reasoning**: Testing on running BRP (port 15702) confirmed that `.custom_size.0.x` and `.custom_size.0.y` work correctly, while `.custom_size.x` fails with "Expected variant field access to access a Struct variant, found a Tuple variant instead." The current BRP implementation uses tuple format for Option<T> access.
- **Decision**: The testing shows the plan's tuple format is actually correct; the inconsistency should be resolved by updating examples to match the working implementation

## TYPE-SYSTEM-5: String-Based Path Construction Over Type-Safe Approach ✅
- **Category**: TYPE-SYSTEM
- **Status**: APPROVED - To be implemented
- **Location**: Section: Path context creation logic
- **Issue Identified**: The plan uses manual string concatenation for path building (`format!("{parent_field}.{field_name}")`) instead of leveraging the existing type-safe path building in `create_field_context`. This violates the string typing principle by treating paths as primitive strings rather than structured data.
- **Verdict**: CONFIRMED
- **Reasoning**: The finding accurately identifies redundant path construction logic. Multiple methods duplicate the same string building pattern that already exists in `create_field_context`. This violates the DRY principle and bypasses existing type-safe infrastructure. The duplication creates maintenance burden and potential for inconsistent path building behavior.

### Approved Change:
Instead of manual string concatenation in the enum recursion implementation, use the existing `create_field_context` method for type-safe path construction:

```rust
// AVOID: Manual string building
let struct_field_path = match &ctx.location {
    RootOrField::Root { .. } => field_name.clone(),
    RootOrField::Field { field_name: parent_field, .. } => {
        format!("{parent_field}.{field_name}")  // Manual string concat - ERROR PRONE
    }
};

// PREFERRED: Use existing type-safe infrastructure
let nested_ctx = ctx.create_field_context(&field_name, &inner_type.type_name());
// The nested_ctx already contains the correctly computed path_prefix
```

### Implementation Notes:
- All path construction in the enum recursion implementation must use `create_field_context`
- Remove any manual `format!()` calls for path building
- Leverage the existing `path_prefix` field in `MutationPathContext` rather than reconstructing paths
- This applies to both tuple variant access (`.0`, `.1`) and struct variant field access (`.field_name`)

### ⚠️ PREJUDICE WARNING - DESIGN-1: Over-Engineering - Generic Enum Recursion May Be Too Broad
- **Status**: PERMANENTLY REJECTED
- **Category**: DESIGN
- **Location**: Section: Solution - Make Enums Recurse Like Structs
- **Issue**: The plan proposes a generic solution for ALL enum types when the problem is specifically with Option<T> types. The examples show recursion for Result<String, Error>, Handle<Mesh>, and custom enums, but the bugs identified are exclusively about Option types.
- **Verdict**: REJECTED
- **Critical Note**: This suggestion to limit the solution to Option<T> only is ABSOLUTELY REJECTED. Bevy contains numerous enum types (Result<T,E>, Handle<T>, Color variants, custom game enums, etc.) that ALL require nested mutation path generation. We are implementing a GENERIC enum recursion solution to handle ALL current and future enum scenarios uniformly, not just Option<T>. Option<T> is merely one of many enum types that need this functionality.
- **DO NOT SUGGEST THIS AGAIN**: Any future recommendations to narrow the scope to Option-only solutions will be permanently rejected

