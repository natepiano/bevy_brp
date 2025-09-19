# Expected Changes in Mutation Test JSON

This file documents expected changes between baseline and current mutation test JSON files that occur due to intentional refactoring. These changes should be recognized and grouped separately from unexpected changes during comparison.

## Expected Change #1: Removal of "variants" Arrays from Mutation Paths

### Description
The refactoring in commit 7db229f removed "variants" arrays from individual field-level mutation paths. These arrays previously indicated which enum variant owned each field (e.g., "LinearRgba" for color fields).

### Structural Change
**Removed**: "variants" field from all field-level mutation paths

### When Comparison Output Says:
```
📌 IDENTIFIED PATTERN: FIELD REMOVED
Fields removed breakdown:
  • 'variants' field: 874 removal(s) across 57 type(s)
```

**This is talking about**: The removal of "variants" arrays from mutation paths, which is EXPECTED.

### Example: bevy_sprite::sprite::Sprite

#### BASELINE (from get_guide.sh)
```json
".color.0.blue": {
  "description": "Mutate the blue field of LinearRgba",
  "example": 3.1415927410125732,
  "path_info": {
    "mutation_status": "mutable",
    "path_kind": "StructField",
    "type": "f32",
    "type_kind": "Value"
  },
  "variants": ["LinearRgba"]  // <-- HAD THIS
}
```

#### CURRENT (from live type guide)
```json
".color.0.blue": {
  "description": "Mutate the blue field of Srgba",
  "example": 3.1415927410125732,
  "path_info": {
    "mutation_status": "mutable",
    "path_kind": "StructField",
    "type": "f32",
    "type_kind": "Value"
  }
  // NO "variants" field
}
```

<HowToIdentify>
To identify this change pattern in the comparison output:

1. Look for FIELD REMOVED pattern with:
   - 'variants' field: 800+ removals across 50+ types

2. This is the EXACT line to match:
   ```
   • 'variants' field: [NUMBER] removal(s) across [NUMBER] type(s)
   ```

3. Any "variants" field removal count above 800 should be considered this expected change.

This is an EXPECTED change from the enum variant optimization refactoring and should not be flagged as an error.
</HowToIdentify>

## Expected Change #2: Removal of "enum_info" Field from Enum Types

### Description
The "enum_info" field has been removed from the root level of enum types. This field previously contained variant information that has been restructured in the current implementation.

### Structural Change
**Removed**: "enum_info" field from root level of enum types

### When Comparison Output Says:
```
📌 IDENTIFIED PATTERN: FIELD REMOVED
Fields removed breakdown:
  • 'enum_info' field: 18 removal(s) across 18 type(s)
```

**This is talking about**: The removal of "enum_info" field from enum types, which is EXPECTED.

### Example: bevy_sprite::sprite::Anchor

#### BASELINE
```json
{
  "type_name": "bevy_sprite::sprite::Anchor",
  "enum_info": [
    {
      "variant_name": "Center",
      "fields": []
    },
    {
      "variant_name": "BottomLeft",
      "fields": []
    }
    // ... more variants
  ],
  "mutation_paths": {
    // ... mutation paths
  }
}
```

#### CURRENT
```json
{
  "type_name": "bevy_sprite::sprite::Anchor",
  // NO "enum_info" field
  "mutation_paths": {
    // ... mutation paths
  }
}
```

<HowToIdentify>
To identify this change pattern in the comparison output:

1. Look for FIELD REMOVED pattern with:
   - 'enum_info' field: removals from type root paths

2. This is the EXACT line to match:
   ```
   • 'enum_info' field: [NUMBER] removal(s) across [NUMBER] type(s)
   ```

3. The path for these removals will be at the root level (empty path or just the type name)

This is an EXPECTED change from refactoring how enum variant information is stored and should not be flagged as an error.
</HowToIdentify>

---

*Note: Additional expected changes will be documented here as they are discovered during testing and refactoring.*