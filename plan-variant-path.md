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

### 1. Type Changes

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

### 2. ExampleGroup applicable_variants
**Implementation in `enum_builder.rs`:**
```rust
// Instead of:
let applicable_variants = vec![variant.name().to_string()];

// Use:
let short_type_name = ctx.type_name().short_name(); // "TestEnumWithSerDe"
let applicable_variants = vec![format!("{}::{}", short_type_name, variant.name())];
// Result: ["TestEnumWithSerDe::Nested"]
```

### 3. Variant Chain Tracking

**New method needed on BrpTypeName in `response_types.rs`**:
```rust
impl BrpTypeName {
    /// Create a full variant name using the short type name
    /// e.g., "extras_plugin::TestEnumWithSerDe" + "Nested" → "TestEnumWithSerDe::Nested"
    pub fn variant_name(&self, variant: &str) -> String {
        format!("{}::{}", self.short_name(), variant)
    }
}
```

**Implementation in builder.rs**:
```rust
// Current: extended.push((ctx.type_name().clone(), variants.to_vec()));
// Enhanced:
for variant in variants {
    let full_variant = ctx.type_name().variant_name(variant);
    enhanced_chain.push(VariantPathEntry {
        path: ctx.mutation_path.clone(),
        variant: full_variant,
    });
}
```

### 4. Path Requirement Example Building

**CHANGE 1 - Add field to `MutationPathInternal` (types.rs:68-84):**
```rust
pub struct MutationPathInternal {
    pub example: Value,
    pub enum_root_examples: Option<Vec<ExampleGroup>>,
    pub path: String,
    pub type_name: BrpTypeName,
    pub path_kind: PathKind,
    pub mutation_status: MutationStatus,
    pub mutation_status_reason: Option<Value>,
    pub path_requirement: Option<PathRequirement>,  // ← ADD THIS FIELD
}
```

**CHANGE 2 - Build and populate path_requirement in builder.rs (lines 409-417):**
```rust
// builder.rs - CHANGE TO:
fn build_mutation_path_internal_with_enum_examples(
    ctx: &RecursionContext,
    example: Value,
    enum_root_examples: Option<Vec<super::types::ExampleGroup>>,
    status: MutationStatus,
    mutation_status_reason: Option<Value>,
) -> MutationPathInternal {
    // NEW: Build complete path_requirement if variant chain exists
    let path_requirement = match &ctx.enum_context {
        Some(EnumContext::Child { variant_chain }) if !variant_chain.is_empty() => {
            Some(PathRequirement {
                description: generate_variant_description(variant_chain),
                example: example.clone(),  // Use the example we already built!
                variant_path: variant_chain.clone(),  // Already Vec<VariantPathEntry> from Step 3
            })
        }
        _ => None,
    };

    MutationPathInternal {
        path: ctx.mutation_path.clone(),
        example,
        enum_root_examples,
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutation_status: status,
        mutation_status_reason,
        path_requirement,  // ← ADD THIS
    }
}

// NEW HELPER FUNCTION to add in builder.rs:
fn generate_variant_description(variant_chain: &[VariantPathEntry]) -> String {
    if variant_chain.len() == 1 {
        format!("To use this mutation path, the root must be set to {}",
                variant_chain[0].variant)
    } else {
        let requirements: Vec<String> = variant_chain.iter()
            .map(|entry| {
                if entry.path.is_empty() {
                    format!("root must be set to {}", entry.variant)
                } else {
                    format!("{} must be set to {}", entry.path, entry.variant)
                }
            })
            .collect();
        format!("To use this mutation path, {}", requirements.join(" and "))
    }
}
```

**CHANGE 3 - Copy field in `from_mutation_path_internal()` (types.rs:156-191):**
```rust
pub fn from_mutation_path_internal(
    path: &MutationPathInternal,
    registry: &HashMap<BrpTypeName, Value>,
) -> Self {
    // ... existing code for type_kind, description, etc. ...

    Self {
        description,
        path_info: PathInfo {
            path_kind: path.path_kind.clone(),
            type_name: path.type_name.clone(),
            type_kind,
            mutation_status: path.mutation_status,
            mutation_status_reason: path.mutation_status_reason.clone(),
            path_requirement: path.path_requirement.clone(),  // ← ADD THIS
        },
        examples: /*...*/,
        example: /*...*/,
        note: None,
    }
}
```

### Implementation Steps
1. **File: `types.rs`** - Add `PathRequirement` and `VariantPathEntry` structs, modify `PathInfo` struct, add `path_requirement` field to `MutationPathInternal`
2. **File: `response_types.rs`** - Add `variant_name()` method to `BrpTypeName`
3. **File: `enum_builder.rs`** - Update enum builders to use full enum names in `applicable_variants` (use `BrpTypeName::short_name()` and format)
4. **File: `recursion_context.rs`** - Change `EnumContext::Child.variant_chain` from `Vec<(BrpTypeName, Vec<String>)>` to `Vec<VariantPathEntry>`
5. **File: `builder.rs`** - Modify variant chain extension to push `VariantPathEntry` structs, add `generate_variant_description()` helper, build complete `PathRequirement` in `build_mutation_path_internal_with_enum_examples()`
6. **File: `types.rs`** - Update `from_mutation_path_internal()` to copy `path_requirement` field from `MutationPathInternal` to `PathInfo`
