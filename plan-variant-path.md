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

## Design Review Skip Notes

## TYPE-SYSTEM-1: String typing violation for variant names in PathRequirement - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Type Changes
- **Issue**: The plan uses String for variant field in VariantPathEntry, but this represents structured enum variant data that should use a more type-safe approach
- **Reasoning**: This finding is a false positive. After analyzing the plan, the String approach is actually appropriate here. The variant field stores values like 'TestEnumWithSerDe::Nested' that are created by a controlled method (variant_name()) and only used as complete units for display purposes. The code shows no need to access individual type or variant components separately. Creating a separate VariantName type would add unnecessary complexity without providing real benefits - it would require extra construction/deconstruction steps while the simple String already provides the needed functionality safely through the controlled creation method.

## DESIGN-1: Significant field duplication between PathRequirement and MutationPath - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Type Changes
- **Issue**: PathRequirement duplicates description and example fields that already exist in MutationPath, violating DRY principle
- **Reasoning**: After deeper investigation, this finding is incorrect. PathRequirement serves a fundamentally different purpose than MutationPath fields. The PathRequirement.example shows the complete setup context needed BEFORE using a nested path (like showing the entire root enum structure), while MutationPath.example shows what value to send for the immediate field. The PathRequirement.description provides setup instructions ("To use this path, root must be set to X"), while MutationPath.description describes the operation ("Mutate field Y"). These are complementary, not duplicative - PathRequirement educates agents on prerequisites while MutationPath guides immediate usage.

## DESIGN-3: Unnecessary type change for variant_chain in EnumContext - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Variant Chain Tracking
- **Issue**: Plan proposes changing variant_chain type then immediately converting from the old format, adding unnecessary complexity
- **Reasoning**: After careful analysis, this finding is incorrect. The plan's approach to change variant_chain from Vec<(BrpTypeName, Vec<String>)> to Vec<VariantPathEntry> is actually necessary and correct. The key insight is that VariantPathEntry includes a path field that tracks WHERE each variant requirement applies (like "" for root or ".nested_config" for a nested field). The current format Vec<(BrpTypeName, Vec<String>)> only groups variants by type but doesn't track the path context. This path information is essential for the final PathRequirement output to show which path location requires which variant. The plan isn't adding unnecessary complexity - it's adding necessary path tracking that enables the entire feature.

## IMPLEMENTATION-1: Missing validation rules for new PathRequirement structures - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Type Changes
- **Issue**: Plan introduces new structures but doesn't specify validation rules for variant_path consistency or path format validation
- **Reasoning**: The finding misunderstands the nature of these structures. They are not user input that needs validation - they are internally constructed data structures built by our controlled code. The variant_name() method ensures correct formatting, and the path building logic in builder.rs ensures correct structure. Adding validation would be over-engineering for internally-generated data that is already guaranteed to be correct by construction.

## SIMPLIFICATION-1: Overly complex wrapper structure for simple variant path data - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Type Changes
- **Issue**: PathRequirement wrapper adds unnecessary indirection when variant_path could be added directly to existing structures
- **Reasoning**: The finding is incorrect because PathRequirement serves a specific educational purpose distinct from the main mutation path data. The PathRequirement.description explains prerequisites ("To use this path, root must be X"), while MutationPath.description explains the operation ("Mutate field Y"). The PathRequirement.example shows the complete setup context (entire root enum structure), while MutationPath.example shows the immediate field value. These three fields (description, example, variant_path) form a cohesive unit of prerequisite information that belongs together. Splitting them up by putting only variant_path in PathInfo would scatter related information and lose the semantic grouping.

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

// Use the variant_name() method consistently:
let applicable_variants = vec![ctx.type_name().variant_name(variant.name())];
// Result: ["TestEnumWithSerDe::Nested"]
```

### 3. Variant Chain Tracking

**New methods needed on BrpTypeName in `mcp/src/brp_tools/brp_type_guide/response_types.rs`**:
```rust
impl BrpTypeName {
    /// Shorten an enum type name while preserving generic parameters
    /// e.g., "core::option::Option<alloc::string::String>" → "Option<String>"
    /// e.g., "extras_plugin::TestEnumWithSerDe" → "TestEnumWithSerDe"
    pub fn short_enum_type_name(&self) -> String {
        let type_str = &self.0;

        // Find generic bracket if present
        if let Some(angle_pos) = type_str.find('<') {
            // Split into base type and generic params
            let base_type = &type_str[..angle_pos];
            let generic_part = &type_str[angle_pos..];

            // Shorten the base type
            let short_base = base_type.rsplit("::").next().unwrap_or(base_type);

            // Process generic parameters recursively
            let mut result = String::from(short_base);
            result.push('<');

            // Simple approach: shorten each :: separated segment within generics
            let inner = &generic_part[1..generic_part.len()-1]; // Remove < >
            let parts: Vec<String> = inner.split(',').map(|part| {
                let trimmed = part.trim();
                // For each type in the generic params, take the last component
                if trimmed.contains("::") {
                    trimmed.rsplit("::").next().unwrap_or(trimmed).to_string()
                } else {
                    trimmed.to_string()
                }
            }).collect();

            result.push_str(&parts.join(", "));
            result.push('>');
            result
        } else {
            // No generics, just shorten the type name
            type_str.rsplit("::").next().unwrap_or(type_str).to_string()
        }
    }

    /// Create a full variant name using the shortened enum type name
    /// e.g., "core::option::Option<String>" + "Some" → "Option<String>::Some"
    pub fn variant_name(&self, variant: &str) -> String {
        format!("{}::{}", self.short_enum_type_name(), variant)
    }
}
```

**Implementation in builder.rs `process_all_children` function (lines 256-272)**:
```rust
// Update the variant_chain construction to use VariantPathEntry
let variant_chain = match &ctx.enum_context {
    Some(super::recursion_context::EnumContext::Child {
        variant_chain: parent_chain,
    }) => {
        // We're already in a variant - extend the chain
        let mut extended = parent_chain.clone();
        for variant in &variants {
            extended.push(VariantPathEntry {
                path: ctx.mutation_path.clone(),
                variant: ctx.type_name().variant_name(variant),
            });
        }
        extended
    }
    _ => {
        // Start a new chain
        variants.iter().map(|variant| VariantPathEntry {
            path: ctx.mutation_path.clone(),
            variant: ctx.type_name().variant_name(variant),
        }).collect()
    }
};

child_ctx.enum_context =
    Some(super::recursion_context::EnumContext::Child { variant_chain });
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
2. **File: `mcp/src/brp_tools/brp_type_guide/response_types.rs`** - Add `variant_name()` method to `BrpTypeName`
3. **File: `enum_builder.rs`** - Update enum builders to use full enum names in `applicable_variants` (use `ctx.type_name().variant_name()` consistently)
4. **File: `recursion_context.rs`** - Change `EnumContext::Child.variant_chain` from `Vec<(BrpTypeName, Vec<String>)>` to `Vec<VariantPathEntry>`
5. **File: `builder.rs`** - Modify variant chain extension to push `VariantPathEntry` structs, add `generate_variant_description()` helper, build complete `PathRequirement` in `build_mutation_path_internal_with_enum_examples()`
6. **File: `types.rs`** - Update `from_mutation_path_internal()` to copy `path_requirement` field from `MutationPathInternal` to `PathInfo`
7. **File: `enum_builder.rs`** - Update `flatten_variant_chain` function and remove `VARIANT_PATH_SEPARATOR` constant:

```rust
/// Extract variant names for ExampleGroup applicable_variants field
/// NOTE: This is ONLY for ExampleGroup objects, NOT for PathRequirement.variant_path
/// PathRequirement uses VariantPathEntry structures directly
fn flatten_variant_chain(variant_chain: &[VariantPathEntry]) -> Vec<String> {
    // With the new structure, variant names are already properly formatted
    // via variant_name() when VariantPathEntry objects are created
    // Just extract them - no dot-notation joining needed anymore
    variant_chain.iter()
        .map(|entry| entry.variant.clone())
        .collect()
}
// Remove: use crate::brp_tools::brp_type_guide::constants::VARIANT_PATH_SEPARATOR;
// Remove: VARIANT_PATH_SEPARATOR constant from constants file as it's no longer needed
```
