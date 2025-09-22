# Bug Report: Color Enum Field Name Confusion in Type Guide Generation

## Executive Summary
The type guide generator has a critical bug where enum variants with the same structural signature but different field names get their fields mixed up. This causes mutations to fail with "missing field" errors because the generated examples use wrong field names.

## Root Cause
The enum builder groups variants by their **structural signature** (field types only), ignoring field names. When multiple variants have the same structure but different field names, only one set of field names survives, and all variants incorrectly use those field names.

### Example: `bevy_pbr::fog::DistanceFog`

This component has two `bevy_color::color::Color` fields that demonstrate the issue:

```rust
pub struct DistanceFog {
    pub color: Color,                    // The fog color
    pub directional_light_color: Color,  // Light scattering color
    pub directional_light_exponent: f32,
    pub falloff: FogFalloff,
}
```

## The Bug in Action

### What SHOULD be generated for Color variants:

```json
// Srgba variant - CORRECT field names
{
  "Srgba": {
    "red": 1.0,
    "green": 0.5,
    "blue": 0.0,
    "alpha": 1.0
  }
}

// Hsla variant - CORRECT field names
{
  "Hsla": {
    "hue": 180.0,
    "saturation": 1.0,
    "lightness": 0.5,
    "alpha": 1.0
  }
}

// Xyza variant - CORRECT field names
{
  "Xyza": {
    "x": 0.5,
    "y": 0.5,
    "z": 0.5,
    "alpha": 1.0
  }
}
```

### What IS ACTUALLY generated (non-deterministic):

#### Run 1: All variants get Xyza's field names
```json
{
  ".color": {
    "examples": [
      {
        "applicable_variants": ["Color::Srgba"],
        "example": {
          "Srgba": {
            "x": 3.14,      // ❌ WRONG - should be "red"
            "y": 3.14,      // ❌ WRONG - should be "green"
            "z": 3.14,      // ❌ WRONG - should be "blue"
            "alpha": 3.14   // ✅ Correct
          }
        }
      },
      {
        "applicable_variants": ["Color::Hsla"],
        "example": {
          "Hsla": {
            "x": 3.14,      // ❌ WRONG - should be "hue"
            "y": 3.14,      // ❌ WRONG - should be "saturation"
            "z": 3.14,      // ❌ WRONG - should be "lightness"
            "alpha": 3.14   // ✅ Correct
          }
        }
      }
    ]
  }
}
```

#### Run 2: All variants get mixed field names
```json
{
  ".color": {
    "examples": [
      {
        "applicable_variants": ["Color::Srgba"],
        "example": {
          "Srgba": {
            "chroma": 3.14,    // ❌ WRONG - not even a field in Srgba!
            "hue": 3.14,       // ❌ WRONG - should be "green"
            "lightness": 3.14, // ❌ WRONG - should be "blue"
            "alpha": 3.14      // ✅ Correct
          }
        }
      }
    ]
  }
}
```

## Mutation Test Failures

When the mutation test tries to use these incorrect examples:

```bash
# Attempt to mutate with incorrect field structure
bevy/mutate_component {
  "entity": 123,
  "component": "bevy_pbr::fog::DistanceFog",
  "path": ".color",
  "value": {
    "Srgba": {
      "x": 1.0,      # ❌ BRP Error: missing field 'red'
      "y": 0.0,      # ❌ BRP Error: missing field 'green'
      "z": 0.0,      # ❌ BRP Error: missing field 'blue'
      "alpha": 1.0
    }
  }
}

# Error Response:
"missing field `red`"
```

## Why This Happens

### Current Algorithm (Buggy)
1. `collect_children()` groups ALL Color variants together because they have same structure (4 f32 fields)
2. Creates ONE set of PathKind children with field names from whichever variant is processed first
3. ALL variants use these same field names in their examples
4. Field names are WRONG for most variants

### The Grouping Problem
```rust
// In enum_builder.rs
fn group_variants_by_signature(variants) {
    // Groups Srgba, Hsla, Hsva, Laba, etc. ALL TOGETHER
    // because they all have signature: Struct with 4 f32 fields
    // But they have DIFFERENT field names!
}
```

## Impact
- **Affected Types**: Any enum where multiple variants have same structure but different field names
- **Severity**: HIGH - Makes mutations fail completely
- **Frequency**: Affects ALL Color enums in DistanceFog, materials, lights, etc.
- **Non-deterministic**: Different field names appear in different runs (HashMap iteration order)

## The Fix: Plan 1 & 2

### Plan 1: Defer Grouping
Instead of grouping variants early (losing field name info), defer grouping to output stage:
- Process each variant individually during recursion
- Preserve ALL field name information
- Group only for final output presentation

### Plan 2: Variant Chain Tracking
Track the complete variant chain to provide correct examples:
- Each mutation path knows its exact variant requirements
- Provide complete, correct examples for each specific variant
- No more field name confusion

## Test Case for Validation

After implementing the fix, this mutation MUST work:

```rust
// Test: Mutate DistanceFog color to red using correct Srgba fields
let mutation = json!({
    "entity": entity_id,
    "component": "bevy_pbr::fog::DistanceFog",
    "path": ".color",
    "value": {
        "Srgba": {
            "red": 1.0,      // ✅ Correct field name
            "green": 0.0,    // ✅ Correct field name
            "blue": 0.0,     // ✅ Correct field name
            "alpha": 1.0     // ✅ Correct field name
        }
    }
});

// This MUST succeed, not fail with "missing field" errors
```

## Rationale to Wait for Proper Fix

While we could hack a quick fix specific to Color enums, this is a **systematic architectural issue** that affects:
1. Any enum with variants that have same structure but different field names
2. The entire mutation path generation system's correctness
3. User trust in the type guide's examples

The proper fix (Plan 1 & 2) will:
- Solve this systematically for ALL affected enums
- Provide correct, complete examples for every mutation path
- Make the type guide actually trustworthy for automated testing

Therefore, we should implement the comprehensive solution rather than a band-aid fix.