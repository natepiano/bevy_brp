# Implementation Plan: Keep NotMutableReason Typed Until Serialization

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

### Step 1: Update Internal Types and Construction Functions ✅ COMPLETED

**Objective**: Change `MutationPathInternal.mutability_reason` from `Option<Value>` to `Option<NotMutableReason>` and update all construction function signatures and call sites in `path_builder.rs`.

**Change Type**: ATOMIC GROUP - Breaking changes (all must be done together)

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

**Implementation Details**: See Phase 1 and Phase 2 sections below

**Build Command**:
```bash
cargo build
```

**Success Criteria**: Code compiles without errors after all changes in this step are complete

---

### Step 2: Update Serialization Boundary ✅ COMPLETED

**Objective**: Add conversion from `Option<NotMutableReason>` to `Option<Value>` at the serialization boundary in `into_mutation_path_external`.

**Change Type**: Breaking (depends on Step 1)

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**Dependencies**: Requires Step 1 completion

**Implementation Details**: See Phase 3 section below

**Build Command**:
```bash
cargo build
```

**Success Criteria**: Code compiles without errors, conversion happens at correct boundary

---

### Step 3: Update Enum Builder Functions ✅ COMPLETED

**Objective**: Update `build_enum_mutability_reason` return type and `build_enum_root_path` parameter type to use `Option<NotMutableReason>`.

**Change Type**: ATOMIC GROUP - Breaking changes (function signatures must be updated together)

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Dependencies**: Requires Step 1 completion

**Implementation Details**: See Phase 4 section below

**Build Command**:
```bash
cargo build
```

**Success Criteria**: Code compiles without errors, all three match branches handled correctly

---

### Step 4: Verification and Testing ✅ COMPLETED

**Objective**: Run comprehensive validation to ensure the refactoring works correctly and produces identical JSON output.

**Dependencies**: Requires Steps 1-3 completion

**Implementation Details**: See Phase 5 section below

**Test Commands**:
```bash
# Compile check
cargo build

# Test with type guide generation
mcp__brp__brp_launch_bevy_example --target=extras_plugin --profile=debug
mcp__brp__brp_type_guide --types='["extras_plugin::TestMixedMutabilityEnum"]'

# Validate existing mutation tests
.claude/commands/mutation_test.sh
```

**Success Criteria**:
- Code compiles without errors
- Type guide JSON output unchanged
- All mutation tests pass
- Internal code can pattern match on `NotMutableReason` variants

---

## Problem Statement

Currently, `NotMutableReason` (a strongly-typed enum) is converted to JSON (`Option<Value>`) immediately when creating `MutationPathInternal`, losing type safety throughout internal processing. This is technical debt that violates the pattern used for other fields in the same struct.

### Current Architecture (Technical Debt)

```rust
NotMutableReason (enum)
    ↓ [Converted at line 559 in path_builder.rs]
    Option<Value> (JSON)
    ↓ [Stored in MutationPathInternal]
    ↓ [Passed through all internal processing as JSON]
    ↓ [Serialized to external API]
    PathInfo.mutability_reason: Option<Value>
```

### Root Cause

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs:559`
```rust
fn build_not_mutable_path(ctx: &RecursionContext, reason: NotMutableReason) -> MutationPathInternal {
    Self::build_mutation_path_internal(
        ctx,
        PathExample::Simple(json!(null)),
        Mutability::NotMutable,
        Option::<Value>::from(&reason),  // ← CONVERTS TOO EARLY!
        None,
    )
}
```

### Why This Is Technical Debt

**Inconsistency with other fields in `MutationPathInternal`:**
```rust
pub struct MutationPathInternal {
    pub path_kind: PathKind,                    // ← Typed enum ✅
    pub mutability: Mutability,                 // ← Typed enum ✅
    pub mutability_reason: Option<Value>,       // ← JSON blob ❌
    pub example: PathExample,                   // ← Typed enum ✅
    pub enum_path_data: Option<EnumPathData>,   // ← Typed struct ✅
}
```

All other fields remain strongly typed until the serialization boundary (`into_mutation_path_external`), but `mutability_reason` is converted to JSON immediately.

### Impact

1. **Type safety loss** - Cannot pattern match on variants, no compile-time validation
2. **Harder debugging** - JSON blobs instead of pretty-printed enum variants
3. **Inconsistent patterns** - Violates "keep types until serialization" principle
4. **Future code issues** - Any code needing to inspect reasons must parse JSON instead of matching enums

---

## Solution Overview

**Keep `NotMutableReason` typed throughout internal processing, convert only at the API serialization boundary.**

---

## Phase 1: Update MutationPathInternal Type

### 1.1 Change mutability_reason field type

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs:43`

**Current:**
```rust
pub struct MutationPathInternal {
    pub example: PathExample,
    pub mutation_path: MutationPath,
    pub type_name: BrpTypeName,
    pub path_kind: PathKind,
    pub mutability: Mutability,
    pub mutability_reason: Option<Value>,  // ← JSON
    pub enum_path_data: Option<EnumPathData>,
    pub depth: usize,
    pub partial_root_examples: Option<HashMap<Vec<VariantName>, Value>>,
}
```

**Change to:**
```rust
pub struct MutationPathInternal {
    pub example: PathExample,
    pub mutation_path: MutationPath,
    pub type_name: BrpTypeName,
    pub path_kind: PathKind,
    pub mutability: Mutability,
    pub mutability_reason: Option<NotMutableReason>,  // ← Typed enum
    pub enum_path_data: Option<EnumPathData>,
    pub depth: usize,
    pub partial_root_examples: Option<HashMap<Vec<VariantName>, Value>>,
}
```

### 1.2 Add import for NotMutableReason

**File:** Same file, at top

Add to imports:
```rust
use super::not_mutable_reason::NotMutableReason;
```

---

## Phase 2: Update Construction Functions

### 2.1 Update build_mutation_path_internal signature

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs:96-112`

**Current:**
```rust
fn build_mutation_path_internal(
    ctx: &RecursionContext,
    example: PathExample,
    status: Mutability,
    mutability_reason: Option<Value>,  // ← JSON
    partial_root_examples: Option<HashMap<Vec<VariantName>, Value>>,
) -> MutationPathInternal {
```

**Change to:**
```rust
fn build_mutation_path_internal(
    ctx: &RecursionContext,
    example: PathExample,
    status: Mutability,
    mutability_reason: Option<NotMutableReason>,  // ← Typed enum
    partial_root_examples: Option<HashMap<Vec<VariantName>, Value>>,
) -> MutationPathInternal {
```

### 2.2 Update build_not_mutable_path

**File:** Same file, lines 559-570

**Current:**
```rust
fn build_not_mutable_path(
    ctx: &RecursionContext,
    reason: NotMutableReason,
) -> MutationPathInternal {
    Self::build_mutation_path_internal(
        ctx,
        PathExample::Simple(json!(null)),
        Mutability::NotMutable,
        Option::<Value>::from(&reason),  // ← Remove conversion
        None,
    )
}
```

**Change to:**
```rust
fn build_not_mutable_path(
    ctx: &RecursionContext,
    reason: NotMutableReason,
) -> MutationPathInternal {
    Self::build_mutation_path_internal(
        ctx,
        PathExample::Simple(json!(null)),
        Mutability::NotMutable,
        Some(reason),  // ← Pass enum directly
        None,
    )
}
```

### 2.3 Remove obsolete conversion at determine_parent_mutability usage

**File:** Same file, lines 123-126

After Phase 2.1 updates `build_mutation_path_internal` to accept `Option<NotMutableReason>`, the conversion at line 126 becomes obsolete.

**Current:**
```rust
// Compute parent's mutation status from children's statuses
let (parent_status, reason_enum) = determine_parent_mutability(ctx, &all_paths);

// Convert NotMutableReason to Value if present
let mutability_reason = reason_enum.as_ref().and_then(Option::<Value>::from);
```

**Change to:**
```rust
// Compute parent's mutation status from children's statuses
let (parent_status, mutability_reason) = determine_parent_mutability(ctx, &all_paths);

// Conversion removed - pass typed enum directly
```

**Explanation:**
- Delete line 126 (the conversion)
- Rename `reason_enum` to `mutability_reason` for clarity
- Pass `mutability_reason` directly to `build_mutation_path_internal` (which now accepts `Option<NotMutableReason>`)

### 2.4 Update other call sites

**File:** Same file, lines 126, 156-161

**Line 126 (different location - around line 580):**
```rust
// Current:
Self::build_mutation_path_internal(ctx, example, Mutability::Mutable, None, None)

// No change needed - None is None for both types
```

**Lines 156-161:**
```rust
// Current:
Self::build_mutation_path_internal(
    ctx,
    PathExample::Simple(json!(null)),
    Mutability::PartiallyMutable,
    Option::<Value>::from(&NotMutableReason::from_partial_mutability(
        ctx.type_name.clone(),
        &children,
    )),
    None,
)

// Change to:
Self::build_mutation_path_internal(
    ctx,
    PathExample::Simple(json!(null)),
    Mutability::PartiallyMutable,
    Some(NotMutableReason::from_partial_mutability(
        ctx.type_name.clone(),
        &children,
    )),
    None,
)
```

### 2.5 Add import for NotMutableReason

**File:** Same file, at top

Add to imports:
```rust
use super::not_mutable_reason::NotMutableReason;
```

---

## Phase 3: Update Serialization Boundary

### 3.1 Convert at into_mutation_path_external

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs:76-110`

This is the serialization boundary where `MutationPathInternal` becomes `MutationPathExternal` (which contains `PathInfo` for the external API).

**Current structure (lines 76-110):**
```rust
pub fn into_mutation_path_external(
    self,
    registry: &HashMap<BrpTypeName, Value>,
) -> MutationPathExternal {
    // ... path_example resolution ...

    let (enum_instructions, applicable_variants, root_example) = self.resolve_enum_data_mut();

    MutationPathExternal {
        description,
        path_info: PathInfo {
            path_kind: self.path_kind,
            type_name: self.type_name,
            type_kind,
            mutability: self.mutability,
            mutability_reason: self.mutability_reason,  // ← Currently Option<Value>
            enum_instructions,
            applicable_variants,
            root_example,
        },
        path_example,
    }
}
```

**Change to:**
```rust
pub fn into_mutation_path_external(
    self,
    registry: &HashMap<BrpTypeName, Value>,
) -> MutationPathExternal {
    // ... path_example resolution ...

    let (enum_instructions, applicable_variants, root_example) = self.resolve_enum_data_mut();

    MutationPathExternal {
        description,
        path_info: PathInfo {
            path_kind: self.path_kind,
            type_name: self.type_name,
            type_kind,
            mutability: self.mutability,
            mutability_reason: self.mutability_reason
                .as_ref()
                .and_then(Option::<Value>::from),  // ← Convert here
            enum_instructions,
            applicable_variants,
            root_example,
        },
        path_example,
    }
}
```

**Note:** The existing `From<&NotMutableReason> for Option<Value>` implementation (at `not_mutable_reason.rs:178-212`) handles the conversion.

---

## Phase 4: Update enum_builder.rs

### 4.1 Update build_enum_mutability_reason return type

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs:630-668`

Update the function return type and handle all three match branches to use `NotMutableReason` instead of raw JSON.

**Current:**
```rust
fn build_enum_mutability_reason(
    enum_mutability: Mutability,
    enum_examples: &[ExampleGroup],
    type_name: BrpTypeName,
) -> Option<Value> {
    match enum_mutability {
        Mutability::PartiallyMutable => {
            let mutability_issues: Vec<MutabilityIssue> = enum_examples
                .iter()
                .flat_map(|eg| {
                    eg.applicable_variants.iter().map(|variant| {
                        MutabilityIssue::from_variant_name(
                            variant.clone(),
                            type_name.clone(),
                            eg.mutability,
                        )
                    })
                })
                .collect();

            let message = "Some variants are mutable while others are not".to_string();

            Option::<Value>::from(&NotMutableReason::from_partial_mutability(
                type_name,
                mutability_issues,
                message,
            ))
        }
        Mutability::NotMutable => {
            // All variants are not mutable
            Some(json!({
                "message": "No variants in this enum can be mutated"
            }))
        }
        Mutability::Mutable => None,
    }
}
```

**Change to:**
```rust
fn build_enum_mutability_reason(
    enum_mutability: Mutability,
    enum_examples: &[ExampleGroup],
    type_name: BrpTypeName,
) -> Option<NotMutableReason> {
    match enum_mutability {
        Mutability::PartiallyMutable => {
            let mutability_issues: Vec<MutabilityIssue> = enum_examples
                .iter()
                .flat_map(|eg| {
                    eg.applicable_variants.iter().map(|variant| {
                        MutabilityIssue::from_variant_name(
                            variant.clone(),
                            type_name.clone(),
                            eg.mutability,
                        )
                    })
                })
                .collect();

            let message = "Some variants are mutable while others are not".to_string();

            Some(NotMutableReason::from_partial_mutability(
                type_name,
                mutability_issues,
                message,
            ))
        }
        Mutability::NotMutable => {
            // Use NoMutableChildren variant instead of raw JSON
            Some(NotMutableReason::NoMutableChildren { parent_type: type_name })
        }
        Mutability::Mutable => None,
    }
}
```

**Changes:**
1. Return type: `Option<Value>` → `Option<NotMutableReason>`
2. PartiallyMutable branch: Remove `Option::<Value>::from(&...)` wrapper, return `NotMutableReason` directly
3. NotMutable branch: Replace raw JSON with `NotMutableReason::NoMutableChildren { parent_type: type_name }`
4. Mutable branch: No change (still returns None)

**Note:** The `NotMutableReason` import already exists in this file at line 39: `use super::super::NotMutableReason;`. No additional imports needed.

### 4.2 Update build_enum_root_path signature

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs:676`

After Phase 4.1 changes `build_enum_mutability_reason` to return `Option<NotMutableReason>`, the call at line 741 will pass this typed value to `build_enum_root_path` at line 749. The parameter type must match.

**Current:**
```rust
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    enum_mutability: Mutability,
    mutability_reason: Option<Value>,  // ← Change this
) -> MutationPathInternal {
```

**Change to:**
```rust
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    enum_mutability: Mutability,
    mutability_reason: Option<NotMutableReason>,  // ← Updated parameter type
) -> MutationPathInternal {
```

**Note:** No changes needed to the function body - the parameter is passed through to the struct field which Phase 1 already updates to the correct type.

**Call site verification:** The call site at lines 740-749 requires no changes. After Phase 4.1, `build_enum_mutability_reason` returns `Option<NotMutableReason>`, which is then passed directly to `build_enum_root_path` at line 749. The types match perfectly with no conversion needed.

---

## Phase 5: Verification and Testing

### 5.1 Compile check
```bash
cargo build
```

Verify no compilation errors related to type mismatches.

### 5.2 Test with type guide generation

```bash
mcp__brp__brp_launch_bevy_example --target=extras_plugin --profile=debug
mcp__brp__brp_type_guide --types='["extras_plugin::TestMixedMutabilityEnum"]'
```

Verify:
1. `mutability_reason` field in JSON output is still correct
2. Simple string reasons appear as strings
3. PartialChildMutability reasons appear as structured objects
4. No "unknown reason" fallbacks

### 5.3 Validate existing mutation tests

```bash
.claude/commands/mutation_test.sh
```

Ensure no regressions in mutation test functionality.

---

## Expected Outcomes

### Type Safety Improvements

1. **Internal code can pattern match:**
   ```rust
   match &path.mutability_reason {
       Some(NotMutableReason::PartialChildMutability { mutable, not_mutable, .. }) => {
           // Type-safe access to structured data
       }
       Some(NotMutableReason::NoExampleAvailable(type_name)) => {
           // Type-safe access to specific reason
       }
       _ => {}
   }
   ```

2. **Compile-time validation:**
   - Refactoring `NotMutableReason` enum is type-checked
   - Cannot accidentally use wrong field names
   - IDE autocomplete works correctly

3. **Consistent with other fields:**
   - All enum/struct fields stay typed until serialization
   - Follows Rust best practices

### External API Unchanged

The JSON output to external consumers (AI agents via MCP) remains identical:
- Simple reasons: `"mutability_reason": "Type Arc has no example"`
- Complex reasons: `"mutability_reason": {"message": "...", "mutable": [...], "not_mutable": [...]}`

---

## Files Modified Summary

1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` - Change field type, update serialization
2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` - Update function signatures and call sites
3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` - Update helper function return type

### Total Estimate:
- Implementation: 30-45 minutes
- Testing/validation: 15-30 minutes
- **Total: 1-1.5 hours**

---

## Success Criteria

- [ ] Code compiles without errors
- [ ] Type guide JSON output unchanged
- [ ] Internal code can pattern match on `NotMutableReason` variants
- [ ] All mutation tests pass
- [ ] Consistent with other typed fields in `MutationPathInternal`

---

## Dependencies

None - this is a self-contained refactoring that doesn't depend on other plans.

---

## Follow-up Work

After this plan is implemented, `root-example-fix.md` can use the typed enum directly in Phase 2.1:

```rust
let reason_detail = p.mutability_reason
    .as_ref()
    .map(|reason| format!("{reason}"))
    .unwrap_or_else(|| "unknown reason".to_string());
```

Much simpler than parsing JSON!
