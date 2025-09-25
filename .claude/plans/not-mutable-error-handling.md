# NotMutableReason Error Handling Refactor Plan

## Design Review Skip Notes

## TYPE-SYSTEM-1: Information Loss in Type-to-String Conversion - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: New Internal Error Type
- **Issue**: Converting TypeGuideError::NotMutable to Error::SchemaProcessing loses structured type information. Current Error::NotMutable preserves full NotMutableReason enum, but proposed conversion flattens to strings.
- **Reasoning**: The finding is incorrect because NotMutableReason errors never escape the module as errors - they are always caught internally by handle_assemble_error() and converted to valid MutationPath objects with MutationStatus::NotMutable. The plan correctly recognizes that NotMutableReason is internal information that should stay within the mutation_path_builder module.
- **Decision**: User elected to skip this recommendation

## IMPLEMENTATION-GAP-1: Missing Complete Boundary Identification - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Update boundary functions
- **Issue**: Plan only identifies recurse_mutation_paths as boundary function but doesn't systematically identify ALL functions that expose Result<T> containing NotMutableReason to external callers
- **Reasoning**: The finding is incorrect because NotMutableReason errors never propagate to TypeGuide::from_registry_schema or TypeGuideEngine::generate_response. These errors are always caught within MutationPathBuilder::build_paths() by handle_assemble_error() and converted to successful MutationPath results with NotMutable status. External functions only receive Vec<MutationPathInternal> where NotMutable cases are valid results, not errors. The plan correctly identifies that conversion is only needed within the mutation_path_builder module.
- **Decision**: User elected to skip this recommendation

## Overview
Refactor `NotMutableReason` to be internal to the TypeGuide module, preventing leakage of internal mutation path building concepts into the general error system.

## Design: Internal Error Type with Conversion

### Core Concept
- Create `TypeGuideError` inside `mutation_path_builder` module
- `NotMutableReason` remains completely internal to mutation_path_builder
- Convert to public `Error` types at module boundaries
- Remove `Error::NotMutable` variant from public error enum

### Benefits
1. **Encapsulation**: TypeGuide implementation details stay internal
2. **Clean API**: Public error enum only contains general-purpose errors
3. **Flexibility**: Internal error types can evolve without affecting public API
4. **Type Safety**: Compiler enforces boundary conversions

## Implementation

### 1. New Internal Error Type
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/type_guide_error.rs`

```rust
use error_stack::Report;
use super::NotMutableReason;
use crate::error::Error;

/// Internal error type for TypeGuide operations
pub(crate) enum TypeGuideError {
    /// Type cannot be mutated for specific reason
    NotMutable(NotMutableReason),
    /// Wrapper for general errors
    Other(Report<Error>),
}

impl From<Report<Error>> for TypeGuideError {
    fn from(err: Report<Error>) -> Self {
        TypeGuideError::Other(err)
    }
}

impl From<Error> for TypeGuideError {
    fn from(err: Error) -> Self {
        TypeGuideError::Other(Report::new(err))
    }
}

/// Convert internal TypeGuideError to public Error at module boundary
impl From<TypeGuideError> for Report<Error> {
    fn from(tge: TypeGuideError) -> Self {
        match tge {
            TypeGuideError::NotMutable(reason) => {
                // Convert to SchemaProcessing error with details
                // Note: This conversion rarely happens since NotMutableReason is usually
                // caught internally and converted to MutationPath results
                Report::new(Error::SchemaProcessing {
                    message: reason.to_string(),
                    type_name: Some(reason.get_deepest_failing_type().to_string()),
                    operation: Some("mutation_path_building".to_string()),
                    details: None,  // Display impl already provides detailed info in message
                })
            }
            TypeGuideError::Other(err) => err,
        }
    }
}

impl TypeGuideError {
    /// Helper to create NotMutable error
    pub fn not_mutable(reason: NotMutableReason) -> Self {
        TypeGuideError::NotMutable(reason)
    }
}
```

### 2. Update NotMutableReason Implementation
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/not_mutable_reason.rs`

No changes needed. The existing `get_deepest_failing_type()` and `Display` implementations provide all necessary functionality.

### 3. Update Internal Result Type Alias
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

```rust
// Add new module
mod type_guide_error;

// Internal result type for mutation path builder operations
// Use distinct name to avoid confusion with standard Result<T>
pub(crate) type TypeGuideResult<T> = Result<T, TypeGuideError>;

// Re-export for internal use only
pub(crate) use type_guide_error::TypeGuideError;

// Remove from public exports:
// pub use not_mutable_reason::NotMutableReason;  // DELETE THIS LINE
```

### 4. Update PathBuilder Trait
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

```rust
use super::type_guide_error::TypeGuideError;
use super::TypeGuideResult;

pub trait PathBuilder {
    // Change return types from Result<T> to TypeGuideResult<T>
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> TypeGuideResult<Vec<MutationPathInternal>> {
        Ok(vec![])
    }

    fn collect_children(&self, ctx: &RecursionContext) -> TypeGuideResult<Self::Iter<'_>>;

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> TypeGuideResult<Value> {
        Ok(json!(null))
    }

    fn check_collection_element_complexity(
        &self,
        element: &Value,
        ctx: &RecursionContext,
    ) -> TypeGuideResult<()> {
        use crate::json_object::JsonObjectAccess;
        if element.is_complex_type() {
            return Err(TypeGuideError::not_mutable(
                NotMutableReason::ComplexCollectionKey(ctx.type_name().clone())
            ));
        }
        Ok(())
    }
}
```

### 5. Update All Builder Implementations

**Files to update**:
- `builders/array_builder.rs`
- `builders/list_builder.rs`
- `builders/map_builder.rs`
- `builders/set_builder.rs`
- `builders/struct_builder.rs`
- `builders/tuple_builder.rs`
- `builders/value_builder.rs`

**Change pattern**:
```rust
// From:
fn assemble_from_children(...) -> Result<Value> {
    return Err(Error::NotMutable(NotMutableReason::SomeReason { ... }).into());
}

// To:
fn assemble_from_children(...) -> TypeGuideResult<Value> {
    return Err(TypeGuideError::not_mutable(NotMutableReason::SomeReason { ... }));
}
```

### 6. Update Main Builder
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

```rust
use super::type_guide_error::TypeGuideError;
use super::TypeGuideResult;

// Update all function signatures from Result<T> to TypeGuideResult<T>
// Update error creation sites

// At the public API boundary (recurse_mutation_paths function):
pub fn recurse_mutation_paths(
    type_name: &str,
    registry: &HashMap<String, Value>,
    wrapper_info: Option<&HashMap<String, Value>>,
    enum_variants: Option<&HashMap<String, String>>,
) -> Result<HashMap<String, MutationPath>> {
    // Internal processing uses TypeGuideResult
    let internal_result: TypeGuideResult<HashMap<String, MutationPathInternal>> =
        do_internal_processing()?;

    // Convert at boundary
    internal_result
        .map(|paths| convert_to_public_paths(paths))
        .map_err(|e| e.into()) // TypeGuideError -> Report<Error>
}
```

### 7. Cleanup Error.rs
**Location**: `mcp/src/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // DELETE THIS VARIANT:
    // NotMutable(#[from] mutation_path_builder::NotMutableReason),

    // Keep existing variants:
    SchemaProcessing {
        message: String,
        type_name: Option<String>,
        operation: Option<String>,
        details: Option<String>,
    },
    // ... other variants
}
```

### 8. Update Call Sites Outside TypeGuide

**Files that may reference Error::NotMutable**:
- Search for `Error::NotMutable` usage
- These should be internal to mutation_path_builder only
- Any external usage should be converted to check for SchemaProcessing errors instead

```bash
# Find external references
rg "Error::NotMutable" --glob '!**/mutation_path_builder/**'
```

**Update handle_assemble_error function**:
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Since this function is internal to mutation_path_builder, update it to work with TypeGuideError:

```rust
/// Handle errors from `assemble_from_children`, creating `NotMutatable` paths when appropriate
fn handle_assemble_error(
    ctx: &RecursionContext,
    error: TypeGuideError,  // Changed from Report<Error>
) -> TypeGuideResult<Vec<MutationPathInternal>> {
    // Check if it's a NotMutatable condition
    match error {
        TypeGuideError::NotMutable(reason) => {
            // Return a single NotMutatable path for this type
            Ok(vec![Self::build_not_mutable_path(ctx, reason)])
        }
        TypeGuideError::Other(err) => {
            // Real error - propagate it
            Err(TypeGuideError::Other(err))
        }
    }
}
```

Note: The `as_not_mutable()` method on Error will be removed, so all internal code must use pattern matching on TypeGuideError instead.

## Migration Steps

1. **Create TypeGuideError** - Add new error type and conversion impl
2. **Update type aliases** - Add BuilderResult type alias
3. **Update trait signatures** - Change PathBuilder trait to use BuilderResult
4. **Update implementations** - Change all builders to use TypeGuideError
5. **Update boundary functions** - Add conversion at public API boundaries
6. **Remove from public API** - Remove NotMutableReason from public exports
7. **Clean up Error enum** - Remove Error::NotMutable variant
8. **Test** - Ensure all tests pass with new error handling

## Testing Strategy

1. **Unit tests** - Verify internal error creation and conversion
2. **Integration tests** - Ensure TypeGuide API still returns expected errors
3. **Boundary tests** - Verify conversions happen correctly at module boundaries
4. **Regression tests** - Ensure no information is lost in error conversion

## Benefits of This Approach

1. **Encapsulation**: Internal implementation details don't leak to public API
2. **Maintainability**: Can refactor internal errors without breaking public API
3. **Type Safety**: Compiler enforces proper conversions at boundaries
4. **Clarity**: Public error enum only contains truly public error cases
5. **Flexibility**: Internal errors can be as detailed as needed without API concerns

## Potential Issues and Solutions

**Issue**: Loss of structured error information at boundary
**Solution**: Encode key information in SchemaProcessing fields (message, details)

**Issue**: More complex error handling internally
**Solution**: Type aliases and helper functions reduce boilerplate

**Issue**: Need to update many call sites
**Solution**: Systematic approach, compiler will catch all sites that need updating