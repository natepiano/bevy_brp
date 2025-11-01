# Check Knowledge Function Complexity Fix

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
   cargo build --package bevy_brp_mcp
   cargo nextest run --package bevy_brp_mcp
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

### Step 1: Add KnowledgeAction enum ✅ COMPLETED

**Objective**: Create the `KnowledgeAction` enum in `type_knowledge.rs` to represent control flow decisions based on type knowledge lookup.

**Why**: Replace the opaque tuple return type `(Option<Result<...>>, Option<Value>)` with a self-documenting enum that clearly expresses intent.

**Changes**:
- Add `KnowledgeAction` enum with 3 variants after `TypeKnowledge` definition
- Variants: `CompleteWithExample(Value)`, `UseExampleAndRecurse(Value)`, `NoHardcodedKnowledge`

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`

**Build command**:
```bash
cargo build --package bevy_brp_mcp
```

**Change type**: Additive (safe - no breaking changes)

---

### Step 2: Add check_knowledge() method ✅ COMPLETED

**Objective**: Add `check_knowledge()` method to `RecursionContext` that translates `TypeKnowledge` into `KnowledgeAction`.

**Why**: Create a single interpretation point where all builders get consistent behavior when consulting the knowledge base.

**Changes**:
- Update imports in `recursion_context.rs` to include `KnowledgeAction` and `TypeKnowledge`
- Add `check_knowledge()` method that matches on `find_knowledge()` result and returns appropriate `KnowledgeAction`

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

**Build command**:
```bash
cargo build --package bevy_brp_mcp
```

**Change type**: Additive (safe - no breaking changes)

**Dependencies**: Requires Step 1 (imports `KnowledgeAction`)

---

### Step 3: Update path_builder.rs ✅ COMPLETED

**Objective**: Update `path_builder.rs` to use the new `ctx.check_knowledge()` method and remove the old private `check_knowledge` function.

**Why**: Migrate to the new API and eliminate the opaque tuple-based implementation.

**Changes**:
- Add import for `KnowledgeAction`
- Replace lines 92-96 with new match statement using `ctx.check_knowledge()?`
- Update documentation comment at line 400
- Remove old private `check_knowledge` function (lines 582-626, 45 lines total)

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

**Build command**:
```bash
cargo build --package bevy_brp_mcp
```

**Change type**: Breaking (removes old function, but it's private)

**Dependencies**: Requires Steps 1 and 2

---

### Step 4: Update enum_path_builder.rs ✅ COMPLETED

**Objective**: Update `enum_path_builder.rs` to use the new `ctx.check_knowledge()` method, fixing two bugs in the process.

**Why**: Fix semantic bug where `TreatAsRootValue` enums incorrectly expose variant paths, and fix error suppression bug where `.ok()` silently swallows errors.

**Changes**:
- Add import for `KnowledgeAction`
- Replace lines 106-120 with new match statement using `ctx.check_knowledge()?`
- Handle `CompleteWithExample` by returning single root path immediately
- Properly propagate errors instead of silently swallowing them

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Build and test commands**:
```bash
cargo build --package bevy_brp_mcp
cargo nextest run --package bevy_brp_mcp
```

**Change type**: Breaking (changes behavior - bug fixes)

**Dependencies**: Requires Steps 1 and 2

**Bug fixes**:
1. TreatAsRootValue enums now return immediately (previously continued processing variants)
2. Errors are now propagated (previously silently swallowed)

---

### Step 5: Complete Validation ✅ COMPLETED

**Objective**: Verify all changes are correct and complete.

**Validation steps**:

1. **Search verification** - No remaining references to old function:
   ```bash
   rg "Self::check_knowledge" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/
   # Should return no matches
   ```

2. **Compilation check**:
   ```bash
   cargo build --package bevy_brp_mcp
   # Should compile without errors
   ```

3. **Unit tests**:
   ```bash
   cargo nextest run --package bevy_brp_mcp
   # All tests should pass
   ```

4. **File length check**:
   ```bash
   wc -l mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs
   # Should show 581 lines (626 - 45 deleted lines)
   ```

5. **Documentation review**:
   - Confirm line 400 comment no longer references `check_knowledge`
   - Confirm `ctx.check_knowledge()` is used consistently

6. **Enum behavioral changes verification**:
   ```bash
   rg "select_preferred_example" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs -A 2 -B 2
   # Verify fallback is in NoHardcodedKnowledge branch
   ```

7. **Error propagation verification**:
   ```bash
   rg "find_knowledge.*\.ok\(\)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/
   # Should return no matches
   ```

8. **Mutation test suite** (after unit tests pass):
   - Run comprehensive mutation test suite to validate type guide output unchanged
   - See Testing Strategy section for details

---

## Problem

The `check_knowledge` function in `path_builder.rs:577-620` has a semantically opaque return type that encodes multiple distinct control flow decisions in an unnamed tuple.

### Current Return Type
```rust
(
    Option<Result<Vec<MutationPathInternal>, BuilderError>>,  // First element
    Option<Value>,                                             // Second element
)
```

### Four Distinct Semantic Outcomes

1. **Complete immediately** (TreatAsRootValue): `(Some(Ok(vec![...])), None)`
   - Don't recurse into children, return these paths now

2. **Use example and recurse** (TeachAndRecurse): `(None, Some(example))`
   - Continue processing children but use this hardcoded example

3. **No knowledge found**: `(None, None)`
   - Continue with normal processing, assemble example from children

4. **Error**: `(Some(Err(e)), None)`
   - Propagate error from `find_knowledge()`

### Problems with Current Approach

1. **Implicit semantics**: Comment on line 607 admits: *"this is not obvious by the current return type"*

2. **Inconsistent usage across builders**:
   - `path_builder.rs:577-625` has its own `check_knowledge()` private function
   - `enum_path_builder.rs:109-114` directly calls `ctx.find_knowledge()` and extracts example
   - Other builders may have their own variations or ignore knowledge entirely

3. **Enum builder loses semantics**: In `enum_path_builder.rs:109-114`, the code treats `TreatAsRootValue` and `TeachAndRecurse` identically:
   ```rust
   let default_example = ctx
       .find_knowledge()
       .ok()              // ← Silently ignores errors!
       .flatten()
       .map(|knowledge| knowledge.example().clone())  // ← Just extracts example!
   ```
   This loses the distinction between "stop here" vs "recurse with this example"

4. **Silent error suppression**: Enum builder uses `.ok()` which swallows errors, while struct builder propagates them

5. **Tuple forces mental mapping**: Developers must remember:
   - `(Some(Ok(_)), None)` = early return
   - `(None, Some(_))` = continue with example
   - `(None, None)` = continue without example
   - `(Some(Err(_)), None)` = error

## Solution

### 1. Define `KnowledgeAction` enum in `type_knowledge.rs`

**Location**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs` (after `TypeKnowledge` enum definition)

**Why here?** This module already contains `TypeKnowledge` and is the central location for knowledge-related types. Placing `KnowledgeAction` here:
- Makes it available to all builders without circular dependencies
- Co-locates related types (knowledge domain)
- Follows the existing pattern of shared types in this module

```rust
/// Action to take based on type knowledge lookup
///
/// This enum represents the **control flow decisions** that builders should make
/// after consulting the knowledge base, distinct from `TypeKnowledge` which
/// represents the **static facts** stored in the knowledge base.
#[derive(Debug, Clone)]
pub enum KnowledgeAction {
    /// Use this example as the root value - DO NOT recurse into children
    ///
    /// Returned for `TreatAsRootValue` knowledge where the type should be treated
    /// as opaque (e.g., `Duration`, `String`, primitive wrappers).
    CompleteWithExample(Value),

    /// Use this example but CONTINUE recursing into children
    ///
    /// Returned for `TeachAndRecurse` knowledge where we want to override the
    /// example but still expose child mutation paths (e.g., struct field defaults,
    /// enum variant selection).
    UseExampleAndRecurse(Value),

    /// No hardcoded knowledge found - assemble example from children normally
    NoHardcodedKnowledge,
}
```

**Note**: Changed `Complete(Vec<MutationPathInternal>)` to `CompleteWithExample(Value)` to avoid coupling knowledge lookup with path construction. The caller constructs the path.

#### Implementation Details

**Trait Derivations:**
- `Debug`: Required for error messages and debugging
- `Clone`: Required because enum contains `Value` and usage patterns require cloning
- **NOT** `PartialEq`: `Value` comparison is complex; not needed for control flow
- **NOT** `Copy`: Cannot derive because `Value` is not `Copy`

**Value Ownership:**
The variants store **owned** `Value` objects (not references) to avoid lifetime complexity. This is consistent with `PathExample` which also contains owned `Value`, and matches the plan's implementation which clones values at creation.

**Module Visibility:**
- `KnowledgeAction` is `pub enum` but **NOT exported from `mod.rs`**
- This is an internal type used only within `mutation_path_builder`
- Follows the same pattern as `TypeKnowledge` (not in public module exports)

**Import Statements:**
No new imports needed in `type_knowledge.rs` - the file already imports `Value` at line 11.

### 2. Add `check_knowledge()` method to `RecursionContext`

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

**Why here?** Makes `check_knowledge()` universally available to all builders via the context object they already have.

**Step A: Update imports at top of file (lines 84-85)**

Consolidate the `type_knowledge` imports to include `KnowledgeAction` and `TypeKnowledge`:

```rust
// OLD (lines 84-85):
use super::super::type_knowledge::BRP_TYPE_KNOWLEDGE;
use super::super::type_knowledge::KnowledgeKey;

// NEW:
use super::super::type_knowledge::BRP_TYPE_KNOWLEDGE;
use super::super::type_knowledge::KnowledgeAction;
use super::super::type_knowledge::KnowledgeKey;
use super::super::type_knowledge::TypeKnowledge;
```

**Step B: Add method to impl block (after line 334, where `find_knowledge()` ends)**

```rust
impl RecursionContext {
    // ... existing methods ...

    /// Check knowledge and determine what action to take
    ///
    /// This is the **single interpretation point** where we translate `TypeKnowledge`
    /// (static facts) into `KnowledgeAction` (control flow decisions). All builders
    /// should use this method instead of calling `find_knowledge()` directly to ensure
    /// consistent behavior.
    pub fn check_knowledge(&self) -> Result<KnowledgeAction, BuilderError> {
        match self.find_knowledge()? {
            Some(TypeKnowledge::TreatAsRootValue { example, .. }) => {
                // Return example immediately - caller will build single root path
                Ok(KnowledgeAction::CompleteWithExample(example.clone()))
            }
            Some(TypeKnowledge::TeachAndRecurse { example }) => {
                // Use this example but continue recursing children
                Ok(KnowledgeAction::UseExampleAndRecurse(example.clone()))
            }
            None => {
                // No knowledge - proceed with normal processing
                Ok(KnowledgeAction::NoHardcodedKnowledge)
            }
        }
    }
}
```

**Note**: The `BuilderError` type is already imported at line 86, so the return type is covered.

### 3. Update `path_builder.rs` to use the new method

#### Step 3a: Add import at top of file (after line 28)

Add the `KnowledgeAction` import after the existing `TypeKnowledge` import:

```rust
// Existing at line 28:
use super::super::type_knowledge::TypeKnowledge;

// Add new line after line 28:
use super::super::type_knowledge::KnowledgeAction;
```

**Note**: Keep imports one per line (don't consolidate). Use `super::super::` path style to match existing imports.

#### Step 3b: Update the call site (lines 92-96)

**Prerequisites:** Complete Steps 1 and 2 first (`KnowledgeAction` enum and `RecursionContext::check_knowledge()` method must exist).

**Replace lines 92-96:**
```rust
// OLD:
let (knowledge_result, knowledge_example) = Self::check_knowledge(ctx);
if let Some(result) = knowledge_result {
    return result;
}
```

**With:**
```rust
// NEW:
let knowledge_example = match ctx.check_knowledge()? {
    KnowledgeAction::CompleteWithExample(example) => {
        // Build single root path and return immediately
        // Note: build_mutation_path_internal() returns MutationPathInternal,
        // so we wrap in vec![] to match build_paths() return type
        return Ok(vec![Self::build_mutation_path_internal(
            ctx,
            PathExample::Simple(example),
            Mutability::Mutable,
            None,
            None,
        )]);
    }
    KnowledgeAction::UseExampleAndRecurse(example) => Some(example),
    KnowledgeAction::NoHardcodedKnowledge => None,
};
```

**Technical Verification:**
- ✅ Error propagation: `?` operator works with `Result<KnowledgeAction, BuilderError>` - no error conversion needed
  - `ctx.check_knowledge()` returns `Result<KnowledgeAction, BuilderError>` (defined in Step 2)
  - `build_paths()` returns `Result<Vec<MutationPathInternal>, BuilderError>`
  - Error types match exactly, so `?` operator propagates errors directly without conversion
  - `BuilderError` is already imported in path_builder.rs from the parent module
- ✅ `PathExample::Simple(example)` is the correct enum variant
- ✅ `knowledge_example` variable maintains `Option<Value>` type for downstream usage (line 121)
- ✅ `Mutability::Mutable` import already exists at line 49
- ✅ `build_mutation_path_internal()` call signature matches function definition
- ✅ **Return type clarification**: `build_mutation_path_internal()` returns `MutationPathInternal` (singular), so we wrap it in `vec![]` to match `build_paths()` return type of `Vec<MutationPathInternal>`

#### Step 3c: Update documentation reference (line 400)

The `build_mutation_path_internal` documentation references the old `check_knowledge` function.

**Replace line 400:**
```rust
/// Used by `build_not_mutable_path` for `NotMutableReason`s and `check_knowledge`
```

**With:**
```rust
/// Used by `build_not_mutable_path` for `NotMutableReason`s and knowledge handling
```

#### Step 3d: Remove the private `check_knowledge` function (lines 582-626)

Delete the entire function (45 lines total). This is the last function in the file.

### 4. Update `enum_path_builder.rs` to use the new method

#### Step 4a: Add import at top of file (after line 62)

Add the `KnowledgeAction` import:

```rust
use crate::brp_tools::brp_type_guide::type_knowledge::KnowledgeAction;
```

**Note**: All required types are already imported (`Mutability`, `PathExample`, `MutationPathInternal`, `EnumPathInfo`).

#### Step 4b: Update default example selection (lines 106-120)

**Prerequisites:** Complete Steps 1 and 2 first (`KnowledgeAction` enum and `RecursionContext::check_knowledge()` method must exist).

#### Behavioral Changes

**IMPORTANT**: This change fixes two bugs in enum knowledge handling:

1. **TreatAsRootValue enums now return immediately** (previously continued processing variants)
   - Example: If an enum has `TreatAsRootValue` knowledge, it should be treated as opaque
   - Old behavior: Generated variant paths anyway (semantic bug)
   - New behavior: Returns single root path, no variant paths (correct)

2. **Errors are now propagated** (previously silently swallowed)
   - Old behavior: `.ok()` converted errors to `None`, then fell back to variant selection
   - New behavior: `?` operator propagates errors to caller for proper handling

**Fallback behavior preserved**: When no knowledge exists (`NoHardcodedKnowledge`), the code still falls back to `select_preferred_example(&enum_examples)`. This is the same as the current behavior when `.flatten()` returns `None`. The `.or_else()` in the current code is NOT a fallback for invalid knowledge examples - it's a fallback for when no knowledge entry exists at all.

**Knowledge example validation**: No validation is needed because:
- All `TypeKnowledge` variants require a `Value` in the `example` field (enforced by type system)
- All 60+ entries in `type_knowledge.rs` provide valid `Value` objects
- The only way to get "no example" is when no knowledge entry exists (handled by `NoHardcodedKnowledge`)

**Replace lines 106-120:**
```rust
// OLD - treats both knowledge types the same and silently ignores errors:
let default_example = ctx
    .find_knowledge()
    .ok()              // ← Silently ignores errors!
    .flatten()
    .map(|knowledge| knowledge.example().clone())
    .or_else(|| select_preferred_example(&enum_examples))
    .ok_or_else(|| ...)?;
```

**With:**
```rust
// NEW - properly handles TreatAsRootValue and propagates errors:
let default_example = match ctx.check_knowledge()? {
    KnowledgeAction::CompleteWithExample(example) => {
        // Enum is opaque - return single root path immediately
        // Build enum_path_info if nested in another enum
        let enum_path_data = if ctx.variant_chain.is_empty() {
            None
        } else {
            Some(EnumPathInfo {
                variant_chain:       ctx.variant_chain.clone(),
                applicable_variants: Vec::new(),
                root_example:        None,
            })
        };

        return Ok(vec![MutationPathInternal {
            example: PathExample::Simple(example),
            mutation_path: ctx.mutation_path.clone(),
            type_name: ctx.type_name().display_name(),
            path_kind: ctx.path_kind.clone(),
            mutability: Mutability::Mutable,
            mutability_reason: None,
            enum_path_info: enum_path_data,
            depth: *ctx.depth,
            partial_root_examples: None,
        }]);
    }
    KnowledgeAction::UseExampleAndRecurse(example) => {
        // Use this example but still process variants
        example
    }
    KnowledgeAction::NoHardcodedKnowledge => {
        // Use preferred example from processed variants
        select_preferred_example(&enum_examples)
            .ok_or_else(|| {
                BuilderError::SystemError(Report::new(Error::InvalidState(format!(
                    "Enum {} has no valid example: no knowledge and no mutable variants",
                    ctx.type_name()
                ))))
            })?
    }
};
```

**Implementation Note:** `MutationPathInternal` must be constructed manually because `build_mutation_path_internal()` is private to `path_builder.rs`. This follows the existing pattern at line 812-825 in `enum_path_builder.rs`.

**Field Ordering:** The fields in the struct initialization above match the declaration order in `MutationPathInternal` (example, mutation_path, type_name, path_kind, mutability, mutability_reason, enum_path_info, depth, partial_root_examples). While Rust allows any order with named field syntax, matching the declaration order improves code readability and consistency.

**Error Handling:** The `?` operator in `ctx.check_knowledge()?` works correctly because:
- `ctx.check_knowledge()` returns `Result<KnowledgeAction, BuilderError>` (defined in Step 2)
- `build_paths()` in enum_path_builder.rs returns `Result<Vec<MutationPathInternal>, BuilderError>`
- Error types match exactly, enabling direct error propagation without conversion
- This fixes the existing bug where `.ok()` silently swallowed errors

## Alternative Considered: Use `TypeKnowledge` Directly

**Why not just match on `Result<Option<TypeKnowledge>>` at each call site?**

```rust
// This would work:
match ctx.find_knowledge()? {
    Some(TypeKnowledge::TreatAsRootValue { ... }) => ...,
    Some(TypeKnowledge::TeachAndRecurse { ... }) => ...,
    None => ...,
}
```

**Critical Downside: Multiple interpretation points**

Without `KnowledgeAction`, each builder interprets `TypeKnowledge` independently:
- Struct builder might handle `TreatAsRootValue` correctly
- Enum builder might treat it the same as `TeachAndRecurse` (current bug!)
- Future builders might introduce new interpretations

The current bug in enum builder (ignoring `TreatAsRootValue` semantics) demonstrates this risk. Having a **single interpretation point** (`RecursionContext::check_knowledge()`) enforces consistent behavior across all builders.

**Other Downsides:**
1. `TypeKnowledge` represents **static facts** (storage domain), not **control flow decisions** (builder domain)
2. No explicit name for the "no knowledge" case - it's just `None`
3. Error handling inconsistency (struct builder uses `?`, enum builder uses `.ok()`)
4. Doesn't clearly convey the action: "what should I do now?"

**Benefits of `KnowledgeAction`:**
1. **Single interpretation point**: One place where knowledge→action translation happens
2. **Self-documenting**: Variant names say what to do (`CompleteWithExample` vs `UseExampleAndRecurse`)
3. **Enforced consistency**: All builders get the same interpretation of knowledge
4. **Ergonomic with `?` operator**: Clean error propagation
5. **Clear separation**: Knowledge domain vs builder action domain

## Files to Modify

### Core Changes (Required)

1. **`mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`**
   - Add `KnowledgeAction` enum after `TypeKnowledge`
   - No type alias needed (use inline `Result<KnowledgeAction, BuilderError>` in method signatures)

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`**
   - Add imports at top of file (lines 84-85): `KnowledgeAction` and `TypeKnowledge`
   - Add `check_knowledge()` method to `RecursionContext` impl block (after line 334)

3. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`**
   - Add import at top of file (after line 28): `use super::super::type_knowledge::KnowledgeAction;`
   - Update call site at lines 92-96 to use `ctx.check_knowledge()`
   - Update documentation comment at line 400 (remove `check_knowledge` reference)
   - Remove private `check_knowledge` function (lines 582-626, includes closing brace)

4. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`**
   - Add import at top of file (after line 62): `use crate::brp_tools::brp_type_guide::type_knowledge::KnowledgeAction;`
   - Update default example selection (lines 106-120) to use `ctx.check_knowledge()`
   - Handle `CompleteWithExample` by manually constructing `MutationPathInternal` with all 9 fields

### Audit Remaining Builders (Completed - No Action Needed)

The following builders have been audited and **do NOT call `find_knowledge()` directly**:
- ✅ `type_kind_builders/struct_builder.rs`
- ✅ `type_kind_builders/tuple_builder.rs`
- ✅ `type_kind_builders/array_builder.rs`
- ✅ `type_kind_builders/list_builder.rs`
- ✅ `type_kind_builders/map_builder.rs`
- ✅ `type_kind_builders/set_builder.rs`
- ✅ `type_kind_builders/value_builder.rs`

**Audit verification:**
```bash
rg "find_knowledge" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/type_kind_builders/
# Result: No matches found
```

**Conclusion:** NO ACTION NEEDED for type_kind_builders. Only `path_builder.rs` and `enum_path_builder.rs` require updates.

## Testing Strategy

This refactoring should not change type guide output for correctly-implemented types. A comprehensive mutation testing system validates that all type guides generate identical output before and after changes.

**Testing approach:**
- The existing mutation test suite (`.claude/commands/mutation_test.md`) validates 250+ Bevy types
- After implementation, run the mutation tests to verify type guide output is unchanged
- Any differences in output indicate either: (1) a bug in the implementation, or (2) a fix for previously-buggy types (e.g., enums with `TreatAsRootValue` that incorrectly exposed variant paths)

**Expected behavioral changes** (these are bug fixes):
1. Enums with `TreatAsRootValue` knowledge will now return single root paths (previously exposed variants)
2. Errors from knowledge lookup are now propagated (previously silently swallowed in enum builder)

## Benefits

### Type Safety
- Compiler enforces handling all `KnowledgeAction` variants
- Impossible to forget handling `CompleteWithExample` case

### Bug Fixes
- **Fixes enum builder bug**: Now properly distinguishes TreatAsRootValue vs TeachAndRecurse
- **Fixes error suppression**: Enum builder now propagates errors instead of silently ignoring them

### Code Quality
- **Self-documenting**: Names clearly express intent (`CompleteWithExample` vs `UseExampleAndRecurse`)
- **Maintainability**: Future developers immediately understand control flow
- **Consistency**: Single interpretation point prevents divergent behavior
- **Testability**: Each variant can be tested independently

### Architecture
- **Clear separation**: Knowledge domain (static facts) vs builder domain (control flow)
- **Single responsibility**: `TypeKnowledge` stores facts, `KnowledgeAction` directs behavior
- **Extensibility**: Easy to add new actions without modifying builders

## Migration Strategy

To make this change safely and reviewable:

1. **Step 1**: Add `KnowledgeAction` enum to `type_knowledge.rs`
2. **Step 2**: Add `check_knowledge()` method to `RecursionContext`
3. **Step 3**: Update `path_builder.rs` to use new method
4. **Step 4**: Update `enum_path_builder.rs` to use new method

Each step can be committed independently for easier review and rollback if needed.

**Note**: Step 3d removes the old `check_knowledge` function from `path_builder.rs`, so no separate cleanup step is needed. Other builders (`type_kind_builders/*`) have been audited and require no changes.

## Verification Steps

After completing the implementation, verify the changes:

### 1. Search Verification
```bash
# Ensure no remaining references to the old check_knowledge function
rg "Self::check_knowledge" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/
# Should return no matches (exit code 1)
```

### 2. Compilation Check
```bash
cd /Users/natemccoy/rust/bevy_brp
cargo build --package bevy_brp_mcp
# Should compile without errors
```

### 3. Unit Test Validation
```bash
cargo nextest run --package bevy_brp_mcp
# All unit tests should pass
```

**Note**: After unit tests pass, run the comprehensive mutation test suite (see Testing Strategy section) to validate that type guide generation is unchanged for all 250+ Bevy types.

### 4. File Length Check
```bash
wc -l mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs
# Should show 581 lines (626 - 45 deleted lines)
```

### 5. Documentation Review
- Confirm line 400 comment no longer references `check_knowledge`
- Confirm `ctx.check_knowledge()` is used consistently across all builders

### 6. Enum Behavioral Changes Verification
```bash
# Verify fallback behavior is preserved in the correct place
rg "select_preferred_example" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs -A 2 -B 2
# Should show:
# - Line ~176: Function definition
# - Line ~312: Inside NoHardcodedKnowledge match arm (NEW - correct usage)
# - Line ~546: Inside build_partial_root_examples (different use case)

# Verify no .or_else() chains remain that could mask behavioral changes
rg "\.or_else.*select_preferred_example" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/
# Should only show line ~546 in enum_path_builder.rs (build_partial_root_examples)
```

### 7. Error Propagation Verification
After implementation, verify that errors are properly propagated:
- Enum builder no longer uses `.ok()` to swallow errors from knowledge lookup
- Both `path_builder.rs` and `enum_path_builder.rs` use `ctx.check_knowledge()?` for consistent error handling
- Search for any remaining `.ok()` patterns that might suppress errors:
```bash
rg "find_knowledge.*\.ok\(\)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/
# Should return no matches (all usages should now use check_knowledge with ?)
```
