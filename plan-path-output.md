# Mutation Path Output Structure Plan

## Overview
This plan defines a unified output structure for mutation paths that works consistently across all `MutationPathKind` variants while properly representing enum variant groups by their type signatures.

## Core Structure

Every mutation path entry follows this structure:

```json
{
  ".path": {
    "description": "Human-readable description of what this path mutates",
    "mutation_status": "mutatable" | "not_mutatable" | "partially_mutatable",
    "path_info": {
      "path_kind": "MutationPathKind variant",
      "type": "Fully qualified type name",
      "type_kind": "TypeKind variant (Enum, Struct, Value, Array, etc.)"
    },
    "examples": [
      {
        "variants": ["List", "Of", "Variants"],  // Only for enums
        "signature": "Description of type signature",  // Only for enums
        "example": "Actual value to send"
      }
    ],
    "error_reason": "Optional error message for non-mutatable paths"
  }
}
```

## Key Design Decisions

1. **`examples` is always an array** - Provides consistency across all types
2. **Enum variants are grouped by signature** - All variants with identical type signatures share one example
3. **Non-enums have a single example** - Array contains one element with just `example` field
4. **Structure is identical for all `MutationPathKind` variants** - Works for RootValue, StructField, IndexedElement, and ArrayElement

## Examples by MutationPathKind

### 1. RootValue (Enum)

```json
{
  "": {
    "description": "Mutate the entire Color value",
    "mutation_status": "mutatable",
    "path_info": {
      "path_kind": "RootValue",
      "type": "bevy_color::color::Color",
      "type_kind": "Enum"
    },
    "examples": [
      {
        "variants": ["Srgba", "LinearRgba"],
        "signature": "struct{red: f32, green: f32, blue: f32, alpha: f32}",
        "example": {"Srgba": {"red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0}}
      },
      {
        "variants": ["Oklaba"],
        "signature": "struct{lightness: f32, a: f32, b: f32, alpha: f32}",
        "example": {"Oklaba": {"lightness": 0.5, "a": 0.0, "b": 0.0, "alpha": 1.0}}
      }
    ]
  }
}
```

### 2. StructField (Enum - Option<T>)

```json
{
  ".custom_size": {
    "description": "Mutate the custom_size field of bevy_sprite::sprite::Sprite",
    "mutation_status": "mutatable",
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

### 3. StructField (Value)

```json
{
  ".flip_x": {
    "description": "Mutate the flip_x field of bevy_sprite::sprite::Sprite",
    "mutation_status": "mutatable",
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

### 4. StructField (Struct)

```json
{
  ".transform": {
    "description": "Mutate the transform field of bevy_transform::components::transform::Transform",
    "mutation_status": "mutatable",
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

### 5. IndexedElement (Tuple Element)

```json
{
  ".0": {
    "description": "Mutate the first element of a tuple",
    "mutation_status": "mutatable",
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

### 6. ArrayElement

```json
{
  ".points[0]": {
    "description": "Mutate the first element of the points array",
    "mutation_status": "mutatable",
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

### 7. StructField (Complex Enum with Multiple Signatures)

```json
{
  ".image_mode": {
    "description": "Mutate the image_mode field of bevy_sprite::sprite::Sprite",
    "mutation_status": "mutatable",
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

### 8. Non-Mutatable Path

```json
{
  ".internal_state": {
    "description": "Cannot mutate internal_state field",
    "mutation_status": "not_mutatable",
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
   - Example shows one variant name, but lists all in `variants` array

2. **Tuple variants** - Grouped by their tuple type signature
   - Signature: `"tuple(TypeName)"` or `"tuple(Type1, Type2, ...)"` for multiple fields
   - Each unique tuple signature gets its own group

3. **Struct variants** - Grouped by their field names and types
   - Signature: `"struct{field1: Type1, field2: Type2, ...}"`
   - Each unique combination of field names and types gets its own group

## Benefits of This Structure

1. **Consistency** - Same structure for all mutation paths regardless of kind
2. **Clarity** - Enum variants with same signature are explicitly grouped
3. **Completeness** - Shows all valid variants for each signature
4. **Usability** - Users can pick any variant from a group and know the structure
5. **Machine-readable** - Tools can parse and validate mutation values
6. **Human-readable** - Clear what values are valid for each path

## Implementation Notes

- The `examples` array is never empty for mutatable paths
- For enums, deduplicate variants by signature before creating groups
- The `signature` field should be human-readable but also precise
- The `example` in each group should use the first variant alphabetically for consistency
- Non-mutatable paths have empty `examples` array and include `error_reason`