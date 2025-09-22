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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantName(String);

impl Deref for VariantName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for VariantName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for VariantName {
    fn from(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for VariantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for VariantName {
    fn as_ref(&self) -> &str {
        &self.0
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

### Step 4: Update Usage Sites

Update all places that create or consume variant names:

```rust
// In collect_children():
let applicable_variants: Vec<VariantName> = variants_in_group
    .iter()
    .map(|v| VariantName::from(ctx.type_name().variant_name(v.name())))
    .collect();

// In any place that needs the raw string:
let variant_string: &str = variant_name.as_ref();
// or with deref coercion:
let variant_string: &String = &*variant_name;
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

- The `Deref` implementation allows most existing string operations to work unchanged
- The `From` traits make conversion from existing strings straightforward
- The `AsRef<str>` trait enables passing to functions expecting string slices
- Serialization derives ensure JSON output remains identical

## Why This Is Better Than Raw Strings

While variant names are indeed dynamic and discovered at runtime, using a newtype:
- Makes function signatures self-documenting
- Prevents mixing variant names with other string types
- Provides a central place for any future variant name logic
- Follows Rust best practices for domain modeling