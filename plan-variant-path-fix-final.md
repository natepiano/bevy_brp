# Plan: Final Variant Path Implementation Issues

## Investigation Summary

After implementing the initial variant path fixes, two critical issues remain that affect the usability of the mutation path system:

### Issue A: Missing `examples` Array for Enum Paths
**Problem:** Enum field paths like `.nested_config` are showing a single `example` field instead of an `examples` array with multiple signature groups.

**Current Output:**
```json
".nested_config": {
  "example": "Always"
}
```

**Expected Output:**
```json
".nested_config": {
  "examples": [
    {
      "applicable_variants": ["NestedConfigEnum::Always", "NestedConfigEnum::Never"],
      "example": "Always",
      "signature": "unit"
    },
    {
      "applicable_variants": ["NestedConfigEnum::Conditional"],
      "example": {"Conditional": 1000000},
      "signature": "tuple(u32)"
    }
  ]
}
```

**Root Cause:** The enum detection logic in `builder.rs:386` sets `EnumContext::Root` for enum fields, but the `from_mutation_path_internal()` method in `types.rs:199-201` only uses the `examples` array for the root enum path (`""`), not for nested enum fields.

### Issue B: PathRequirement Examples Missing Context
**Problem:** PathRequirement examples contain simple values instead of complete enum structure context.

**Current Output:**
```json
"path_requirement": {
  "example": "Hello, World!"
}
```

**Expected Output:**
```json
"path_requirement": {
  "example": {
    "Special": ["Hello, World!", 1000000]
  }
}
```

**Root Cause:** In `builder.rs:461`, the PathRequirement uses `example.clone()` which is the immediate field value, not the complete parent enum structure needed for context.

## Implementation Plan

### Fix A: Enable Examples Array for Enum Fields

**Location:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:195-203`

**Current Logic:**
```rust
let (examples, example) = path.enum_root_examples.as_ref().map_or_else(
    || {
        // Non-enum or enum child: use single example
        (vec![], Some(path.example.clone()))
    },
    |enum_examples| {
        // Enum root: use the examples array
        (enum_examples.clone(), None)
    },
);
```

**Proposed Fix:**
```rust
let (examples, example) = path.enum_root_examples.as_ref().map_or_else(
    || {
        // Check if this is an enum field (has enum_context and type_kind is Enum)
        if matches!(path.path_kind, PathKind::StructField { .. })
            && matches!(type_kind, TypeKind::Enum) {
            // This is an enum field - it should have examples array but doesn't
            // This indicates the enum_root_examples wasn't populated properly
            tracing::warn!(
                "Enum field {} missing examples array, falling back to single example",
                path.path
            );
        }
        (vec![], Some(path.example.clone()))
    },
    |enum_examples| {
        // Enum root OR enum field: use the examples array
        (enum_examples.clone(), None)
    },
);
```

**Additional Investigation Needed:**
The real fix requires understanding why `enum_root_examples` is not being populated for enum fields. The issue is likely in the enum builder where `EnumContext::Root` paths don't properly pass their examples up to the `MutationPathInternal`.

**Deeper Fix Location:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs:404-415`

The `EnumRoot` case returns a default example, but this should be passed as `enum_root_examples` to `build_mutation_path_internal_with_enum_examples()`.

### Fix B: Generate Proper PathRequirement Context Examples

**Location:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs:454-467`

**Current Code:**
```rust
let path_requirement = match &ctx.enum_context {
    Some(super::recursion_context::EnumContext::Child { variant_chain })
        if !variant_chain.is_empty() =>
    {
        Some(super::types::PathRequirement {
            description:  Self::generate_variant_description(variant_chain),
            example:      example.clone(), // ← PROBLEM: Uses immediate field value
            variant_path: variant_chain.clone(),
        })
    }
    _ => None,
};
```

**Proposed Fix:**
```rust
let path_requirement = match &ctx.enum_context {
    Some(super::recursion_context::EnumContext::Child { variant_chain })
        if !variant_chain.is_empty() =>
    {
        Some(super::types::PathRequirement {
            description:  Self::generate_variant_description(variant_chain),
            example:      Self::build_context_example(ctx, variant_chain, &example),
            variant_path: variant_chain.clone(),
        })
    }
    _ => None,
};
```

**New Helper Function:**
```rust
/// Build a complete context example showing the full enum structure
/// needed to reach this mutation path
fn build_context_example(
    ctx: &RecursionContext,
    variant_chain: &[super::types::VariantPathEntry],
    field_example: &Value,
) -> Value {
    // Work backwards through the variant chain to build the complete structure
    let mut result = field_example.clone();

    for entry in variant_chain.iter().rev() {
        if entry.path.is_empty() {
            // Root level - wrap in variant
            let variant_name = entry.variant.split("::").last().unwrap_or(&entry.variant);
            if result == json!(null) || result.is_string() {
                // Unit variant or simple value
                result = json!({ variant_name: result });
            } else {
                // Struct variant
                result = json!({ variant_name: result });
            }
        } else {
            // Nested level - build containing structure
            let field_name = entry.path.trim_start_matches('.');
            result = json!({ field_name: result });
        }
    }

    result
}
```

## Implementation Sequence

### Step 1: Fix PathRequirement Context Examples
**Objective:** Generate proper context examples for PathRequirement
**Files:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Risk:** LOW - Isolated change to example generation
**Impact:** PathRequirement examples will show complete enum structures

### Step 2: Investigate Enum Field Examples Array Issue
**Objective:** Understand why `enum_root_examples` is not populated for enum fields
**Files:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Risk:** MEDIUM - Requires understanding complex enum handling flow
**Impact:** Enum fields will properly display examples arrays instead of single example

### Step 3: Fix Examples Array Population
**Objective:** Ensure enum fields get proper examples arrays
**Files:** Multiple enum builder and protocol enforcer files
**Risk:** HIGH - Complex change affecting enum example generation
**Impact:** All enum fields will display proper examples arrays with signatures

## Validation Criteria

After fixes, the following should match the reference JSON:

1. **Enum field paths** like `.nested_config` should have `examples` arrays, not single `example`
2. **PathRequirement examples** should show complete enum structures, not simple field values
3. **All variant names** should be properly formatted (already fixed)
4. **Signature groups** should only show representative variants (already fixed)

## Expected Timeline

- **Step 1:** 1-2 hours (straightforward helper function)
- **Step 2:** 2-4 hours (investigation of enum flow)
- **Step 3:** 4-6 hours (complex enum builder changes)

**Total Effort:** 1-2 days to complete both remaining issues.

## Risk Assessment

**Step 1 (Low Risk):** Safe isolated change to PathRequirement example generation.

**Steps 2-3 (Medium-High Risk):** Changes to enum example flow could affect:
- Root enum path generation
- Enum child path generation
- Example serialization
- Signature grouping logic

Recommend thorough testing with multiple enum types after implementation.