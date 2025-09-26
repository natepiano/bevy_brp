# Fix for NotMutableReason Information Loss - Collaborative Execution Plan

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
   cargo build
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

### Step 1: Core Infrastructure Setup ⏳ PENDING
**Objective**: Add BuilderError enum and update trait signatures
**Files**: `mutation_path_builder/mod.rs`, `mutation_path_builder/path_builder.rs`
**Type**: ATOMIC GROUP - START (won't compile until all steps complete)

**Changes**:
- Create BuilderError enum in mod.rs
- Update PathBuilder trait methods to use BuilderError
- Set up module visibility for BuilderError

**Build Command**:
```bash
cargo build
```
**Expected**: ❌ Compilation errors (implementations don't match trait)

### Step 2: Update All Builder Implementations ⏳ PENDING
**Objective**: Update all 8 builder files to use BuilderError
**Files**: All files in `mutation_path_builder/builders/` plus `enum_path_builder.rs`
**Type**: ATOMIC GROUP - CONTINUE

**Changes**:
- Update assemble_from_children() signatures in all builders
- Convert Error returns to BuilderError::SystemError
- Add specific NotMutableReason returns where needed

**Build Command**:
```bash
cargo build
```
**Expected**: ❌ Compilation errors (boundary conversion missing)

### Step 3: Update Internal Infrastructure ⏳ PENDING
**Objective**: Update recursion_context and MutationPathBuilder
**Files**: `mutation_path_builder/recursion_context.rs`, `mutation_path_builder/builder.rs`
**Type**: ATOMIC GROUP - CONTINUE

**Changes**:
- Update require_registry_schema() to return BuilderError
- Update MutationPathBuilder's build_paths implementation
- Handle mutation status with proper NotMutableReason

**Build Command**:
```bash
cargo build
```
**Expected**: ❌ Compilation errors (boundary not complete)

### Step 4: Implement Module Boundary Conversion ⏳ PENDING
**Objective**: Complete boundary conversion in recurse_mutation_paths
**Files**: `mutation_path_builder/builder.rs`
**Type**: ATOMIC GROUP - COMPLETE

**Changes**:
- Move current recurse_mutation_paths body to build_paths_internal
- Add BuilderError to Error conversion at module boundary
- Remove all .map_err patterns that lose information

**Build Command**:
```bash
cargo build --release
```
**Expected**: ✅ Successful compilation

### Step 5: Validation and Testing ⏳ PENDING
**Objective**: Install and validate the fix
**Type**: VALIDATION

**Steps**:
1. Build and install MCP tool:
   ```bash
   cargo build --release
   cargo install --path mcp
   ```
2. User must reload: `/mcp reconnect brp`
3. User runs validation: `@.claude/commands/create_mutation_test_json.md`
4. Verify 0 differences from baseline

**Success Criteria**:
- All 145 types with correct NotMutableReason values
- No incorrect NoMutableChildren errors

## Migration Strategy

**Migration Strategy: Atomic**

This collaborative plan uses atomic implementation by necessity. The BuilderError type change affects all trait implementations and cannot be done incrementally. The Collaborative Execution Protocol above defines the atomic group boundaries with Steps 1-4 forming a single unit that must be completed together for successful compilation.

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

## Solution: Internal Error Enum

Create an internal error type that can carry both real errors and NotMutableReason, **replacing MutationResult entirely**.

The BuilderError flows through all internal functions without conversion. Only at the module's public interface do we convert BuilderError appropriately.

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

## Implementation Details

### 1. Replace MutationResult with BuilderError in `mutation_path_builder/mod.rs`:
```rust
// CURRENT CODE TO REMOVE:
pub(super) type MutationResult = Result<Vec<MutationPathInternal>, NotMutableReason>;

// ADD THIS INSTEAD:
use crate::error::Error;

#[derive(Debug)]
pub(super) enum BuilderError {
    NotMutable(NotMutableReason),
    SystemError(Error),
}

// Note: pub(super) makes it visible to the entire mutation_path_builder module
// This allows all submodules to import and use BuilderError
```

### 2. Update trait signatures in `path_builder.rs`:
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

### 3. Update ALL builder implementations to use BuilderError:

[Details for each builder file follow with specific error conversions...]

### 4. Add specific NotMutableReason returns in path_builder.rs:
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

### 5. Update recursion_context.rs to use BuilderError:
```rust
// Update require_registry_schema() to return BuilderError:
// CURRENT SIGNATURE:
pub fn require_registry_schema(&self) -> crate::error::Result<&Value> {
    // current implementation
}

// CHANGE TO:
pub fn require_registry_schema(&self) -> Result<&Value, BuilderError> {
    self.registry.get(self.type_name()).ok_or_else(|| {
        BuilderError::NotMutable(NotMutableReason::NotInRegistry(self.type_name().clone()))
    })
}
// Note: Keep return type as &Value (not &SchemaStruct which doesn't exist)
```

### 6. Update builder.rs to handle BuilderError throughout:

**IMPORTANT: MutationPathBuilder<B> struct implementation**:
```rust
// MutationPathBuilder<B> is a generic struct that implements PathBuilder trait
// CURRENT SIGNATURE:
impl<B: PathBuilder<Item = PathKind>> PathBuilder for MutationPathBuilder<B> {
    type Item = B::Item;
    type Iter<'a> = B::Iter<'a> where Self: 'a, B: 'a;

    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> MutationResult {
        // Current implementation with MutationResult
    }
}

// CHANGE TO:
impl<B: PathBuilder<Item = PathKind>> PathBuilder for MutationPathBuilder<B> {
    type Item = B::Item;
    type Iter<'a> = B::Iter<'a> where Self: 'a, B: 'a;

    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>, BuilderError> {
        // The existing recursion and registry checks stay the same
        // When calling inner.build_paths(), it now returns BuilderError (good!)

        // In the section handling mutation status:
        match Self::determine_mutation_status(&paths, ctx) {
            MutationStatus::NotMutable => {
                let reason = NotMutableReason::NoMutableChildren {
                    parent_type: ctx.type_name().clone()
                };
                Ok(vec![Self::build_not_mutable_path(ctx, reason)])
            }
            MutationStatus::PartiallyMutable => {
                let reason = NotMutableReason::from_partial_mutability(
                    ctx.type_name().clone(),
                    summaries
                );
                Ok(vec![Self::build_not_mutable_path(ctx, reason)])
            }
            MutationStatus::FullyMutable => {
                Ok(paths)
            }
        }
    }

    // The build_not_mutable_path method stays the same - it's already correct:
    fn build_not_mutable_path(ctx: &RecursionContext, reason: NotMutableReason) -> MutationPathInternal {
        // Existing implementation unchanged
    }
}
```

### 7. Update ALL collect_children() call sites in builder.rs:
Since `collect_children()` still returns `Result<Self::Iter<'_>, Error>`, we need to handle the conversion at the call sites:
```rust
// In builder.rs, in the build_example function where collect_children is called:
// CURRENT CODE:
let children = builder.collect_children(&ctx)?;  // This propagates Error

// CHANGE TO:
let children = builder.collect_children(&ctx)
    .map_err(|e| BuilderError::SystemError(e))?;  // Convert Error to BuilderError

// This pattern applies to EVERY place where collect_children() is called
// The conversion wraps the system Error in BuilderError::SystemError
```

### 8. Update enum_path_builder.rs error handling:
```rust
// Remove these 3 .map_err patterns that lose error information:

// In the function where enum variants are extracted and grouped:
extract_and_group_variants(ctx)
    .map_err(|_e| ctx.create_no_mutable_children_error())?;
// CHANGE TO:
extract_and_group_variants(ctx)?;  // Let BuilderError propagate

// Where a single variant is selected from grouped variants:
select_single_variant(&grouped_variants, ctx)
    .map_err(|_e| ctx.create_no_mutable_children_error())?;
// CHANGE TO:
select_single_variant(&grouped_variants, ctx)?;  // Let BuilderError propagate

// Where the variant example is built:
build_variant_example(&selected_variant, ctx, depth.increment())
    .map_err(|_e| ctx.create_no_mutable_children_error())?;
// CHANGE TO:
build_variant_example(&selected_variant, ctx, depth.increment())?;  // Let BuilderError propagate
```

### 9. Add From implementations for BuilderError in `mod.rs`:
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

### 10. Update module's public interface in `builder.rs`:
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

### 11. Key Pattern Examples for Builder Implementations:

**Pattern for simple error conversion (array, list, struct builders):**
```rust
// BEFORE:
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value> {
    let schema = ctx.require_registry_schema()?;
    // ... rest of implementation
}

// AFTER:
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value, BuilderError> {
    let schema = ctx.require_registry_schema()?;  // Now returns BuilderError
    // ... rest of implementation unchanged
}
```

**Pattern for NotMutableReason returns (value_builder.rs):**
```rust
// AFTER:
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    _children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value, BuilderError> {
    if !ctx.has_serialize_deserialize() {
        return Err(BuilderError::NotMutable(
            NotMutableReason::MissingSerializationTraits(ctx.type_name().clone())
        ));
    }
    // ... rest of implementation
}
```

**Pattern for complex error returns (map_builder.rs has 5 error sites):**
```rust
// BEFORE:
return Err(Error::General(format!(
    "Expected key/value in map children"
)));

// AFTER:
return Err(BuilderError::SystemError(Error::General(format!(
    "Expected key/value in map children"
))));
```

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