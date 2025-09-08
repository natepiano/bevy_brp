# Mutation Path Output Structure Plan

## Overview
This plan defines a unified output structure for mutation paths that works consistently across all `PathKind` variants while properly representing enum variant groups by their type signatures.

## Core Structure

Every mutation path entry follows this structure:

```json
{
  ".path": {
    "description": "Human-readable description of what this path mutates",
    "mutation_status": "mutatable" | "not_mutatable" | "partially_mutatable",
    "variants": ["List", "Of", "Applicable", "Variants"],  // For enum paths only, shows which variants this path works with (renamed from enum_variants field in code)
    "path_info": {
      "path_kind": "PathKind variant",
      "type": "Fully qualified type name",
      "type_kind": "TypeKind variant (Enum, Struct, Value, Array, etc.)"
    },
    "examples": [
      {
        "variants": ["List", "Of", "Variants"],  // Only for enum types, groups variants by signature
        "signature": "Description of variant structure",  // Only for enum types, signature of this variant group
        "example": "Actual value to send"
      }
    ],
    "error_reason": "Optional error message for non-mutatable paths"
  }
}
```

### Field Naming Clarification

The `variants` field in the JSON output corresponds to the `enum_variants` field in the current Rust code. During implementation, either:
1. Keep the current code field name `enum_variants` and use serde rename: `#[serde(rename = "variants")]`
2. Rename the field in the code from `enum_variants` to `variants` to match the JSON output

The field uses `Option<Vec<String>>` with `#[serde(skip_serializing_if = "Option::is_none")]` to handle presence/absence cleanly through the type system.

## Key Design Decisions

1. **`variants` field at path level** - Only present for enum types, lists applicable variants
2. **For enum types**: 
   - Path-level `variants` lists ALL possible variants for the enum
   - Each example group has `variants` and `signature` fields
   - Example `variants` lists ONLY the variants that share that signature structure
   - This creates a filtered subset relationship: example.variants ⊆ path.variants
3. **For enum field paths**: `variants` lists which parent variants contain this field
4. **For non-enum types**: 
   - No `variants` field at path level
   - Single example in array with just `example` field (no variants/signature)
5. **Structure is consistent for all `PathKind` variants** - Works for RootValue, StructField, IndexedElement, and ArrayElement

## Examples by PathKind

### 1. RootValue (Enum)

```json
{
  "": {
    "description": "Replace the entire TestEnumWithSerDe value",
    "mutation_status": "mutatable",
    "variants": ["Active", "Inactive", "Special", "Custom"],
    "signature": "enum",
    "path_info": {
      "path_kind": "RootValue",
      "type": "extras_plugin::TestEnumWithSerDe",
      "type_kind": "Enum"
    },
    "examples": [
      {
        "variants": ["Active", "Inactive"],
        "signature": "unit",
        "example": "Active"
      },
      {
        "variants": ["Special"],
        "signature": "tuple(String, u32)",
        "example": {"Special": ["Hello, World!", 42]}
      },
      {
        "variants": ["Custom"],
        "signature": "struct{name: String, value: f32, enabled: bool}",
        "example": {"Custom": {"name": "test", "value": 3.14, "enabled": true}}
      }
    ]
  }
}
```

### 2. IndexedElement (Enum Field - Tuple Variant)

```json
{
  ".0": {
    "description": "Mutate element 0 of TestEnumWithSerDe",
    "mutation_status": "mutatable",
    "variants": ["Special"],
    "signature": "String",
    "path_info": {
      "path_kind": "IndexedElement",
      "type": "alloc::string::String",
      "type_kind": "Value"
    },
    "examples": [
      {
        "example": "Hello, World!"
      }
    ]
  }
}
```

### 3. StructField (Enum Field - Struct Variant)

```json
{
  ".name": {
    "description": "Mutate the name field of TestEnumWithSerDe",
    "mutation_status": "mutatable",
    "variants": ["Custom"],
    "signature": "String",
    "path_info": {
      "path_kind": "StructField",
      "type": "alloc::string::String",
      "type_kind": "Value"
    },
    "examples": [
      {
        "example": "Hello, World!"
      }
    ]
  }
}
```

### 4. StructField (Enum - Option<T>)

```json
{
  ".custom_size": {
    "description": "Mutate the custom_size field of bevy_sprite::sprite::Sprite",
    "mutation_status": "mutatable",
    "variants": ["None", "Some"],
    "signature": "enum",
    "path_info": {
      "path_kind": "StructField",
      "type": "core::option::Option<glam::Vec2>",
      "type_kind": "Enum"
    },
    "examples": [
      {
        "variants": ["None"],
        "signature": "unit",
        "example": "None"
      },
      {
        "variants": ["Some"],
        "signature": "tuple(glam::Vec2)",
        "example": {"Some": [64.0, 64.0]}
      }
    ]
  }
}
```

### 5. StructField (Value)

```json
{
  ".flip_x": {
    "description": "Mutate the flip_x field of bevy_sprite::sprite::Sprite",
    "mutation_status": "mutatable",
    "variants": [],
    "signature": "bool",
    "path_info": {
      "path_kind": "StructField",
      "type": "bool",
      "type_kind": "Value"
    },
    "examples": [
      {
        "example": true
      }
    ]
  }
}
```

### 6. StructField (Struct)

```json
{
  ".transform": {
    "description": "Mutate the transform field of bevy_transform::components::transform::Transform",
    "mutation_status": "mutatable",
    "variants": [],
    "signature": "Transform",
    "path_info": {
      "path_kind": "StructField",
      "type": "bevy_transform::components::transform::Transform",
      "type_kind": "Struct"
    },
    "examples": [
      {
        "example": {
          "translation": [0.0, 0.0, 0.0],
          "rotation": [0.0, 0.0, 0.0, 1.0],
          "scale": [1.0, 1.0, 1.0]
        }
      }
    ]
  }
}
```

### 7. IndexedElement (Tuple Element)

```json
{
  ".0": {
    "description": "Mutate the first element of a tuple",
    "mutation_status": "mutatable",
    "variants": [],
    "signature": "f32",
    "path_info": {
      "path_kind": "IndexedElement",
      "type": "f32",
      "type_kind": "Value"
    },
    "examples": [
      {
        "example": 3.14159
      }
    ]
  }
}
```

### 8. ArrayElement

```json
{
  ".points[0]": {
    "description": "Mutate the first element of the points array",
    "mutation_status": "mutatable",
    "variants": [],
    "signature": "Vec2",
    "path_info": {
      "path_kind": "ArrayElement",
      "type": "glam::Vec2",
      "type_kind": "Struct"
    },
    "examples": [
      {
        "example": [10.0, 20.0]
      }
    ]
  }
}
```

### 9. StructField (Complex Enum with Multiple Signatures)

```json
{
  ".image_mode": {
    "description": "Mutate the image_mode field of bevy_sprite::sprite::Sprite",
    "mutation_status": "mutatable",
    "variants": ["Auto", "Scale", "Sliced", "Tiled"],
    "signature": "enum",
    "path_info": {
      "path_kind": "StructField",
      "type": "bevy_sprite::sprite::SpriteImageMode",
      "type_kind": "Enum"
    },
    "examples": [
      {
        "variants": ["Auto"],
        "signature": "unit",
        "example": "Auto"
      },
      {
        "variants": ["Scale"],
        "signature": "tuple(bevy_sprite::sprite::ScalingMode)",
        "example": {"Scale": "FillCenter"}
      },
      {
        "variants": ["Sliced"],
        "signature": "tuple(bevy_sprite::texture_slice::slicer::TextureSlicer)",
        "example": {
          "Sliced": {
            "border": {"left": 10.0, "right": 10.0, "top": 10.0, "bottom": 10.0},
            "center_scale_mode": "Stretch",
            "sides_scale_mode": "Stretch",
            "max_corner_scale": 1.0
          }
        }
      },
      {
        "variants": ["Tiled"],
        "signature": "struct{stretch_value: f32, tile_x: bool, tile_y: bool}",
        "example": {
          "Tiled": {
            "stretch_value": 1.0,
            "tile_x": true,
            "tile_y": false
          }
        }
      }
    ]
  }
}
```

### 10. StructField (Set)

```json
{
  ".string_set": {
    "description": "Mutate the string_set field of SimpleSetComponent",
    "mutation_status": "mutatable",
    "variants": [],
    "signature": "HashSet<String>",
    "path_info": {
      "path_kind": "StructField",
      "type": "std::collections::hash::set::HashSet<String>",
      "type_kind": "Set"
    },
    "examples": [
      {
        "example": ["hello", "world", "test"]
      }
    ]
  }
}
```

### 11. Non-Mutatable Path

```json
{
  ".internal_state": {
    "description": "Cannot mutate internal_state field",
    "mutation_status": "not_mutatable",
    "variants": [],
    "signature": "State",
    "path_info": {
      "path_kind": "StructField",
      "type": "bevy_internal::State",
      "type_kind": "Struct"
    },
    "examples": [],
    "error_reason": "Type bevy_internal::State lacks Serialize/Deserialize traits required for mutation"
  }
}
```

## Enum Variant Grouping Rules

When processing enum types, variants are grouped by their structural signature:

1. **Unit variants** - All variants with no data are grouped together
   - Signature: `"unit"`
   - Example shows one variant name as the example, but lists ALL unit variants in the group's `variants` array

2. **Tuple variants** - Grouped by their tuple type signature
   - Signature: `"tuple(TypeName)"` or `"tuple(Type1, Type2, ...)"` for multiple fields
   - Each unique tuple signature gets its own group
   - The group's `variants` array lists all variants sharing that exact signature

3. **Struct variants** - Grouped by their field names and types
   - Signature: `"struct{field1: Type1, field2: Type2, ...}"`
   - Each unique combination of field names and types gets its own group
   - The group's `variants` array lists all variants with matching field structure

### Variant Filtering Example

For an enum with variants `[Active, Inactive, Special, Custom]`:
- Path-level: `"variants": ["Active", "Inactive", "Special", "Custom"]` (ALL variants)
- Example group for unit signature: `"variants": ["Active", "Inactive"]` (ONLY unit variants)
- Example group for tuple signature: `"variants": ["Special"]` (ONLY that tuple variant)
- Example group for struct signature: `"variants": ["Custom"]` (ONLY that struct variant)

## Benefits of This Structure

1. **Consistency** - Same structure for all mutation paths regardless of kind
2. **Clarity** - Enum variants with same signature are explicitly grouped
3. **Completeness** - Shows all valid variants for each signature
4. **Usability** - Users can pick any variant from a group and know the structure
5. **Machine-readable** - Tools can parse and validate mutation values
6. **Human-readable** - Clear what values are valid for each path

## Code Structure Locations

The mutation path building system is organized in a modular structure:

### Core Types and Traits
- **`PathKind`** (formerly `MutationPathKind`) - Defined in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/path_kind.rs` (line 9)
- **`TypeKind`** - Defined in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/type_kind.rs` (includes Array, Enum, List, Map, Set, Struct, Tuple, TupleStruct, Value)
- **`MutationPathBuilder` trait** - Defined in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/mod.rs` (line 20)
- **`RecursionContext`** (formerly `MutationPathContext`) - Defined in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`
- **`RootOrField`** - Defined in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`

### Builder Implementations
- **`EnumMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/enum_builder.rs`
- **`StructMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/struct_builder.rs`
- **`ArrayMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/array_builder.rs`
- **`TupleMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/tuple_builder.rs`
- **`ListMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/list_builder.rs`
- **`MapMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/map_builder.rs`
- **`SetMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/set_builder.rs`
- **`DefaultMutationBuilder`** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/default_builder.rs`

### Module Organization
- **Main module** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/mod.rs`
- **Builders module** - `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/mod.rs`

### Key Functions for Enum Handling
- **`build_all_enum_examples`** - Located in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/enum_builder.rs`
- **`deduplicate_variant_signatures`** - Located in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/enum_builder.rs`
- **`extract_enum_variants`** - Located in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/enum_builder.rs`

## Implementation Notes

- The `examples` array is never empty for mutatable paths
- For enums, deduplicate variants by signature before creating groups
- The `signature` field should be human-readable but also precise
- The `example` in each group should use the first variant alphabetically for consistency
- Non-mutatable paths have empty `examples` array and include `error_reason`
- Enum variant grouping logic is implemented in `EnumMutationBuilder::build_paths()`
- The modular builder structure allows each type kind to handle its own mutation logic independently
- `ListMutationBuilder` now adds both list-level and element-level mutation paths
- `SetMutationBuilder` handles `HashSet` and `BTreeSet` types similar to `ListMutationBuilder`
- The context uses `PathKind` instead of `MutationPathKind` for consistency
- `RecursionContext` handles path prefix building for nested structures

### Conversion Strategy from MutationPathInternal

The conversion from `MutationPathInternal` to the new `MutationPath` structure should:

1. **Use existing serialization** - Leverage existing Display/ToString implementations
2. **Reuse `deduplicate_variant_signatures`** logic for grouping enum variants
3. **For enum types**: Group examples by variant structure
   - Extract variants from the `example` field's object keys
   - Group by their structural signature (unit/tuple/struct)
   - Create `ExampleGroup` entries with variants and signature fields
4. **For non-enum types**: Single example without variants/signature fields

#### Signature Generation for Enum Variants

Signatures ONLY appear in the examples array for enum types to describe variant structures:

- **Unit variants**: `"unit"`
- **Tuple variants**: 
  - Single element: `"tuple(TypeName)"`
  - Multiple elements: `"tuple(Type1, Type2, Type3)"`
- **Struct variants**: `"struct{field1: Type1, field2: Type2}"`

The path-level `variants` field lists all enum variants. There is no path-level signature field.
For non-enum types, the examples array contains a single example with just the `example` field.

### MutationPathBuilder Trait Integration

The `MutationPathBuilder` trait interface remains unchanged - it continues to return `Vec<MutationPathInternal>`. The new output format is achieved through the conversion layer:

1. **Keep trait unchanged**: `build_paths()` continues returning `Result<Vec<MutationPathInternal>>`
2. **Update MutationPath struct**: Add new fields (`variants`, `signature`, `examples` array)
3. **Update conversion method**: `MutationPath::from_mutation_path_internal()` transforms the internal representation to the new output format
4. **Backward compatibility**: Existing builders don't need changes; only the conversion layer updates

This approach separates internal representation (MutationPathInternal) from external API (MutationPath), allowing evolution of the output format without breaking the builder infrastructure.

## TYPE-SYSTEM-4: VariantSignature enum lacks Display trait required by conversion strategy ✅
- **Category**: TYPE-SYSTEM
- **Status**: APPROVED - To be implemented
- **Location**: Section: Conversion Strategy from MutationPathInternal
- **Issue Identified**: Plan specifies leveraging 'existing VariantSignature enum with Display impl for human-readable strings' but the current VariantSignature enum is private and lacks Display implementation
- **Verdict**: CONFIRMED
- **Reasoning**: This is a real issue that prevents implementation of the planned conversion strategy. The plan document explicitly states on line 495 to 'Leverage existing VariantSignature enum with Display impl for human-readable strings', but the current code lacks both public visibility and Display implementation. The conversion method needs to generate human-readable signature strings for the new output format, but cannot do so without Display trait on VariantSignature. This is a straightforward fix that requires adding public visibility and Display implementation.

### Approved Change:
Make VariantSignature public and implement Display trait:
```rust
/// Variant signatures for deduplication - same signature means same inner structure
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum VariantSignature {
    /// Unit variants (no data)
    Unit,
    /// Tuple variants with specified types
    Tuple(Vec<BrpTypeName>),
    /// Struct variants with field names and types
    Struct(Vec<(String, BrpTypeName)>),
}

impl std::fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Tuple(types) => {
                if types.len() == 1 {
                    write!(f, "tuple({})", types[0])
                } else {
                    write!(f, "tuple({})", types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "))
                }
            }
            Self::Struct(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, typ)| format!("{name}: {typ}"))
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
}
```

### Implementation Notes:
The VariantSignature enum needs to be made public and have Display implementation added to support the conversion strategy's requirement for human-readable signature strings.

## DESIGN-4: Inconsistent signature format specification across plan sections ✅
- **Category**: DESIGN
- **Status**: APPROVED - To be implemented
- **Location**: Section: Signature Generation for Enum Variants vs Examples by PathKind
- **Issue Identified**: Plan shows conflicting signature formats - Conversion Strategy section specifies unit as '()' while Examples section shows 'unit', tuple format differs between '(TypeName)' vs 'tuple(TypeName)'
- **Verdict**: CONFIRMED
- **Reasoning**: This is a real inconsistency in the plan document. The specification section shows one format while all examples use a different, more descriptive format. The examples format is used consistently across 9+ examples throughout the document, making it the clear standard. The examples format ('unit', 'tuple(Type)', 'struct{field: Type}') is more readable and descriptive than the specification format ('()', '(Type)', 'struct { field: Type }'). Since implementation will follow the examples, the specification section needs to be corrected to match.

### Approved Change:
Updated signature format specification to match the consistent format used throughout all examples:
- Unit variants: `"unit"` (was `"()"`)
- Tuple variants: `"tuple(TypeName)"` (was `"(TypeName)"`)  
- Struct variants: `"struct{field1: Type1, field2: Type2}"` (was `"struct { field1: Type1, field2: Type2 }"`)

### Implementation Notes:
The specification now matches the format consistently used in all examples throughout the document, eliminating confusion for implementers.

## DESIGN-5: Missing handling of Option-specific fields in conversion strategy ✅
- **Category**: DESIGN
- **Status**: APPROVED - To be implemented
- **Location**: Section: MutationPathBuilder Trait Integration
- **Issue Identified**: Plan doesn't specify how existing Option-specific fields (example_some, example_none) integrate with new unified examples array structure
- **Verdict**: CONFIRMED
- **Reasoning**: This is a real design inconsistency. The current code has separate `example_some` and `example_none` fields that are hardcoded to None, but the plan's unified examples array structure treats Option types as regular enums with variants ['Some', 'None']. The plan document shows Option<T> types using the same examples array format as other enums (see the custom_size field example), but doesn't explicitly state that the old Option-specific fields should be removed. The conversion method needs to be updated to generate the proper examples array for Option types instead of using the deprecated Option-specific fields.

### Approved Change:
Remove Option-specific fields from MutationPath struct and use unified examples array:
```rust
// Remove these fields from MutationPath struct:
// - pub example_some: Option<Value>
// - pub example_none: Option<Value>

// Remove hardcoded assignments in from_mutation_path_internal method:
// - example_some: None,
// - example_none: None,

// Option types will use the unified examples array like other enums:
// examples: [
//   {
//     "variants": ["None"],
//     "signature": "unit",
//     "example": "None"
//   },
//   {
//     "variants": ["Some"], 
//     "signature": "tuple(T)",
//     "example": {"Some": value}
//   }
// ]
```

### Implementation Notes:
Option types should be treated as regular enums with variants ["Some", "None"] using the unified examples array structure, eliminating the need for special-case fields.

## DESIGN-6: Conversion strategy lacks specificity for signature generation implementation - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Conversion Strategy from MutationPathInternal
- **Issue**: Plan says to reuse deduplicate_variant_signatures logic but doesn't explain how VariantSignature enum maps to human-readable signature strings shown in examples
- **Reasoning**: The finding correctly identified that signature generation implementation was missing, but the solution already exists in the plan - TYPE-SYSTEM-4 already approved adding Display implementation to VariantSignature enum
- **Existing Implementation**: The approved change from TYPE-SYSTEM-4 includes Display implementation that converts VariantSignature enum values to human-readable strings, though it needs to use the corrected format from DESIGN-4
- **Plan Section**: TYPE-SYSTEM-4: VariantSignature enum lacks Display trait required by conversion strategy ✅
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### Implementation Notes:
The Display implementation from TYPE-SYSTEM-4 should be updated to use the corrected signature format from DESIGN-4 ("unit", "tuple(Type)", "struct{field: Type}" instead of "()", "(Type)", "struct { field: Type }").

## Design Review Skip Notes

### DUPLICATION-1: Complete Output Format Duplication - REJECTED
- **Status**: REJECTED - This is an atomic design change, not duplication
- **Category**: DUPLICATION
- **Location**: Section: Core Structure
- **Issue**: The plan proposes an entirely new JSON output structure that duplicates functionality already present in the existing MutationPath struct
- **Verdict**: MODIFIED
- **Decision**: Plan represents complete replacement of output format to achieve consistent structure and variant applicability clarity. User confirmed atomic change approach is correct.

## DESIGN-1: Missing conversion strategy from MutationPathInternal ✅
- **Category**: DESIGN
- **Status**: APPROVED - To be implemented
- **Location**: Section: Implementation Notes
- **Issue Identified**: Plan doesn't specify how the existing from_mutation_path_internal conversion method would be modified to produce the new structure
- **Verdict**: CONFIRMED
- **Reasoning**: The plan proposes a completely different output structure for MutationPath but provided no guidance on how to modify the existing conversion method. The conversion method needs to be completely rewritten to produce the new unified format.

### Approved Change:
Added "Conversion Strategy from MutationPathInternal" section that specifies:
1. Leverage existing `VariantSignature` enum with Display impl for human-readable strings
2. Reuse `deduplicate_variant_signatures` logic for grouping variants
3. Use existing enum Display/serialization - never hardcode strings
4. Group enum examples by signature during conversion
5. For non-enum types, create single example group without variant/signature fields

### Implementation Notes:
The conversion should build on existing infrastructure in `enum_builder.rs` rather than creating parallel string-based systems. This maintains type safety while achieving the desired output format.

## DESIGN-2: No integration with existing MutationPathBuilder trait ✅
- **Category**: DESIGN
- **Status**: APPROVED - To be implemented
- **Location**: Section: Code Structure Locations
- **Issue Identified**: Plan doesn't explain how the new output format integrates with the existing MutationPathBuilder trait that returns Vec<MutationPathInternal>
- **Verdict**: CONFIRMED
- **Reasoning**: The plan defines a completely new JSON output structure but doesn't explain how this integrates with the existing MutationPathBuilder trait. The plan needs to specify whether the trait interface changes or if the conversion layer handles the transformation.

### Approved Change:
Added "MutationPathBuilder Trait Integration" section that clarifies:
1. Keep the trait interface unchanged for backward compatibility
2. Update MutationPath struct with new fields (variants, signature, examples)
3. Handle transformation in the conversion layer (from_mutation_path_internal)
4. Maintain separation between internal representation and external API

### Implementation Notes:
This approach maintains backward compatibility while achieving the new output format through the conversion layer.

### TYPE-SYSTEM-1: String-based mutation_status violates enum typing principles
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: Section: Core Structure
- **Issue**: Plan proposes string literals for mutation_status instead of using the existing type-safe MutationStatus enum
- **Proposed Change**: Use the existing MutationStatus enum instead of string literals
- **Verdict**: REJECTED
- **Reasoning**: This is a false positive. The plan shows the JSON output format where the enum values appear as strings, but the code correctly uses the type-safe MutationStatus enum with serde serialization. The enum's snake_case serialization automatically converts Mutatable to 'mutatable', NotMutatable to 'not_mutatable', and PartiallyMutatable to 'partially_mutatable'. The implementation maintains full type safety while producing the correct JSON format described in the plan.
- **Decision**: User elected to skip this recommendation

### TYPE-SYSTEM-2: Inconsistent variants field structure creates conditional logic
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: Section: Core Structure
- **Issue**: Plan shows variants field sometimes empty array, sometimes omitted, creating string-based conditional handling instead of using existing type-safe enum_variants Option
- **Proposed Change**: Keep the existing Option<Vec<String>> type which already handles presence/absence correctly through the type system
- **Verdict**: REJECTED
- **Reasoning**: The current implementation correctly uses Option<Vec<String>> for enum_variants with proper type safety. The plan has been updated to clarify: (1) The JSON 'variants' field maps to the code's 'enum_variants' field, (2) Implementation can either rename the field or use serde rename attribute, (3) The field properly uses Option with skip_serializing_if for clean presence/absence handling, (4) For enum types, the path-level variants lists ALL variants while example-level variants are filtered to only those sharing the same signature.
- **Decision**: User elected to skip this recommendation after clarifying the plan

### TYPE-SYSTEM-3: PathKind variant serialization regression
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: Section: Core Structure
- **Issue**: Plan path_kind field shows string variants instead of leveraging existing custom Serialize implementation that maintains type safety
- **Proposed Change**: Use the existing PathKind enum with its custom serialization instead of requiring string literals
- **Verdict**: REJECTED
- **Reasoning**: This finding is based on a fundamental misunderstanding. The current custom Serialize implementation DOES produce string variants exactly as the plan specifies. Looking at the plan document, all examples show path_kind as strings like 'RootValue', 'StructField', etc. The current implementation serializes the enum to these exact string values through its custom Serialize trait, which calls to_string(), which uses Display, which returns variant_name(). The plan intentionally separates concerns - path_kind provides a simple string categorization while detailed type information is preserved in separate fields like 'type' and 'type_kind'. The current implementation correctly implements the planned architecture.
- **Decision**: User elected to skip this recommendation

### DESIGN-3: Signature field lacks clear type definition - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Category**: DESIGN
- **Location**: Section: Core Structure
- **Issue**: Plan introduces 'signature' field with human-readable descriptions but doesn't specify the type system or validation rules
- **Existing Implementation**: The plan already specifies in "Conversion Strategy from MutationPathInternal" to leverage the existing VariantSignature enum with a Display impl for human-readable strings
- **Plan Section**: Section: Conversion Strategy from MutationPathInternal, point #1
- **Verdict**: CONFIRMED
- **Reasoning**: The finding was valid but the solution already exists in the plan - we should extend VariantSignature with Display impl rather than using free-form strings
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### QUALITY-1: Inconsistent field presence across examples - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Category**: QUALITY
- **Location**: Section: Examples by PathKind
- **Issue**: Documentation incorrectly states 'Only for enum root paths' when variants/signature fields actually appear for all enum types regardless of PathKind
- **Existing Implementation**: The plan Core Structure section was updated during this review to correctly state "Only for enum types" instead of "Only for enum root paths"
- **Plan Section**: Section: Core Structure
- **Verdict**: MODIFIED
- **Reasoning**: The finding correctly identified inconsistent documentation, but the fix was already applied during this review session
- **Critical Note**: This documentation issue was already corrected - future reviewers should check current plan state before suggesting

### QUALITY-2: Variant grouping logic undefined for edge cases - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Category**: QUALITY
- **Location**: Section: Enum Variant Grouping Rules
- **Issue**: Plan doesn't address how the new grouping differs from existing deduplicate_variant_signatures logic or what happens with complex nested signatures
- **Existing Implementation**: The plan "Conversion Strategy from MutationPathInternal" section explicitly states to reuse existing deduplicate_variant_signatures logic and shows exact signature format
- **Plan Section**: Section: Conversion Strategy from MutationPathInternal
- **Verdict**: CONFIRMED
- **Reasoning**: The finding claims the plan doesn't explain the relationship to existing logic, but the plan explicitly says to reuse deduplicate_variant_signatures and provides detailed signature formatting rules
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting