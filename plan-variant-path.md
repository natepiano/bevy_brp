# Plan: Variant Path Structure for Mutation Paths

## Problem Statement

We need to clearly distinguish between two different concepts in our mutation path system:

1. **ExampleGroup applicable_variants**: Which variants within the current enum an example applies to
2. **MutationPath variant requirements**: The complete path through nested enums required for a mutation path to be valid

## Current Confusion

Right now we use `applicable_variants` for both concepts, which creates ambiguity:

```json
// In ExampleGroup - means "this example works for these variants of THIS enum"
"applicable_variants": ["TestEnumWithSerDe::Active", "TestEnumWithSerDe::Inactive"]

// At MutationPath level - means "this path requires this chain of variants"
"applicable_variants": ["TestEnumWithSerDe::Nested", "NestedConfigEnum::Conditional"]
```

## Proposed Solution

### 1. Keep `applicable_variants` for ExampleGroup
This shows which variants of the current enum type the example applies to:

```json
{
  "applicable_variants": ["TestEnumWithSerDe::Active", "TestEnumWithSerDe::Inactive"],
  "example": "Active",
  "signature": "unit"
}
```

### 2. Add `variant_path` for MutationPath
This shows the complete path through nested enums required for the mutation to be valid:

```json
{
  "description": "Mutate element 0 of NestedConfigEnum",
  "example": 1000000,
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Nested"},
    {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
  ],
  "path_info": { ... }
}
```

## Structure Definition

### ExampleGroup (unchanged)
```json
{
  "applicable_variants": ["EnumName::Variant1", "EnumName::Variant2"],
  "example": { ... },
  "signature": "..."
}
```

### MutationPath (new variant_path field)
```json
{
  "description": "...",
  "variant_path": [
    {"path": "relative_path", "variant": "EnumType::VariantName"}
  ],
  "examples": [ ... ],  // For enum-type paths
  "example": { ... },   // For value-type paths
  "path_info": { ... }
}
```

## Examples

### Simple enum field
```json
".enabled": {
  "description": "Mutate the enabled field of TestEnumWithSerDe",
  "example": true,
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Custom"}
  ],
  "path_info": { ... }
}
```

### Nested enum field
```json
".nested_config": {
  "description": "Mutate the nested_config field of TestEnumWithSerDe enum",
  "examples": [
    {
      "applicable_variants": ["NestedConfigEnum::Always", "NestedConfigEnum::Never"],
      "example": "Always",
      "signature": "unit"
    }
  ],
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Nested"}
  ],
  "path_info": { ... }
}
```

### Deep nested path
```json
".nested_config.0": {
  "description": "Mutate element 0 of NestedConfigEnum",
  "example": 1000000,
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Nested"},
    {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
  ],
  "path_info": { ... }
}
```

## Benefits

1. **Clear separation of concerns**: ExampleGroup variants vs MutationPath requirements
2. **Explicit path mapping**: No ambiguity about which enum at which path
3. **Scalable**: Works for arbitrary nesting depth
4. **Self-documenting**: A coding agent can clearly see the required variant chain

## Implementation Plan

### Minimal Changes Approach

The key insight is that we can reuse existing recursive example building machinery with minimal new types.

### 1. Type Changes (Minimal)

**NEW types to add to `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRequirement {
    pub description: String,
    pub example: Value,
    pub variant_path: Vec<VariantPathEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPathEntry {
    pub path: String,
    pub variant: String,
}
```

**MODIFIED type in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`:**
```rust
pub struct PathInfo {
    // ... existing fields ...
    pub path_requirement: Option<PathRequirement>, // ADD THIS FIELD
}
```

**Note:** `PathRequirement` does not need a separate `path` field since the `variant_path` array already contains path information for each variant requirement, and the `example` always shows what to set the root to.

### 2. ExampleGroup applicable_variants (Simple Fix)
- Change from short names (`"Nested"`) to full names (`"TestEnumWithSerDe::Nested"`)
- This is just a string formatting change in enum builders
- Keep as array format (no structural changes)

### 3. Variant Chain Tracking (Already Partially Exists)
- We already have `EnumContext` with variant chains in `RecursionContext`
- Enhance variant chain accumulation during recursion
- Track both the path and variant at each level: `Vec<(String, String)>`

### 4. Constrained Example Building (Reuse Existing Machinery)
The `path_requirement` example is a **constrained version** of our normal recursive example building:
- Normal building: Choose default/first variants at each enum level
- Constrained building: Choose specific variants based on variant chain

**Key insight:** The `path_requirement.example` ALWAYS shows what to mutate the **root path** with, even for deeply nested paths. The variant_path documents the complete chain, but the example shows the complete root structure needed.

**Examples:**
```json
// For .nested_config path:
"path_requirement": {
  "description": "To use this mutation path, the root must be set to TestEnumWithSerDe::Nested",
  "example": {
    "Nested": {
      "nested_config": "Always",        // ← default for nested_config
      "other_field": "Hello, World!"    // ← default for other_field
    }
  },
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Nested"}
  ]
}

// For .nested_config.0 path (deeply nested):
"path_requirement": {
  "description": "Root must be Nested AND nested_config must be Conditional",
  "example": {
    "Nested": {
      "nested_config": {"Conditional": 1000000},  // ← specific variant required
      "other_field": "Hello, World!"              // ← still need other fields
    }
  },
  "variant_path": [
    {"path": "", "variant": "TestEnumWithSerDe::Nested"},
    {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
  ]
}
```

**Implementation approach:**
1. Add optional `VariantConstraints` parameter to example building functions
2. When `VariantConstraints` is provided, choose specified variants instead of defaults
3. Reuse all existing example assembly logic
4. Always build from root, even for nested paths

### 5. MutationPath Assembly Enhancement
In `from_mutation_path_internal()`:
1. If variant chain exists, build `path_requirement`:
   - Extract variant chain: `[("", "TestEnumWithSerDe::Nested"), (".nested_config", "NestedConfigEnum::Conditional")]`
   - Build constrained root example using variant chain as constraints
   - Generate human-readable description from variant chain
   - Create `variant_path` array directly from variant chain

### 6. Example Building Logic (No Structural Changes)
Current recursive building already produces correct examples. We just need:
- **Constraint context**: When building with constraints, choose specified variants
- **Root example building**: Use constraints to build prerequisite examples
- **Reuse existing assembly**: All the complex recursive logic stays the same

### Implementation Steps
1. **File: `types.rs`** - Add `PathRequirement` and `VariantPathEntry` structs, modify `PathInfo` struct
2. **File: `new_enum_builder.rs`** - Update enum builders to use full enum names in `applicable_variants`
3. **File: `recursion_context.rs`** - Enhance variant chain tracking in `RecursionContext`
4. **File: `types.rs`** - Add constraint parameter to example building methods
5. **File: `types.rs`** - Update `from_mutation_path_internal` to generate `path_requirement` when variant chain exists

### Key Benefits
- **Minimal complexity**: Reuse 95% of existing recursive building
- **No new types**: Just add one field to existing struct
- **Leverages existing work**: Variant chain tracking already partially implemented
- **Clean separation**: Normal examples vs constrained examples use same machinery

## Key Insight

The `path_requirement` represents **prerequisite state building**: "To enable this mutation path, configure the root with this exact structure." This is fundamentally different from `applicable_variants` which represents **choice within an enum**: "This example works for any of these variants of the current enum."

The path_requirement example is just a constrained version of our existing recursive example building where we specify variant choices instead of using defaults.