# NotMutableReason Error Handling Refactor Plan

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for user confirmation ("go ahead" or similar)

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & VALIDATE**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt
   ```

5. **CONFIRM**: Wait for user to confirm the build succeeded

6. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

7. **PROCEED**: Move to next step only after confirmation
</Instructions>

<ExecuteImplementation>
    Find the next ⏳ PENDING step in the INTERACTIVE IMPLEMENTATION SEQUENCE below.

    For the current step:
    1. Follow the <Instructions/> above for executing the step
    2. When step is complete, use Edit tool to mark it as ✅ COMPLETED
    3. Continue to next PENDING step

    If all steps are COMPLETED:
        Display: "✅ Implementation complete! All steps have been executed."
</ExecuteImplementation>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### Step 1: Add Internal Type Infrastructure ⏳ PENDING
**Objective**: Create the foundation for internal error handling with MutationResult type alias

**Changes**:
- Add `MutationResult` type alias to `mutation_path_builder/mod.rs`
- Update `PathBuilder` trait definition in `builders/mod.rs`
- Keep `NotMutableReason` private to the module

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/mod.rs`

**Build Command**:
```bash
cargo build
```

**Expected Result**: Code compiles with new type alias available internally

### Step 2: Update All Builder Implementations ⏳ PENDING
**Objective**: Migrate all builders to use MutationResult and return NotMutableReason directly

**Changes**:
- Update all builder `build_paths` methods to return `MutationResult`
- Change error returns from `Error::NotMutable(reason).into()` to just `Err(reason)`
- Update `recurse_mutation_paths` to handle internal MutationResult
- Update `enum_path_builder::process_enum` return type

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/array_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/list_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/tuple_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/value_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Result**: All internal error handling uses MutationResult

### Step 3: Remove Public API Exposure ⏳ PENDING
**Objective**: Clean up public API by removing NotMutableReason exposure

**Changes**:
- Remove `NotMutableReason` import from `error.rs`
- Remove `Error::NotMutable` variant
- Remove `as_not_mutable()` method
- Remove public exports from module files

**Files to modify**:
- `mcp/src/error.rs`
- `mcp/src/brp_tools/mod.rs`
- `mcp/src/brp_tools/brp_type_guide/mod.rs`

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Result**: NotMutableReason is fully encapsulated

### Step 4: Validate Complete Implementation ⏳ PENDING
**Objective**: Ensure everything works correctly

**Tests to run**:
```bash
cargo nextest run
```

**Validation checklist**:
- [ ] All tests pass
- [ ] NotMutableReason is not accessible outside mutation_path_builder
- [ ] Error messages are still meaningful
- [ ] No compilation warnings

**Expected Result**: Full test suite passes with refactored error handling

---

## Design Review Skip Notes
*Finding tracked here were reviewed but skipped/rejected during design review.*

## IMPLEMENTATION-GAP-1: Missing coverage for direct build_not_mutable_path calls - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Update build_not_mutable_path Access
- **Issue**: Plan shows updating lines 453-454 and 465-466 where build_not_mutable_path is called directly, but doesn't explain how these calls will work with the new private NotMutableError type. The examples shown assume direct access to NotMutableReason variants.
- **Reasoning**: The finding is incorrect because it misunderstands Rust module privacy. Making NotMutableReason private to the mutation_path_builder module (changing from 'pub use' to 'use') does not affect internal access within the module. The direct calls to build_not_mutable_path in check_depth_limit and check_registry will continue to work without any changes.
- **Decision**: User elected to skip this recommendation

## DESIGN-3: Inconsistent error construction patterns across plan - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Update Builder Implementations
- **Issue**: Plan shows different error construction patterns: some use `error_stack::Report::new(NotMutableError(...))` while others omit the full path. This inconsistency will lead to implementation confusion.
- **Reasoning**: The plan has been amended to use the Result-based approach, which naturally enforces consistent error construction. All builders now return MutationResult and construct errors the same way: Err(NotMutableReason::...). The inconsistency issue is resolved.
- **Decision**: User elected to skip this recommendation

## Overview
Refactor `NotMutableReason` to be completely internal to the mutation_path_builder module using a Result-based type alias approach, removing it from the public Error enum while maintaining compile-time type safety.

## Current Problem
- `NotMutableReason` is exposed in the public `Error` enum
- Internal mutation path building details leak into the public API
- The `as_not_mutable()` method on Error exposes implementation details

## Solution: Result-based Type Alias Approach

### Core Design
1. Create an internal `MutationResult` type alias within mutation_path_builder
2. All internal builders return `MutationResult` instead of public `Result`
3. Convert `NotMutableReason` to mutation paths at the module boundary
4. Remove `Error::NotMutable` variant from public enum
5. Remove `as_not_mutable()` method from Error

### Benefits
- **Complete Encapsulation**: NotMutableReason stays entirely within mutation_path_builder
- **Compile-Time Type Safety**: No runtime downcasting, pure compile-time guarantees
- **Clean Public API**: No mutation path building concepts in public Error enum
- **Simple Implementation**: Standard Rust Result pattern matching
- **Zero Runtime Overhead**: Pure compile-time abstraction

## Implementation Details

### 1. Add Internal Type Alias
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

```rust
// Make NotMutableReason private to this module
use not_mutable_reason::NotMutableReason;

// Internal result type for mutation path building
// This type alias is used by all internal builders
pub(super) type MutationResult = std::result::Result<Vec<MutationPathInternal>, NotMutableReason>;
```

### 2. Update Builder Trait Definition
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/mod.rs`

Update the `PathBuilder` trait to use `MutationResult`:
```rust
pub(crate) trait PathBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> MutationResult;  // Changed from Result<Vec<MutationPathInternal>>
}
```

### 3. Update Builder Implementations
**Files to update**:
- `builders/array_builder.rs`
- `builders/list_builder.rs`
- `builders/map_builder.rs`
- `builders/set_builder.rs`
- `builders/struct_builder.rs`
- `builders/tuple_builder.rs`
- `builders/value_builder.rs`
- `builders/enum_builder.rs`
- `path_builder.rs` (has ComplexCollectionKey error)
- `recursion_context.rs` (has NotInRegistry error in require_registry_schema)

**Change pattern for all builders**:
```rust
// OLD: Return Error::NotMutable wrapped in Result
return Err(Error::NotMutable(NotMutableReason::SomeReason { ... }).into());

// NEW: Return NotMutableReason directly
return Err(NotMutableReason::SomeReason { ... });
```

**Specific examples**:

In `value_builder.rs`:
```rust
impl PathBuilder for ValueMutationBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> MutationResult {
        // In serialization trait check:
        if !has_serialize {
            return Err(NotMutableReason::MissingSerializationTraits(
                ctx.type_name().clone()
            ));
        }

        // In the fallback case when no example is available:
        Err(NotMutableReason::NoExampleAvailable(
            ctx.type_name().clone()
        ))
    }
}
```

In `tuple_builder.rs`:
```rust
impl PathBuilder for TupleMutationBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> MutationResult {
        // In tuple handle validation section:
        if elements.len() == 1 && elements[0].is_handle() {
            return Err(NotMutableReason::NonMutableHandle {
                container_type: ctx.type_name().clone(),
                element_type: elements[0].clone(),
            });
        }
        // ... rest of implementation
    }
}
```

In `path_builder.rs`:
```rust
// In build_paths method, complex type check:
if element.is_complex_type() {
    return Err(NotMutableReason::ComplexCollectionKey(
        ctx.type_name().clone(),
    ));
}
```

In `recursion_context.rs`:
```rust
// In require_registry_schema method:
self.registry.get(self.type_name()).ok_or_else(|| {
    NotMutableReason::NotInRegistry(self.type_name().clone())
})
```

### 4. Update Error Handling at Module Boundary
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

Update the `recurse_mutation_paths` function to handle `MutationResult` internally and convert at the boundary:
```rust
pub fn recurse_mutation_paths(
    type_kind: TypeKind,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<Vec<MutationPathInternal>> {  // Public Result type

    // Check depth and registry first (these remain unchanged)
    if let Some(result) = check_depth_limit(ctx, depth) {
        return result;
    }

    if let Some(result) = check_registry(ctx) {
        return result;
    }

    // Internal processing returns MutationResult
    let internal_result: MutationResult = match type_kind {
        TypeKind::Enum => enum_path_builder::process_enum(ctx, depth),
        TypeKind::Struct => MutationPathBuilder::new(StructMutationBuilder).build_paths(ctx, depth),
        TypeKind::Tuple => MutationPathBuilder::new(TupleMutationBuilder).build_paths(ctx, depth),
        TypeKind::Array => MutationPathBuilder::new(ArrayMutationBuilder).build_paths(ctx, depth),
        TypeKind::List => MutationPathBuilder::new(ListMutationBuilder).build_paths(ctx, depth),
        TypeKind::Map => MutationPathBuilder::new(MapMutationBuilder).build_paths(ctx, depth),
        TypeKind::Set => MutationPathBuilder::new(SetMutationBuilder).build_paths(ctx, depth),
        TypeKind::Value => MutationPathBuilder::new(ValueMutationBuilder).build_paths(ctx, depth),
    };

    // Convert at the boundary - compile-time safe!
    match internal_result {
        Ok(paths) => Ok(paths),
        Err(reason) => {
            // Convert NotMutableReason to mutation path internally
            Ok(vec![
                MutationPathBuilder::<ValueMutationBuilder>::build_not_mutable_path(ctx, reason)
            ])
        }
    }
}
```

### 5. Update enum_path_builder
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Update the `process_enum` function to return `MutationResult`:
```rust
pub(super) fn process_enum(
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> MutationResult {  // Changed from Result<Vec<MutationPathInternal>>
    // Implementation remains the same, just returns MutationResult
    // Any NotMutableReason errors are returned directly without wrapping
}
```

### 6. Keep Direct build_not_mutable_path Calls Unchanged
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

The `check_depth_limit` and `check_registry` functions remain unchanged since `NotMutableReason` is still accessible within the module:

```rust
// In builder.rs, within check_depth_limit function (no changes needed):
fn check_depth_limit(ctx: &RecursionContext, depth: RecursionDepth) -> Option<Result<Vec<MutationPathInternal>>> {
    if depth.exceeds_limit() {
        Some(Ok(vec![Self::build_not_mutable_path(
            ctx,
            NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
        )]))
    } else {
        None
    }
}

// In builder.rs, within check_registry function (no changes needed):
fn check_registry(ctx: &RecursionContext) -> Option<Result<Vec<MutationPathInternal>>> {
    if ctx.require_registry_schema().is_err() {
        Some(Ok(vec![Self::build_not_mutable_path(
            ctx,
            NotMutableReason::NotInRegistry(ctx.type_name().clone()),
        )]))
    } else {
        None
    }
}
```

### 7. Clean Up Public Error Enum
**Location**: `mcp/src/error.rs`

Remove:
```rust
// Remove NotMutableReason import from module imports:
use crate::brp_tools::NotMutableReason;

// Remove NotMutable variant from Error enum:
#[error("Type cannot be mutated: {0}")]
NotMutable(NotMutableReason),

// Remove from Debug impl:
Self::NotMutable(reason) => f.debug_tuple("NotMutable").field(reason).finish(),

// Remove as_not_mutable method:
pub const fn as_not_mutable(&self) -> Option<&NotMutableReason> {
    match self {
        Self::NotMutable(reason) => Some(reason),
        _ => None,
    }
}
```

### 8. Update Module Exports
**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

Remove public export:
```rust
// Remove this line:
pub use not_mutable_reason::NotMutableReason;

// Keep it internal:
use not_mutable_reason::NotMutableReason;
```

## Migration Strategy
**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Testing Strategy

1. **Existing tests should continue to pass** - the change is internal
2. **Verify NotMutable paths are still created correctly**
3. **Ensure no NotMutableReason types leak to public API**
4. **Check that error messages are still meaningful**

## Rollback Plan

If issues arise:
1. The changes are localized to mutation_path_builder module
2. Git history preserves the old approach
3. Can temporarily re-add Error::NotMutable variant if needed

## Summary

This Result-based approach achieves complete encapsulation while maintaining compile-time type safety:
- Add 1 type alias (`MutationResult`) to module
- Update trait and function signatures to use `MutationResult`
- Update builder implementations to return `NotMutableReason` directly
- Convert at single boundary point in `recurse_mutation_paths`
- Remove ~20 lines from Error enum
- No runtime downcasting or complex error handling needed

Key advantages over the original downcasting approach:
- **Compile-time type safety**: No runtime type checking
- **Zero overhead**: Pure compile-time abstraction
- **Simpler implementation**: Standard Rust Result pattern
- **Better maintainability**: Clear separation between internal and public APIs