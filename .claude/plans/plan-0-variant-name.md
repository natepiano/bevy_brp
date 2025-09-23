# Plan 0: VariantName Newtype Wrapper

## Goal
Add a type-safe newtype wrapper for variant names throughout the codebase, following the established pattern used for `BrpTypeName` and `FullMutationPath`. This provides better documentation, type safety at API boundaries, and clearer intent in function signatures.

## Motivation
Currently, variant names are represented as raw `String` values throughout the codebase. While these are dynamically generated from Bevy's reflection system, wrapping them in a newtype provides:
- Clear documentation about what the string represents
- Type safety at function boundaries (can't accidentally pass wrong string)
- Consistent pattern with other newtypes in the codebase
- Single place to add variant name validation or manipulation if needed

## Implementation

### Step 1: Define VariantName Type

Add to `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` after the `FullMutationPath` implementation:

```rust
/// A variant name from a Bevy enum type (e.g., "Option<String>::Some", "Color::Srgba")
///
/// This newtype wrapper provides type safety and documentation for variant names
/// discovered through Bevy's reflection system at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantName(String);

impl From<String> for VariantName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl std::fmt::Display for VariantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### Step 2: Export VariantName from Module

Add VariantName to the public exports in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`:

```rust
pub use types::{
    MutationPath, MutationPathInternal, MutationStatus, PathAction,
    PathSignature, VariantName, FullMutationPath
};
```

### Step 3: Update PathKindWithVariants

In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`:

```rust
// Change from:
pub struct PathKindWithVariants {
    pub path: Option<PathKind>,
    pub applicable_variants: Vec<String>,
}

// To:
pub struct PathKindWithVariants {
    pub path: Option<PathKind>,
    pub applicable_variants: Vec<VariantName>,
}
```

### Step 4: Update Trait Signatures and Usage Sites

Update the MaybeVariants trait and all places that create or consume variant names:

```rust
// First, update the MaybeVariants trait in path_builder.rs:
pub trait MaybeVariants {
    /// Returns applicable variants if this is from an enum builder
    fn applicable_variants(&self) -> Option<&[VariantName]> {
        None
    }

    /// Extract the `PathKind` if there is one (`None` for unit variants)
    fn into_path_kind(self) -> Option<PathKind>;
}

// Update ALL implementors of MaybeVariants:

// 1. PathKind implementation in path_kind.rs:
impl MaybeVariants for PathKind {
    fn applicable_variants(&self) -> Option<&[VariantName]> {
        None // Regular paths have no variant information
    }
    fn into_path_kind(self) -> Option<PathKind> {
        Some(self)
    }
}

// Update the variant_name factory function to return VariantName directly:
// In mcp/src/brp_tools/brp_type_guide/brp_type_name.rs:
pub fn variant_name(&self, variant: &str) -> VariantName {
    VariantName::from(format!("{}::{}", self.short_enum_type_name(), variant))
}

// Then in collect_children(), update the local variable type:
let applicable_variants: Vec<VariantName> = variants_in_group
    .iter()
    .map(|v| ctx.type_name().variant_name(v.name()))
    .collect();

// Also update extract_variant_name() helper function in enum_builder.rs:
fn extract_variant_name(field_name: &str) -> Option<VariantName> {
    field_name
        .split("::")
        .last()
        .map(|s| VariantName::from(s.to_string()))
}

// 2. PathKindWithVariants implementation in enum_builder.rs:
impl MaybeVariants for PathKindWithVariants {
    fn applicable_variants(&self) -> Option<&[VariantName]> {
        Some(&self.applicable_variants)
    }
    fn into_path_kind(self) -> Option<PathKind> {
        self.path
    }
}

// Update builder.rs line 207 to keep VariantName throughout:
let variant_info = item.applicable_variants().map(<[VariantName]>::to_vec);

// For string comparisons in builder.rs (e.g., line 594), use PartialEq directly:
// The PartialEq trait on VariantName allows direct comparison without conversion
if let Some(examples) = enum_examples {
    entry.variant_example = examples
        .iter()
        .find(|ex| ex.applicable_variants.contains(&entry.variant))
        .map_or_else(|| current_example.clone(), |ex| ex.example.clone());
}

// For format strings that need the variant name as a string, use Display trait:
let instructions = format!("Set to variant {}", variant_name); // Display trait handles conversion

// Only convert to String at JSON serialization boundaries when absolutely necessary
```

### Step 5: Update Related Structures

In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`:

```rust
// ExampleGroup structure:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<VariantName>,  // Changed from Vec<String>
    /// Example value for this group
    pub example: Value,
    /// The variant signature as a string
    pub signature: String,
}

// VariantPath structure:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPath {
    /// The mutation path where this variant is required (e.g., `""`, `".nested_config"`)
    pub full_mutation_path: FullMutationPath,
    /// The variant name including enum type (e.g., `"TestEnumWithSerDe::Nested"`)
    #[serde(skip)]
    pub variant: VariantName,  // Changed from String
    /// Clear instruction for this step (e.g., `"Set root to TestEnumWithSerDe::Nested"`)
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub instructions: String,
    /// The exact mutation value needed for this step
    #[serde(skip_serializing_if = "Value::is_null", default)]
    pub variant_example: Value,
}

// VariantExampleData (if it exists):
#[derive(Debug, Clone)]
struct VariantExampleData {
    variant_name: VariantName,  // Changed from String
    signature: VariantSignature,
    example: Value,
}
```

## Benefits

1. **Type Safety**: Can't accidentally pass a random string where a variant name is expected
2. **Documentation**: The type name itself documents what the string represents
3. **Consistency**: Follows the established newtype pattern in the codebase
4. **Future Flexibility**: Easy to add validation or manipulation methods if needed
5. **Grep-ability**: Easy to find all variant name usage by searching for `VariantName`

## Testing

This is a pure refactoring with no behavior changes. Testing involves:
1. Compile successfully with the new type
2. Verify serialization/deserialization works correctly
3. Confirm all existing tests pass

## Migration Notes

- The `From<String>` trait makes conversion from existing strings straightforward
- Serialization derives ensure JSON output remains identical
- The minimal trait set focuses on actual usage patterns without unnecessary convenience traits

## Why This Is Better Than Raw Strings

While variant names are indeed dynamic and discovered at runtime, using a newtype:
- Makes function signatures self-documenting
- Prevents mixing variant names with other string types
- Provides a central place for any future variant name logic
- Follows Rust best practices for domain modeling

## Design Review Skip Notes

### DESIGN-1: Incomplete module exports specification - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Step 2: Export VariantName from Module
- **Issue**: The plan shows adding FullMutationPath to exports but doesn't address whether PathSignature and other types from types.rs should also be exported
- **Reasoning**: The finding is incorrect because the plan explicitly shows adding all three types (PathSignature, VariantName, FullMutationPath) in Step 2, not just FullMutationPath as claimed
- **Decision**: User elected to skip this recommendation

### DESIGN-4: Missing AsRef<str> implementation usage guidance - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Migration Notes
- **Issue**: The plan provides AsRef<str> implementation but doesn't show how to update string comparison sites like entry.variant comparisons
- **Reasoning**: The finding is incorrect because after removing AsRef<str> from the plan (following the principle of only adding traits we need), this guidance is no longer relevant. Additionally, the .contains() method works correctly with PartialEq/Eq traits without needing AsRef conversions
- **Decision**: User elected to skip this recommendation