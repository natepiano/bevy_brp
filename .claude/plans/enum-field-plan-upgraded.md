# Plan: Consolidate Enum Fields into EnumPathData Structure

**Status**: PREREQUISITE for plan-mutation-path-root-example.md

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

### Step 1: Create EnumPathData Structure ✅ COMPLETED

**Objective**: Define new `EnumPathData` struct with all required fields and helper methods

**Change Type**: ADDITIVE (safe - no existing code affected)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes**:
1. Add new `EnumPathData` struct after the `VariantPath` struct
2. Add implementation with `new()`, `with_applicable_variants()`, and `is_empty()` helper methods

**Build Command**:
```bash
cargo build
```

**Expected Outcome**: New struct compiles successfully, no impact on existing code

---

### Step 2: Add enum_data Field to MutationPathInternal ⏳ PENDING

**Objective**: Add `enum_data: Option<EnumPathData>` field alongside existing enum fields

**Change Type**: ADDITIVE (preparation for atomic swap - both old and new fields coexist)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes**:
1. Add `pub enum_data: Option<EnumPathData>` field to `MutationPathInternal` struct
2. Keep existing `enum_instructions` and `enum_variant_path` fields (temporary coexistence)
3. Initialize `enum_data: None` in any existing constructors

**Build Command**:
```bash
cargo build
```

**Expected Outcome**: Both old and new fields exist, compiles successfully

---

### Step 3: Update All Creation Sites ⏳ PENDING

**Objective**: Migrate both builder functions to populate `enum_data` instead of old fields

**Change Type**: ATOMIC GROUP - CRITICAL (must update both sites together)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` (lines 290-309)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs` (lines 653-668)

**Changes**:

**builder.rs:288-311** - `build_mutation_path_internal()` function:
```rust
// OLD:
let (enum_instructions, enum_variant_path) = if ctx.variant_chain.is_empty() {
    (None, vec![])
} else {
    (
        enum_path_builder::generate_enum_instructions(ctx),
        ctx.variant_chain.clone(),
    )
};
MutationPathInternal {
    // ... other fields ...
    enum_instructions,
    enum_variant_path,
}

// NEW:
let enum_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain: ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        variant_chain_root_example: None,
        enum_instructions: enum_path_builder::generate_enum_instructions(ctx)
            .expect("generate_enum_instructions should return Some when variant_chain is non-empty"),
    })
};
MutationPathInternal {
    // ... other fields ...
    enum_data,
    enum_instructions: enum_data.as_ref().map(|ed| ed.enum_instructions.clone()),
    enum_variant_path: enum_data.as_ref().map(|ed| ed.variant_chain.clone()).unwrap_or_default(),
}
```

**enum_path_builder.rs:646-669** - `create_result_paths()` function:
```rust
// OLD:
let enum_instructions = generate_enum_instructions(ctx);
let enum_variant_path = populate_variant_path(ctx, &enum_examples, &default_example);
let root_mutation_path = MutationPathInternal {
    // ... other fields ...
    enum_instructions,
    enum_variant_path,
};

// NEW:
let enum_data = Some(EnumPathData {
    variant_chain: populate_variant_path(ctx, &enum_examples, &default_example),
    applicable_variants: Vec::new(),
    variant_chain_root_example: None,
    enum_instructions: generate_enum_instructions(ctx)
        .expect("generate_enum_instructions should return Some for enum paths"),
});
let root_mutation_path = MutationPathInternal {
    // ... other fields ...
    enum_data: enum_data.clone(),
    enum_instructions: enum_data.as_ref().map(|ed| ed.enum_instructions.clone()),
    enum_variant_path: enum_data.as_ref().map(|ed| ed.variant_chain.clone()).unwrap_or_default(),
};
```

**Build Command**:
```bash
cargo build
```

**Expected Outcome**: Both old and new fields are populated, compiles successfully

**Dependencies**: Requires Step 2

---

### Step 4: Update All Access Sites ⏳ PENDING

**Objective**: Change all read access from old fields to new `enum_data` field

**Change Type**: ATOMIC GROUP - CRITICAL (all reads must update together)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs` (lines 617-641)

**Changes**:

**enum_path_builder.rs:617-641** - `update_child_variant_paths()` function:
```rust
// OLD:
if !child.enum_variant_path.is_empty() {
    for entry in &mut child.enum_variant_path {
        if entry.full_mutation_path == *current_path {
            entry.instructions = format!(/* ... */);
            entry.variant_example = examples.iter().find(/* ... */);
        }
    }
}

// NEW:
if let Some(enum_data) = &mut child.enum_data {
    if !enum_data.is_empty() {
        for entry in &mut enum_data.variant_chain {
            if entry.full_mutation_path == *current_path {
                entry.instructions = format!(/* ... */);
                entry.variant_example = examples.iter().find(/* ... */);
            }
        }
    }
}
```

**Build Command**:
```bash
cargo build
```

**Expected Outcome**: Access patterns updated, compiles successfully

**Dependencies**: Requires Step 3

---

### Step 5: Update Serialization/Output ⏳ PENDING

**Objective**: Extract enum data from `enum_data` field in conversion function

**Change Type**: ATOMIC GROUP - CRITICAL (must preserve exact output format)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` (lines 343-344)

**Changes**:

**types.rs:343-344** - `MutationPath::from_mutation_path_internal()` function:
```rust
// OLD:
enum_instructions: path.enum_instructions.clone(),
enum_variant_path: path.enum_variant_path.clone(),

// NEW: Extract from EnumPathData
enum_instructions: path.enum_data.as_ref().map(|ed| ed.enum_instructions.clone()),
enum_variant_path: path.enum_data.as_ref().map(|ed| ed.variant_chain.clone()).unwrap_or_default(),
```

**Build Command**:
```bash
cargo build
```

**Expected Outcome**: Output conversion uses new field, compiles successfully, output format unchanged

**Dependencies**: Requires Step 4

**Note**: The `PathInfo` struct fields remain unchanged - this is internal-to-output conversion only

---

### Step 6: Remove Old Fields ⏳ PENDING

**Objective**: Delete deprecated `enum_instructions` and `enum_variant_path` fields from `MutationPathInternal`

**Change Type**: ATOMIC GROUP END (final cleanup)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` (lines 198, 200)

**Changes**:
1. Remove `pub enum_instructions: Option<String>` from `MutationPathInternal` (line 198)
2. Remove `pub enum_variant_path: Vec<VariantPath>` from `MutationPathInternal` (line 200)
3. Remove field assignments from Step 3 creation sites (no longer needed)

**Build Command**:
```bash
cargo build
```

**Expected Outcome**: Old fields removed, compiles successfully, no references remain

**Dependencies**: Requires Step 5

---

### Step 7: Complete Validation ⏳ PENDING

**Objective**: Run full test suite and verify behavior is unchanged

**Files to Validate**: All modified files

**Validation Steps**:
1. Run complete test suite:
   ```bash
   cargo nextest run
   ```

2. Run mutation tests specifically:
   ```bash
   cargo nextest run mutation_test
   ```

3. Verify output format unchanged (if baseline exists):
   - Check `TestVariantChainEnum` generates correct paths
   - Verify `enum_instructions` field present in output
   - Confirm `enum_variant_path` arrays match previous output

4. Check success criteria:
   - ✅ All enum-related data consolidated into `EnumPathData`
   - ✅ No references to old `enum_variant_path` field remain
   - ✅ All tests pass
   - ✅ Output format preserves enum information correctly
   - ✅ Code is cleaner and more maintainable

**Dependencies**: Requires Step 6

---

## Executive Summary

Refactor `MutationPathInternal` to consolidate all enum-related fields into a dedicated `EnumPathData` struct. This improves code organization, makes enum-specific data explicit, and prepares the codebase for enhanced enum variant chain root example tracking.

## Motivation

Currently, `MutationPathInternal` has enum-related fields scattered throughout its structure (e.g., `enum_variant_path: Vec<VariantPath>`). This makes it difficult to:
- Identify which fields are enum-specific
- Add new enum-related metadata
- Pass enum data as a cohesive unit
- Maintain clear separation between enum and non-enum path data

Consolidating into `EnumPathData` provides:
- Clear organizational structure
- Single point of access for enum-specific data
- Foundation for variant chain root example tracking
- Better type safety and API clarity

## Current State

`MutationPathInternal` currently has:
```rust
pub struct MutationPathInternal {
    // ... other fields ...
    pub enum_variant_path: Vec<VariantPath>,
    // possibly other enum-related fields
}
```

## Target State

```rust
pub struct MutationPathInternal {
    // ... other fields ...
    pub enum_data: Option<EnumPathData>,
}

pub struct EnumPathData {
    pub variant_chain: Vec<VariantPath>,
    pub applicable_variants: Vec<VariantName>,
    pub variant_chain_root_example: Option<Value>,
    pub enum_instructions: String,
}
```

## Complete Site Analysis

Before implementation, here are ALL the sites that touch `enum_instructions` and `enum_variant_path`:

**Field Definitions (Step 2 & 6)**:
- types.rs:198 - `enum_instructions: Option<String>` in MutationPathInternal → REMOVE
- types.rs:200 - `enum_variant_path: Vec<VariantPath>` in MutationPathInternal → REMOVE
- types.rs:247 - `enum_instructions: Option<String>` in PathInfo (output struct) → KEEP (no change)
- types.rs:250 - `enum_variant_path: Vec<VariantPath>` in PathInfo (output struct) → KEEP (no change)

**Creation Sites (Step 3)**:
- builder.rs:290-309 - `build_mutation_path_internal()` function
- enum_path_builder.rs:653-668 - `create_result_paths()` function

**Read/Access Sites (Step 4)**:
- enum_path_builder.rs:617-641 - `update_child_variant_paths()` function

**Serialization Sites (Step 5)**:
- types.rs:343-344 - `MutationPath::from_mutation_path_internal()` function

**String Literals (informational only - no code change needed)**:
- enum_path_builder.rs:469 - error message text contains "enum_variant_path" string

Total sites requiring code changes: **6 locations**

## Implementation Details

### Phase 1: Create EnumPathData Structure (Step 1)

**File**: `mutation_path.rs` (or appropriate module)

1. Define `EnumPathData` struct with initial fields:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EnumPathData {
    /// Chain of enum variants from root to this path with full metadata
    pub variant_chain: Vec<VariantPath>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// Complete root example for this specific variant chain
    pub variant_chain_root_example: Option<Value>,

    /// Human-readable instructions for using this enum path
    pub enum_instructions: String,
}
```

2. Add constructor and helper methods:
```rust
impl EnumPathData {
    pub fn new(variant_chain: Vec<VariantPath>, enum_instructions: String) -> Self {
        Self {
            variant_chain,
            applicable_variants: Vec::new(),
            variant_chain_root_example: None,
            enum_instructions,
        }
    }

    pub fn with_applicable_variants(mut self, variants: Vec<VariantName>) -> Self {
        self.applicable_variants = variants;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.variant_chain.is_empty()
    }
}
```

### Phase 2: Add enum_data Field (Step 2)

**File**: `mutation_path.rs`

1. Add `enum_data` field alongside existing fields:
```rust
pub struct MutationPathInternal {
    // ... existing fields ...
    pub enum_instructions: Option<String>,  // Keep temporarily
    pub enum_variant_path: Vec<VariantPath>,  // Keep temporarily
    pub enum_data: Option<EnumPathData>,  // NEW
}
```

2. Initialize in constructors:
```rust
impl MutationPathInternal {
    pub fn new(/* ... */) -> Self {
        Self {
            // ... other fields ...
            enum_data: None,
        }
    }
}
```

### Phase 3: Update Creation Sites (Step 3)

See Step 3 in INTERACTIVE IMPLEMENTATION SEQUENCE for detailed code changes.

### Phase 4: Update Access Sites (Step 4)

See Step 4 in INTERACTIVE IMPLEMENTATION SEQUENCE for detailed code changes.

Common patterns to handle:
- Checking if path has enum variants: `path.enum_data.is_some()`
- Checking if non-empty: `path.enum_data.as_ref().map_or(false, |ed| !ed.is_empty())`
- Iterating over variant chain: `enum_data.variant_chain.iter()`
- Mutable iteration: `enum_data.variant_chain.iter_mut()`

### Phase 5: Update Serialization (Step 5)

See Step 5 in INTERACTIVE IMPLEMENTATION SEQUENCE for detailed code changes.

Verify that all enum data is preserved in the conversion:
- `enum_instructions` → PathInfo field
- `variant_chain` → `enum_variant_path` field in PathInfo
- `applicable_variants` → currently not in output format (may be added by dependent plan)
- `variant_chain_root_example` → currently not in output format (will be added by dependent plan)

### Phase 6: Remove Old Fields (Step 6)

**File**: `mutation_path.rs`

1. Remove `enum_variant_path` field completely from `MutationPathInternal`
2. Remove `enum_instructions` field completely from `MutationPathInternal`
3. Remove any helper methods that only existed for old fields
4. Ensure all tests pass

### Phase 7: Testing and Validation (Step 7)

1. **Unit Tests**:
   - Test `EnumPathData` construction
   - Test helper methods
   - Test serialization/deserialization if applicable

2. **Integration Tests**:
   - Verify enum paths still generate correctly
   - Check output format matches expectations
   - Validate with `TestVariantChainEnum` example

3. **Regression Tests**:
   - Run full test suite
   - Verify no functionality changes
   - Check performance remains acceptable

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

The execution sequence is optimized for:
1. **Additive changes first** (Step 1-2) - Won't break anything
2. **Atomic breaking change group** (Steps 3-6) - Related changes together with both old/new coexisting temporarily
3. **Validation** (Step 7) - Comprehensive testing

Each step compiles successfully, allowing for incremental validation and rollback if needed.

## Migration Checklist

- [ ] Create `EnumPathData` struct with all fields (Step 1)
- [ ] Add `enum_data: Option<EnumPathData>` to `MutationPathInternal` (Step 2)
- [ ] Update all builders to populate new field (Step 3)
- [ ] Update all code reading old field (Step 4)
- [ ] Update serialization/output code (Step 5)
- [ ] Remove `enum_variant_path` and `enum_instructions` fields (Step 6)
- [ ] Run all tests (Step 7)
- [ ] Update documentation (Step 7)

## Field Descriptions

### variant_chain
The complete chain of `VariantPath` entries from the root type down to the current path. Each entry contains the variant name, full mutation path, instructions, and example value. For example, for path `.middle_struct.nested_enum.name` traversing `TestVariantChainEnum::WithMiddleStruct` → `BottomEnum::VariantB`, this would contain two `VariantPath` entries preserving all metadata needed for mutation guidance and parent enum processing.

### applicable_variants
All enum variants that share the exact same signature (field names and types) and therefore support this mutation path. Used to inform AI agents which variants work with a given path.

Note: This field is currently populated in `ExampleGroup` structure (see enum_path_builder.rs:429-432). During this refactoring, preserve the existing population logic by migrating it to work with the new `EnumPathData` structure. This is NOT future work - the functionality exists and must be preserved during migration.

### variant_chain_root_example
The complete root-level example that correctly demonstrates this specific variant chain. This will be populated by the root example fix plan (plan-mutation-path-root-example.md).

### enum_instructions
Human-readable text explaining how to use this enum path, which variants to select, etc. Always populated when `EnumPathData` exists, generated based on the variant chain by `generate_enum_instructions()`. Note: This is a required `String` field (not `Option<String>`) because the optionality is handled at the `Option<EnumPathData>` level - if you have enum data, you always have instructions.

## Success Criteria

1. All enum-related data consolidated into `EnumPathData`
2. No references to old `enum_variant_path` field remain
3. All tests pass
4. Output format preserves enum information correctly
5. Code is cleaner and more maintainable
6. Ready for variant chain root example enhancement

## Dependencies

None - this is a prerequisite refactoring.

## Dependents

- `plan-mutation-path-root-example.md` requires this refactoring to be complete before implementation

## Notes

- This is a pure refactoring with no functionality changes
- Focus on maintaining exact same behavior during migration
- The new structure will be enhanced by future plans
- Consider adding more fields to `EnumPathData` as needs arise

## Design Review Skip Notes

(This section will be populated if design review suggestions are rejected)
