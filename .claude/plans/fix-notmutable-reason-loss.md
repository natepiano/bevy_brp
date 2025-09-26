# Fix for NotMutableReason Information Loss

## Design Review Skip Notes

### TYPE-SYSTEM-1: Excellent Type-Driven Error Handling Design - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Solution Options - Option 1: Internal Error Enum
- **Issue**: The proposed `BuilderError` enum correctly replaces the problematic string-based error handling pattern. The current `.map_err(|_e| ctx.create_no_mutable_children_error())` pattern loses semantic information, and the `BuilderError` enum with `NotMutable(NotMutableReason)` and `SystemError(Error)` variants provides proper type-driven error distinction.
- **Reasoning**: The finding is incorrect because it treats the proposed `BuilderError` enum as if it's already implemented and working, when it's actually just a plan for future changes. The current code still uses the `MutationResult` type alias approach, not the enum structure being praised.
- **Decision**: User elected to skip this recommendation

### TYPE-SYSTEM-2: Trait Method Signatures Correctly Address Error Information Loss - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Option 1: Internal Error Enum - trait method changes
- **Issue**: The current trait methods `assemble_from_children()` and `check_collection_element_complexity()` return `Result<T, Error>` which cannot carry `NotMutableReason`. The proposed change to `Result<T, BuilderError>` correctly enables specific error preservation.
- **Reasoning**: The finding correctly identifies the problem and the right solution, but this solution already exists in the plan with complete detail including the `BuilderError` enum definition, conversion implementations, and step-by-step implementation instructions.
- **Decision**: User elected to skip this recommendation

## Intent
Restore the specific NotMutableReason error information that was lost in commit `c67aa67` (first bad commit identified via git bisect). The goal is to match the behavior of commit `c13c671` (last known good commit) where mutation test JSON generation produced the expected baseline with 145 types and proper semantic error reasons.

**Key architectural principle**: Keep `NotMutableReason` as an internal control-flow mechanism within the `mutation_path_builder` module. It should NOT leak back to `mcp/src/error.rs` (where it lived in the good commit). Instead, we handle all NotMutableReason situations fluently within our module, converting them to appropriate output, while still propagating real system errors up to callers outside the module.

## Migration Strategy

This plan follows an **Atomic Migration** approach. All changes must be implemented together as a single unit because:

1. **Breaking Changes**: Replacing `MutationResult` with `BuilderError` changes function signatures throughout the module
2. **Interdependent Updates**: Error handling changes are tightly coupled - partial implementation would leave the module in a broken state
3. **Type System Integration**: The new `BuilderError` enum must be consistently used across all trait methods and implementations

The migration cannot be done incrementally while maintaining module functionality. All files in the `mutation_path_builder` module must be updated simultaneously.

## Comprehensive Change Summary

### Files to Modify (12 files total):

1. **mutation_path_builder/mod.rs**
   - Remove `MutationResult` type alias
   - Add `BuilderError` enum with From implementations

2. **mutation_path_builder/builder.rs** (contains the module boundary)
   - Keep `recurse_mutation_paths` as public interface with same signature
   - Move its body to new `build_paths_internal` function with BuilderError return
   - Add BuilderError to success conversion in `recurse_mutation_paths`
   - Update all internal error handling to use BuilderError

3. **mutation_path_builder/path_builder.rs** (trait definition)
   - Change `assemble_from_children()` return type: `Result<Value>` → `Result<Value, BuilderError>`
   - Change `check_collection_element_complexity()` return type: `Result<()>` → `Result<(), BuilderError>`
   - Update default implementations to use BuilderError

3. **mutation_path_builder/builder.rs**
   - Change `build_paths()` return type to use BuilderError
   - Convert `collect_children()` errors: `.map_err(|e| BuilderError::SystemError(e))?`
   - Remove ALL `.map_err(|_e| ctx.create_no_mutable_children_error())` patterns
   - Handle NotMutableReason cases properly

4. **mutation_path_builder/recursion_context.rs**
   - Change `require_registry_schema()` return type to BuilderError

5. **mutation_path_builder/builders/array_builder.rs**
   - Update `assemble_from_children()` signature
   - Convert Error returns to `BuilderError::SystemError(Error)`

6. **mutation_path_builder/builders/list_builder.rs**
   - Update `assemble_from_children()` signature
   - Convert Error returns to `BuilderError::SystemError(Error)`

7. **mutation_path_builder/builders/map_builder.rs**
   - Update `assemble_from_children()` signature
   - Convert all 5 Error returns to `BuilderError::SystemError(Error)`

8. **mutation_path_builder/builders/set_builder.rs**
   - Update `assemble_from_children()` signature
   - Convert both Error returns to `BuilderError::SystemError(Error)`

9. **mutation_path_builder/builders/struct_builder.rs**
   - Update `assemble_from_children()` signature
   - Convert Error returns to `BuilderError::SystemError(Error)`

10. **mutation_path_builder/builders/tuple_builder.rs**
    - Update `assemble_from_children()` signature
    - Convert schema errors to `BuilderError::SystemError(Error)`
    - Add `BuilderError::NotMutable(NonMutableHandle)` for handle check

11. **mutation_path_builder/builders/value_builder.rs**
    - Update `assemble_from_children()` signature
    - Return `BuilderError::NotMutable(MissingSerializationTraits)`
    - Return `BuilderError::NotMutable(NoExampleAvailable)`

12. **mutation_path_builder/enum_path_builder.rs**
    - Remove 3 `.map_err(|_e| ctx.create_no_mutable_children_error())` patterns
    - Let errors propagate naturally as BuilderError

### Error Conversion Pattern:
- All existing `Err(Error::...)` → `Err(BuilderError::SystemError(Error::...))`
- Specific mutation failures → `Err(BuilderError::NotMutable(NotMutableReason::...))`
- At module boundary: `BuilderError::NotMutable` → success with NotMutable status
- At module boundary: `BuilderError::SystemError` → propagate as Error

## Problem
After commit `c67aa67` ("refactor: contain NotMutableReason within mutation_path_builder module using MutationResult"), we have 140 unexpected changes in mutation test output. Investigation revealed that specific error reasons are being lost:
- 75 cases: `MissingSerializationTraits` → `NoMutableChildren`
- 5 cases: `ComplexCollectionKey` → `NoMutableChildren`
- Other cases of lost semantic information

Methods like `assemble_from_children` and `check_collection_element_complexity` return `Result<T, Error>` but need to communicate `NotMutableReason`. Currently they return generic errors that get converted to `NoMutableChildren`, losing semantic information.

## Root Cause
The commit `c67aa67` tried to contain `NotMutableReason` within the mutation_path_builder module but created a mixed error system:
1. `build_paths` returns `MutationResult` (can return `NotMutableReason`)
2. Other trait methods return `Result<T, Error>` (can only return `Error`)
3. The `Error::NotMutable(NotMutableReason)` variant was REMOVED from the Error enum
4. Errors from #2 get blindly converted to `NoMutableChildren` via `.map_err(|_e| ctx.create_no_mutable_children_error())`

Key change: In the working commit, `Error` enum had a `NotMutable(NotMutableReason)` variant that allowed trait methods to return specific mutation failure reasons through the general Error type.

## Solution Options

### Option 1: Internal Error Enum (RECOMMENDED)
Create an internal error type that can carry both real errors and NotMutableReason, **replacing MutationResult entirely**:

```rust
// In mutation_path_builder/mod.rs
// REMOVE: pub type MutationResult = Result<Vec<MutationPathInternal>, NotMutableReason>;
// ADD:
pub(super) enum BuilderError {
    NotMutable(NotMutableReason),
    SystemError(Error),
}

impl From<Error> for BuilderError {
    fn from(e: Error) -> Self {
        BuilderError::SystemError(e)
    }
}

impl From<NotMutableReason> for BuilderError {
    fn from(reason: NotMutableReason) -> Self {
        BuilderError::NotMutable(reason)
    }
}

// Use BuilderError everywhere, including build_paths:
// In builder.rs:
pub(super) fn build_paths(
    type_info: &TypeInfo,
    schema_access: &SchemaAccess,
    knowledge: &TypeKnowledge,
) -> Result<Vec<MutationPathInternal>, BuilderError>

// Change trait methods:
fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<MutationPathDescriptor, Value>)
    -> Result<Value, BuilderError>;

fn check_collection_element_complexity(&self, element: &Value, ctx: &RecursionContext)
    -> Result<(), BuilderError>;
```

Then in builder.rs, when calling methods that return BuilderError:
```rust
// Methods now return BuilderError directly, no conversion needed internally
let result = some_builder.assemble_from_children(ctx, children)?;
```

The BuilderError flows through all internal functions without conversion. Only at the module's public interface (in mod.rs) do we convert BuilderError appropriately.

### Option 2: Type Aliases for Each Return Type
Create specific result types that use NotMutableReason:

```rust
pub(super) type ValueResult = Result<Value, NotMutableReason>;
pub(super) type VoidResult = Result<(), NotMutableReason>;
pub(super) type IterResult<I> = Result<I, NotMutableReason>;

// Change trait methods:
fn assemble_from_children(...) -> ValueResult;
fn check_collection_element_complexity(...) -> VoidResult;
fn collect_children(...) -> IterResult<Self::Iter<'_>>;
```

But this loses the ability to return real system errors, unless we add NotMutableReason variants for system errors.

### Option 3: Nested Results
Use nested results to separate system errors from NotMutableReason:

```rust
type AssembleResult = Result<Result<Value, Error>, NotMutableReason>;
// Inner Result for system errors, outer for NotMutableReason
```

This is awkward to work with and requires double unwrapping.

## Implementation Plan for Option 1

1. **Replace MutationResult with BuilderError** in `mutation_path_builder/mod.rs`:
   ```rust
   // Remove:
   // pub type MutationResult = Result<Vec<MutationPathInternal>, NotMutableReason>;

   // Add:
   #[derive(Debug)]
   pub(super) enum BuilderError {
       NotMutable(NotMutableReason),
       SystemError(Error),
   }
   ```

2. **Update trait signatures in `path_builder.rs`**:
   ```rust
   // Change trait methods to use BuilderError:
   fn assemble_from_children(
       &self,
       _ctx: &RecursionContext,
       _children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {  // Changed from Result<Value>
       Ok(json!(null))
   }

   fn check_collection_element_complexity(
       &self,
       element: &Value,
       ctx: &RecursionContext,
   ) -> Result<(), BuilderError> {  // Changed from Result<()>
       // Update the default implementation to use BuilderError
       use crate::error::Error;
       use crate::json_object::JsonObjectAccess;
       if element.is_complex_type() {
           return Err(BuilderError::SystemError(Error::General(format!(
               "Complex collection key not supported for {}",
               ctx.type_name().display_name()
           ))));
       }
       Ok(())
   }
   ```

   Also update:
   - `build_paths` in `builder.rs`: Change return type from `MutationResult` to `Result<Vec<MutationPathInternal>, BuilderError>`
   - Keep `collect_children` as-is (returns `Result<Self::Iter<'_>, Error>` - doesn't need NotMutableReason)

3. **Update ALL builder implementations to use BuilderError**:

   **In `array_builder.rs`:**
   ```rust
   // collect_children() keeps returning Error (NOT BuilderError):
   fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>, Error> {
       // Existing error returns stay as-is with Error type
       return Err(Error::SchemaProcessing {
           message: format!("Failed to extract element type from schema for array: {}",
                           ctx.type_name()),
           details: None,
       });
   }

   // Update assemble_from_children() signature:
   fn assemble_from_children(
       &self,
       _ctx: &RecursionContext,
       children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Existing logic, but errors return BuilderError::SystemError(...)
   }
   ```

   **In `list_builder.rs`:**
   ```rust
   // collect_children() keeps returning Error (NOT BuilderError):
   fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>, Error> {
       // Existing error returns stay as-is with Error type
       return Err(Error::SchemaProcessing {
           message: format!("Failed to extract element type from schema for list: {}",
                           ctx.type_name()),
           details: None,
       });
   }

   // Update assemble_from_children() signature:
   fn assemble_from_children(
       &self,
       _ctx: &RecursionContext,
       children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Existing logic
   }
   ```

   **In `map_builder.rs`:**
   ```rust
   // collect_children() keeps returning Error (NOT BuilderError):
   fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>, Error> {
       // Existing error returns stay as-is with Error type
       return Err(Error::InvalidState(format!(
           "Failed to extract key type from schema for type: {}",
           ctx.type_name()
       )));
   }

   // In assemble_from_children() where required children are missing:
   return Err(BuilderError::SystemError(Error::InvalidState(format!(
       "Protocol violation: Map type {} missing required 'key' child example",
       ctx.type_name()
   ))));

   // Update assemble_from_children() signature:
   fn assemble_from_children(
       &self,
       ctx: &RecursionContext,
       children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Update all 5 error returns to use BuilderError::SystemError(...)
   }
   ```

   **In `set_builder.rs`:**
   ```rust
   // collect_children() keeps returning Error (NOT BuilderError):
   fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>, Error> {
       // Existing error returns stay as-is with Error type
       return Err(Error::InvalidState(format!(
           "Failed to extract item type from schema for type: {}",
           ctx.type_name()
       )));
   }

   // In assemble_from_children() where required 'items' child is missing:
   return Err(BuilderError::SystemError(Error::InvalidState(format!(
       "Protocol violation: Set type {} missing required 'items' child example",
       ctx.type_name()
   ))));

   // Update assemble_from_children() signature:
   fn assemble_from_children(
       &self,
       ctx: &RecursionContext,
       children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Update all error returns to use BuilderError::SystemError(...)
   }
   ```

   **In `struct_builder.rs`:**
   ```rust
   // collect_children() keeps returning Error (NOT BuilderError):
   fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>, Error> {
       // Existing error returns stay as-is with Error type
       return Err(Error::SchemaProcessing {
           message: format!("Failed to extract type for field '{}' in struct '{}'",
                           field_name, ctx.type_name()),
           details: None,
       });
   }

   // Update assemble_from_children() signature:
   fn assemble_from_children(
       &self,
       ctx: &RecursionContext,
       children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Existing logic
   }
   ```

   **In `tuple_builder.rs`:**
   ```rust
   // collect_children() keeps returning Error (NOT BuilderError):
   fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>, Error> {
       // Existing error returns stay as-is with Error type
       return Err(Error::schema_processing_for_type(
           ctx.type_name().as_str(),
           "extract_prefix_items",
           "Missing prefixItems field in schema",
           None,
       ));
   }

   // In assemble_from_children() where single-element Handle wrapper is detected:
   if elements.len() == 1 && elements[0].is_handle() {
       return Err(BuilderError::NotMutable(
           NotMutableReason::NonMutableHandle {
               container_type: ctx.type_name().clone(),
               element_type: elements[0].clone(),
           }
       ));
   }

   // Update assemble_from_children() signature:
   fn assemble_from_children(
       &self,
       ctx: &RecursionContext,
       children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Update all error returns
   }
   ```

   **In `value_builder.rs`:**
   ```rust
   // Update assemble_from_children():
   fn assemble_from_children(
       &self,
       ctx: &RecursionContext,
       _children: HashMap<MutationPathDescriptor, Value>,
   ) -> Result<Value, BuilderError> {
       // Check if this Value type has serialization support
       if !ctx.value_type_has_serialization(ctx.type_name()) {
           return Err(BuilderError::NotMutable(
               NotMutableReason::MissingSerializationTraits(ctx.type_name().clone())
           ));
       }

       // For leaf types with no children that have serialization
       Err(BuilderError::NotMutable(
           NotMutableReason::NoExampleAvailable(ctx.type_name().clone())
       ))
   }
   ```

4. **Add specific NotMutableReason returns in path_builder.rs**:
   ```rust
   // In check_collection_element_complexity() default implementation:
   fn check_collection_element_complexity(
       &self,
       element: &Value,
       ctx: &RecursionContext,
   ) -> Result<(), BuilderError> {
       use crate::json_object::JsonObjectAccess;
       if element.is_complex_type() {
           return Err(BuilderError::NotMutable(
               NotMutableReason::ComplexCollectionKey(ctx.type_name().clone())
           ));
       }
       Ok(())
   }

   ```

5. **Update recursion_context.rs to use BuilderError**:
   ```rust
   // Update require_registry_schema() to return BuilderError:
   pub fn require_registry_schema(&self) -> Result<&SchemaStruct, BuilderError> {
       self.registry.get(self.type_name()).ok_or_else(|| {
           BuilderError::NotMutable(NotMutableReason::NotInRegistry(self.type_name().clone()))
       })
   }
   ```

6. **Update builder.rs to handle BuilderError throughout**:

   **IMPORTANT: MutationPathBuilder<B> struct implementation**:
   ```rust
   // MutationPathBuilder<B> is a generic struct that implements PathBuilder trait
   // Its build_paths method also needs updating:
   impl<B: PathBuilder<Item = PathKind>> PathBuilder for MutationPathBuilder<B> {
       fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>, BuilderError> {
           // This method implementation also needs to:
           // 1. Change return type from MutationResult to Result<Vec<MutationPathInternal>, BuilderError>
           // 2. When calling inner.build_paths(), it already returns BuilderError (good!)
           // 3. The method has complex logic for handling mutation status that needs careful updating
           // 4. Uses build_not_mutable_path internally which is already a method on Self
       }

       // Note: The build_not_mutable_path method is already defined on this struct:
       fn build_not_mutable_path(ctx: &RecursionContext, reason: NotMutableReason) -> MutationPathInternal {
           // Existing implementation stays the same
       }
   }
   ```

   **Main builder.rs changes**:
   ```rust
   // The current recurse_mutation_paths function body becomes build_paths_internal
   // and changes its return type to use BuilderError:
   fn build_paths_internal(
       type_kind: TypeKind,
       ctx: &RecursionContext,
       depth: RecursionDepth,
   ) -> Result<Vec<MutationPathInternal>, BuilderError> {
       // Move current recurse_mutation_paths implementation here
       // Update to use BuilderError throughout

       // In the recursion depth check section:
       if depth.exceeds_limit() {
           return Ok(vec![MutationPathBuilder::build_not_mutable_path(
               ctx,
               NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
           )]);
       }

       // When require_registry_schema fails, it now returns BuilderError directly
       let schema = ctx.require_registry_schema()?;  // This can return NotInRegistry

       // In mutation status handling for partial mutability:
       MutationStatus::PartiallyMutable => {
           let reason = NotMutableReason::from_partial_mutability(
               ctx.type_name().clone(), summaries
           );
           // Return the not mutable path
           return Ok(vec![MutationPathBuilder::build_not_mutable_path(ctx, reason)]);
       }

       // In mutation status handling for not mutable (ONLY place NoMutableChildren is created):
       MutationStatus::NotMutable => {
           let reason = NotMutableReason::NoMutableChildren {
               parent_type: ctx.type_name().clone()
           };
           // Return the not mutable path
           return Ok(vec![MutationPathBuilder::build_not_mutable_path(ctx, reason)]);
       }

       // Remove these 2 .map_err patterns in builder.rs that lose error information:

       // Around line 96 - in the build_example function:
       let child_example = recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())
           .map_err(|_e| ctx.create_no_mutable_children_error())?;
       // CHANGE TO:
       let child_example = recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

       // Around line 102 - in the assemble_from_children call:
       builder.assemble_from_children(&ctx, assembled_children)
           .map_err(|_e| ctx.create_no_mutable_children_error())?
       // CHANGE TO:
       builder.assemble_from_children(&ctx, assembled_children)?

       // Let BuilderError flow through naturally with the `?` operator
   }
   ```

7. **Update ALL collect_children() methods that return errors**:
   Since `collect_children()` still returns `Result<Self::Iter<'_>, Error>`, we need to handle the conversion at the call sites in `builder.rs`:
   ```rust
   // In builder.rs where collect_children is called:
   let children = builder.collect_children(&ctx)
       .map_err(|e| BuilderError::SystemError(e))?;
   ```

8. **Update enum_path_builder.rs error handling**:
   ```rust
   // Remove these 3 .map_err patterns that lose error information:
   // Line ~171:
   extract_and_group_variants(ctx)
       .map_err(|_e| ctx.create_no_mutable_children_error())?;
   // CHANGE TO:
   extract_and_group_variants(ctx)?;  // Let BuilderError propagate

   // Line ~175:
   select_single_variant(&grouped_variants, ctx)
       .map_err(|_e| ctx.create_no_mutable_children_error())?;
   // CHANGE TO:
   select_single_variant(&grouped_variants, ctx)?;  // Let BuilderError propagate

   // Line ~180:
   build_variant_example(&selected_variant, ctx, depth.increment())
       .map_err(|_e| ctx.create_no_mutable_children_error())?;
   // CHANGE TO:
   build_variant_example(&selected_variant, ctx, depth.increment())?;  // Let BuilderError propagate
   ```

9. **Add From implementations for BuilderError** in `mod.rs`:
   ```rust
   impl From<Error> for BuilderError {
       fn from(e: Error) -> Self {
           BuilderError::SystemError(e)
       }
   }

   impl From<NotMutableReason> for BuilderError {
       fn from(reason: NotMutableReason) -> Self {
           BuilderError::NotMutable(reason)
       }
   }
   ```

10. **Update module's public interface** in `builder.rs`:
   ```rust
   // The public recurse_mutation_paths function is the module boundary
   // It currently returns Result<Vec<MutationPathInternal>, Error>
   // After changes, internally it will work with BuilderError but convert at the boundary:

   pub fn recurse_mutation_paths(
       type_kind: TypeKind,
       ctx: &RecursionContext,
       depth: RecursionDepth,
   ) -> Result<Vec<MutationPathInternal>, Error> {
       // Internal implementation will call build_paths which returns BuilderError
       match build_paths_internal(type_kind, ctx, depth) {
           Ok(paths) => {
               // Normal success - paths that can be mutated
               Ok(paths)
           }
           Err(BuilderError::NotMutable(reason)) => {
               // NotMutableReason is NOT an error - convert to success with NotMutable path
               // Note: Use MutationPathBuilder with appropriate type parameter (e.g., ValueMutationBuilder)
               Ok(vec![MutationPathBuilder::<ValueMutationBuilder>::build_not_mutable_path(ctx, reason)])
           }
           Err(BuilderError::SystemError(e)) => {
               // Real errors propagate to caller
               Err(e)
           }
       }
   }

   // The internal build_paths_internal function (renamed from current implementation)
   // will use BuilderError throughout:
   fn build_paths_internal(
       type_kind: TypeKind,
       ctx: &RecursionContext,
       depth: RecursionDepth,
   ) -> Result<Vec<MutationPathInternal>, BuilderError> {
       // Current implementation logic, but with BuilderError
   }
   ```
   - This maintains the architectural boundary: NotMutableReason is internal control flow, not a public error
   - The public API remains unchanged: `Result<Vec<MutationPathInternal>, Error>`

## Import Requirements

### Files needing BuilderError import only:
```rust
// In array_builder.rs, list_builder.rs, map_builder.rs, set_builder.rs, struct_builder.rs:
use super::super::BuilderError;
```

### Files needing both BuilderError and NotMutableReason:
```rust
// In tuple_builder.rs and value_builder.rs:
use super::super::{BuilderError, NotMutableReason};
```

### Files in mutation_path_builder root:
```rust
// In path_builder.rs:
use super::{BuilderError, NotMutableReason};

// In builder.rs:
use super::{BuilderError, NotMutableReason};

// In recursion_context.rs:
use super::{BuilderError, NotMutableReason};

// In enum_path_builder.rs:
// Already uses MutationResult which will be replaced, so needs:
use super::BuilderError;
```

## Expected Outcome
After this fix:
- `Arc<String>` will report `MissingSerializationTraits` instead of `NoMutableChildren`
- HashMap with complex keys will report `ComplexCollectionKey` instead of `NoMutableChildren`
- The 140 differences from baseline should be resolved
- Users get better diagnostic information about why mutations fail

## Success Criteria and Validation

**Goal**: Running `@.claude/commands/create_mutation_test_json.md` should produce **identical** mutation test JSON output to what was generated in commit `c13c671` (last known good), with all 145 types showing the correct semantic NotMutableReason values.

**Validation Process**:
1. After implementing all changes, build and install the MCP tool:
   ```bash
   cargo build --release
   cargo install --path mcp
   ```
2. User must reload the MCP server:
   - User executes: `/mcp reconnect brp`
   - This reloads the tool with our fixes
3. User executes the validation command:
   - User runs: `@.claude/commands/create_mutation_test_json.md`
   - This generates the mutation test JSON using the fixed code
4. The command will automatically compare against baseline and should show:
   - **0 differences** from the baseline
   - All 145 types with correct NotMutableReason values
   - No instances of incorrect `NoMutableChildren` for types that should have specific reasons

**Critical**: The MCP tool changes cannot be tested until the user reloads, as MCP tools run as subprocesses and continue using the old version until reloaded.