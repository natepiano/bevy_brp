# Plan: Build Root Examples Bottom-Up During Enum Recursion

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

### Step 1: Add Data Structure Storage ✅ COMPLETED

**Objective**: Add the foundational data structures needed for bottom-up partial root building

**Change Type**: ADDITIVE (safe - won't break existing code)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes**:

1. **Update VariantName derives** to support BTreeMap keys and HashSet membership:
   ```rust
   // Current:
   #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
   pub struct VariantName(String);

   // New:
   #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
   pub struct VariantName(String);
   ```

2. **Add `partial_root_examples` field to MutationPathInternal**:
   ```rust
   pub struct MutationPathInternal {
       // ... existing fields ...

       /// For enum root paths at each nesting level: Maps FULL variant chains to partial
       /// root examples built from this enum level down through all descendants.
       pub partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
   }
   ```

3. **Add new fields to EnumPathData**:
   ```rust
   pub struct EnumPathData {
       pub variant_chain: Vec<VariantPath>,

       /// NEW: Variant names where this path is valid
       #[serde(skip_serializing_if = "Vec::is_empty")]
       pub applicable_variants: Vec<VariantName>,

       /// NEW: Complete root example for single-step mutation
       #[serde(skip_serializing_if = "Option::is_none")]
       pub root_example: Option<Value>,
   }
   ```

4. **Update EnumPathData initialization** in `enum_path_builder.rs`:
   ```rust
   EnumPathData {
       variant_chain: ctx.variant_chain.clone(),
       applicable_variants: Vec::new(),  // NEW
       root_example: None,  // NEW
   }
   ```

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**: Code compiles successfully. New fields are added but not yet used.

**Validation**: Confirm cargo build succeeds with no errors.

---

### Step 2: Implement Helper Functions ✅ COMPLETED

**Objective**: Add the helper functions that will build partial root examples during recursion

**Change Type**: ADDITIVE (safe - new functions not yet called)

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Changes**:

Add five new helper functions (see Phase 3 in detailed implementation below for complete code):

1. **`build_partial_root_examples()`** - Builds partial roots for all variant chains
2. **`build_partial_root_for_chain()`** - Builds partial root for specific chain
3. **`wrap_nested_example()`** - Wraps child partial roots into parent examples
4. **`populate_root_examples()`** - Copies roots to paths at root level
5. **`extract_variant_names()`** - Helper to extract variant names from chains

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**: Code compiles successfully. New functions exist but aren't called yet.

**Validation**: Confirm cargo build succeeds with no errors or warnings about unused functions.

---

### Step 3: Integrate Bottom-Up Building ✅ COMPLETED

**Objective**: Connect the helper functions into the main recursion flow to actually build partial roots

**Change Type**: BREAKING (modifies existing flow - requires Steps 1 & 2)

**Dependencies**: MUST complete Steps 1 and 2 first

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Changes**:

Update `create_result_paths()` to call the new building functions:

```rust
fn create_result_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_paths: Vec<MutationPathInternal>,
) -> Result<Vec<MutationPathInternal>> {
    // ... EXISTING code to create enum_data and root_mutation_path ...

    // ==================== NEW CODE ====================
    let partial_roots = build_partial_root_examples(
        &enum_examples,
        &child_paths,
        ctx,
    )?;

    let mut root_mutation_path = root_mutation_path;
    root_mutation_path.partial_root_examples = Some(partial_roots.clone());

    if ctx.variant_chain.is_empty() {
        populate_root_examples(&mut child_paths, &partial_roots);
    }
    // ==================== END NEW CODE ====================

    let mut result = vec![root_mutation_path];
    result.extend(child_paths);
    Ok(result)
}
```

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**: Bottom-up building is now active. Partial roots are being built during recursion.

**Validation**: Confirm cargo build succeeds. The building algorithm is now integrated.

---

### Step 4: Populate Applicable Variants ✅ COMPLETED

**Objective**: Track which enum variants make each path valid

**Change Type**: ADDITIVE (populates fields added in Step 1)

**Dependencies**: Requires Step 1

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Changes**:

Update `process_children()` to populate `applicable_variants`:

```rust
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(HashMap<MutationPathDescriptor, Value>, Vec<MutationPathInternal>)> {
    // ... existing code ...

    for (signature, variants_in_group) in variant_groups {
        let applicable_variants: Vec<VariantName> = variants_in_group
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        // ... existing path processing ...

        let mut child_paths = builder::recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

        // ==================== NEW CODE ====================
        for child_path in &mut child_paths {
            if let Some(enum_data) = &mut child_path.enum_data {
                for variant_name in &applicable_variants {
                    enum_data.applicable_variants.push(variant_name.clone());
                }
            }
        }
        // ==================== END NEW CODE ====================

        // ... rest of existing code ...
    }

    Ok((child_examples, all_child_paths))
}
```

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**: Paths now know which variants they're valid in.

**Validation**: Confirm cargo build succeeds.

---

### Step 5: Expose New Fields to Output ✅ COMPLETED

**Objective**: Make the new fields visible to users in the type guide output

**Change Type**: ADDITIVE (new output fields)

**Dependencies**: Requires Step 1

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Changes**:

1. **Update PathInfo struct** (in types.rs):

   **REMOVE the old `enum_variant_path` field** (we're replacing it, not adding to it):
   ```rust
   // DELETE THIS FIELD:
   /// Ordered list of variant requirements from root to this path (optional)
   #[serde(skip_serializing_if = "Vec::is_empty")]
   pub enum_variant_path: Vec<VariantPath>,
   ```

   **ADD two new fields**:
   ```rust
   pub struct PathInfo {
       // ... existing fields (enum_instructions, etc.) ...

       /// NEW: List of variants where this path is valid
       #[serde(skip_serializing_if = "Option::is_none")]
       pub applicable_variants: Option<Vec<VariantName>>,

       /// NEW: Complete root example for single-step mutation
       #[serde(skip_serializing_if = "Option::is_none")]
       pub root_example: Option<Value>,
   }
   ```

2. **Update `from_mutation_path_internal()`** (in types.rs):

   **REMOVE the old enum_variant_path population**:
   ```rust
   // DELETE THIS:
   enum_variant_path: path
       .enum_data
       .as_ref()
       .map(|ed| ed.variant_chain.clone())
       .unwrap_or_default(),
   ```

   **ADD new field population**:
   ```rust
   // NEW: Extract applicable_variants and root_example from enum_data
   let (applicable_variants, root_example) = path
       .enum_data
       .as_ref()
       .map(|enum_data| {
           let variants = if !enum_data.applicable_variants.is_empty() {
               Some(enum_data.applicable_variants.clone())
           } else {
               None
           };
           (variants, enum_data.root_example.clone())
       })
       .unwrap_or((None, None));

   Self {
       description,
       path_info: PathInfo {
           path_kind: path.path_kind.clone(),
           type_name: path.type_name.clone(),
           type_kind,
           mutation_status: path.mutation_status,
           mutation_status_reason: path.mutation_status_reason.clone(),
           enum_instructions: path
               .enum_data
               .as_ref()
               .map(|ed| super::enum_path_builder::generate_enum_instructions(ed)),
           // REMOVED: enum_variant_path field
           applicable_variants,  // NEW
           root_example,  // NEW
       },
       examples,
       example,
   }
   ```

3. **Update `generate_enum_instructions()`** (in enum_path_builder.rs):

   **CRITICAL**: This function has a signature change and needs call site updates!

   **Old signature** (REMOVE):
   ```rust
   pub fn generate_enum_instructions(ctx: &RecursionContext) -> Option<String>
   ```

   **New signature and implementation**:
   ```rust
   fn generate_enum_instructions(_enum_data: &EnumPathData) -> String {
       // Note: Don't duplicate applicable_variants in the instructions - it's already a separate field
       "First, set the root component to 'root_example', then mutate this path. See 'applicable_variants' for which variants support this field.".to_string()
   }
   ```

   **Call site update**: Find where `generate_enum_instructions(ctx)` is called and change it to use `enum_data` from the `EnumPathData` being created. The call should be `enum_instructions: Some(generate_enum_instructions(&enum_data))` instead of `enum_instructions: generate_enum_instructions(ctx)`.

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Expected Impact**: Type guide output now includes the new fields for single-step mutations.

**Validation**: Confirm cargo build succeeds.

---

### Step 6: Complete Validation ✅ COMPLETED

**Objective**: Verify the complete implementation works as expected

**Tasks**:

1. **Build and install**:
   ```bash
   cargo build && cargo +nightly fmt
   cargo install --path mcp
   ```

2. **Reconnect MCP**: User runs `/mcp reconnect brp`

3. **Test with TestVariantChainEnum**: Run type guide and verify output

4. **Verify Test Case 1** - Shallow Path `.middle_struct`:
   - Has `root_example` with only `WithMiddleStruct` wrapper
   - Shows `applicable_variants: ["TestVariantChainEnum::WithMiddleStruct"]`

5. **Verify Test Case 2** - Deep Path `.middle_struct.nested_enum.name`:
   - Has `root_example` with both `WithMiddleStruct` and `VariantB`
   - Shows `applicable_variants: ["BottomEnum::VariantB"]`
   - Root example uses correct variant (VariantB for `.name`, not VariantA)

6. **Check all verification checklist items**:
   - [ ] `.middle_struct` has `root_example` with only `WithMiddleStruct` wrapper
   - [ ] `.middle_struct.nested_enum.name` has `root_example` with both `WithMiddleStruct` and `VariantB`
   - [ ] `.middle_struct.nested_enum.name` shows `applicable_variants: ["BottomEnum::VariantB"]`
   - [ ] `.middle_struct.nested_enum.value` shows `applicable_variants: ["BottomEnum::VariantA", "BottomEnum::VariantB"]`
   - [ ] Root examples use correct variants
   - [ ] No recursive/infinite structures in root examples
   - [ ] Root-level paths have `path_info: null` (not nested in enums)

**Expected Impact**: Complete feature working end-to-end. Single-step mutations enabled.

**Validation**: All test cases pass, verification checklist complete.

---

## Goal

**Replace multi-step `enum_variant_path` arrays with single-step `root_variant_example` fields.**

Currently, the type guide output (see `TestVariantChainEnum.json`) provides multi-step mutation instructions via `enum_variant_path` arrays. For deeply nested enum fields like `.middle_struct.nested_enum.name`, the agent must:

1. First mutate root to `{"WithMiddleStruct": {"middle_struct": {"nested_enum": {"VariantA": ...}}}}`
2. Then mutate `.middle_struct.nested_enum` to `{"VariantB": {...}}`

**The Issue:** Step 1 uses the wrong variant (VariantA) because we only build one example per enum level during recursion. The field `.name` only exists in VariantB, not VariantA.

**The Solution:** Build complete root examples during recursion that show the CORRECT variant chain for each path. For `.middle_struct.nested_enum.name`, provide a single `root_variant_example`:

```json
{
  "WithMiddleStruct": {
    "middle_struct": {
      "nested_enum": {
        "VariantB": {
          "name": "Hello, World!",
          "value": 3.14
        }
      }
    }
  }
}
```

This enables single-step mutations instead of error-prone multi-step processes.

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step. Each step can be validated independently with cargo build before proceeding to the next step.

## Summary of Changes

This plan fixes the multi-step mutation requirement for deeply nested enum fields by building complete root examples during recursion. The implementation adds:

**New Fields:**
- `MutationPathInternal.partial_root_examples`: Stores partial roots at each enum level
- `EnumPathData.applicable_variants`: Tracks which variants make a path valid
- `EnumPathData.root_example`: Complete root example for the path
- `PathInfo.applicable_variants`: Exposed to user
- `PathInfo.root_example`: Exposed to user

**New Functions:**
- `build_partial_root_examples()`: Builds partial roots for all variant chains
- `build_partial_root_for_chain()`: Builds partial root for specific chain
- `wrap_nested_example()`: Wraps child partial roots into parent examples
- `populate_root_examples()`: Copies roots to paths at root level
- `extract_variant_names()`: Helper to extract variant names

**Modified Functions:**
- `create_result_paths()`: Calls new building functions
- `process_children()`: Populates `applicable_variants`
- `generate_enum_instructions()`: Provides single-step guidance
- `MutationPath::from_mutation_path_internal()`: Exposes new fields

**Key Algorithm:** Bottom-up building where each enum wraps its children's already-built partial roots (one level of wrapping per enum). By the time we reach root, all work is done - just copy results to paths.

## Solution: Bottom-Up Building

**Key Insight:** Build partial root examples at EACH enum level during recursion UP. Each enum wraps its children's already-built partial roots. By the time we reach the root, all work is done - just copy the results to paths.

**Path-Specific Root Example Sizes:**
- Shallow paths (`.middle_struct`) → Small root examples (1 enum level)
- Deep paths (`.middle_struct.nested_enum.name`) → Large root examples (2+ enum levels)

### Data Flow (Bottom-Up)

**Key terminology:**
- "Enum root path at each level" = Any path that is the root of an enum type (has `enum_example_groups`)
- For TestVariantChainEnum: Path `""` is the enum root path
- For BottomEnum: Path `".middle_struct.nested_enum"` is the enum root path

```
[Depth 3] String ".name" field (path ".middle_struct.nested_enum.name")
  → Returns with variant_chain=[WithMiddleStruct, VariantB]
  ↑ No partial roots to build (not an enum root path)

[Depth 2] BottomEnum (path ".middle_struct.nested_enum" - enum root path at this level)
  → Sees child variant_chains: [WithMiddleStruct, VariantB], [WithMiddleStruct, VariantA], etc.
  → Children are primitives (no partial roots to wrap)
  → Builds partial roots for its own root path:
      [WithMiddleStruct, VariantB] → {"VariantB": {"name": "...", "value": ...}}
      [WithMiddleStruct, VariantA] → {"VariantA": 123}
  → Stores these in partial_root_examples on path ".middle_struct.nested_enum"
  ↑ Returns to parent

[Depth 1] MiddleStruct (struct, not enum - no enum root path here)
  → Just passes paths through unchanged
  ↑ Returns to parent

[Depth 0] TestVariantChainEnum (path "" - enum root path at top level)
  → Sees child variant_chains: [WithMiddleStruct, VariantB], [WithMiddleStruct], etc.
  → Searches child_paths for paths with partial_root_examples (finds BottomEnum at ".middle_struct.nested_enum")
  → Builds complete root examples by wrapping (ONE level):
      [WithMiddleStruct, VariantB]:
        Start: {"WithMiddleStruct": {"middle_struct": {"nested_enum": <default>, ...}}}
        Get BottomEnum's partial root for [VariantB]: {"VariantB": {...}}
        Wrap: Insert into nested_enum field
        Result: {"WithMiddleStruct": {"middle_struct": {"nested_enum": {"VariantB": {...}}, ...}}}

      [WithMiddleStruct]:
        Start: {"WithMiddleStruct": {"middle_struct": {...}}}
        No more nesting needed
        Result: {"WithMiddleStruct": {"middle_struct": {...}}}
  → Stores complete roots in partial_root_examples on path ""
  → Populates root_example on all matching descendant paths
```

## Detailed Implementation

**Prerequisites:**

Before implementing, ensure the following imports are added to the relevant files:

```rust
// In types.rs
use std::collections::BTreeMap;

// In enum_path_builder.rs
use std::collections::{BTreeMap, HashSet};
use tracing; // For warning/debug logging
```

### Phase 1: Add Storage for Partial Root Examples

**Location:** `types.rs` - Update `MutationPathInternal` and `EnumData`

**1a. Update `VariantName` to support BTreeMap and HashSet usage:**

The plan uses `BTreeMap<Vec<VariantName>, Value>` for partial root examples and `HashSet<Vec<VariantName>>` for collecting unique chains. For these to work, `VariantName` must implement both `Ord` (for BTreeMap) and `Hash` (for HashSet).

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct VariantName(String);
```

**Why:** `BTreeMap` requires keys to implement `Ord`, and `HashSet` requires `Hash`. The plan uses BTreeMap for deterministic ordering in tests and HashSet for collecting unique variant chains (see build_partial_root_examples function in Phase 3). This matches the pattern for `StructFieldName` in the codebase, which is also a newtype wrapper around `String` and derives both `Hash` and `Ord`.

**1b. Add `partial_root_examples` field to `MutationPathInternal`:**

```rust
pub struct MutationPathInternal {
    // ... existing fields ...

    /// For enum root paths at each nesting level: Maps FULL variant chains to partial
    /// root examples built from this enum level down through all descendants.
    ///
    /// **Populated for paths where `enum_example_groups.is_some()`** - meaning any path that
    /// is the root of an enum type at ANY nesting level:
    /// - Path "" (TestVariantChainEnum) has this field
    /// - Path ".middle_struct.nested_enum" (BottomEnum) has this field
    /// - Leaf paths like ".middle_struct.nested_enum.name" have None
    ///
    /// Example at BottomEnum (path ".middle_struct.nested_enum"):
    ///   [WithMiddleStruct, VariantB] => {"VariantB": {"name": "...", "value": ...}}
    ///   [WithMiddleStruct, VariantA] => {"VariantA": 123}
    ///
    /// Example at TestVariantChainEnum (path ""):
    ///   [WithMiddleStruct, VariantB] => {"WithMiddleStruct": {"middle_struct": {"nested_enum": {"VariantB": {...}}}}}
    ///   [WithMiddleStruct] => {"WithMiddleStruct": {"middle_struct": {...}}}
    ///
    /// None for non-enum paths (structs, primitives) and enum leaf paths.
    pub partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
}
```

**Why:** Each enum at each nesting level needs to store partial root examples indexed by the FULL variant chain (no prefix stripping - keeps code simple and readable). Parent enums look up child's partial roots by searching child_paths for paths with `partial_root_examples.is_some()` and matching variant chains.

**Important:** This field is on `MutationPathInternal` (not in `EnumPathData`) because the top-level enum root path (path "") has `enum_data = None` (since `variant_chain` is empty at the root), yet it still needs to build and store partial roots for its descendants.

**1c. Update `EnumPathData` structure:**

**REMOVE `enum_instructions` field** (generated on-the-fly instead of stored):
```rust
// DELETE THIS FIELD from EnumPathData:
pub enum_instructions: Option<String>,
```

**ADD two new fields to `EnumPathData`:**

```rust
#[derive(Debug, Clone)]
pub struct EnumPathData {
    /// The chain of variant selections from root to this point
    pub variant_chain: Vec<VariantPath>,

    /// NEW: Variant names where this path is valid
    /// Example: [VariantName("VariantB"), VariantName("VariantA")]
    /// Populated during path processing in Phase 5
    pub applicable_variants: Vec<VariantName>,

    /// NEW: Complete root example for single-step mutation
    /// Only populated at root level (when ctx.variant_chain is empty)
    /// Copied from partial_root_examples in Phase 2
    pub root_example: Option<Value>,

    // NOTE: enum_instructions is NOT stored here - it's generated on-the-fly
    // by calling generate_enum_instructions(enum_data) in from_mutation_path_internal()
}
```

**CRITICAL:** Update `EnumPathData` initialization in **BOTH** locations to include ONLY these 3 fields:

**Location 1:** `enum_path_builder.rs` (~line 918-926):
```rust
EnumPathData {
    variant_chain: populate_variant_path(ctx, &enum_examples, &default_example),
    applicable_variants: Vec::new(),           // NEW
    root_example: None,          // NEW
    // REMOVED: enum_instructions field (generated on-the-fly in from_mutation_path_internal)
}
```

**Location 2:** `builder.rs` (~line 292-296):
```rust
EnumPathData {
    variant_chain: ctx.variant_chain.clone(),
    applicable_variants: Vec::new(),           // NEW
    root_example: None,          // NEW
    // REMOVED: enum_instructions field (generated on-the-fly in from_mutation_path_internal)
}
```

### Phase 2: Build Partial Roots at Each Enum Level

**Location:** `enum_path_builder.rs` - Update `create_result_paths()`

**Current behavior:** Only root enum (`ctx.variant_chain.is_empty()`) builds root examples.

**New behavior:** EVERY enum builds partial root examples for all unique child variant chains.

```rust
fn create_result_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_paths: Vec<MutationPathInternal>,
) -> Vec<MutationPathInternal> {
    // ... EXISTING code to create enum_data and root_mutation_path ...

    // ... EXISTING code to update variant_path entries ...

    // ==================== NEW CODE ====================
    // Build partial root examples for all unique variant chains in children
    // This happens at EVERY enum root path (paths where enum_example_groups exists)
    // - For path "" (TestVariantChainEnum): builds roots for all descendants
    // - For path ".middle_struct.nested_enum" (BottomEnum): builds roots for its children
    //
    // Returns an error if building fails (InvalidState - indicates algorithm bug)
    let partial_roots = build_partial_root_examples(
        &enum_examples,
        &child_paths,
        ctx,
    )?;

    // Store partial roots on this enum's root path so parent enums can access them
    // Parent finds these by searching child_paths for paths with partial_root_examples.is_some()
    let mut root_mutation_path = root_mutation_path;
    root_mutation_path.partial_root_examples = Some(partial_roots.clone());

    // If we're at the actual root level (empty variant chain),
    // populate root_example on all paths
    if ctx.variant_chain.is_empty() {
        populate_root_examples(&mut child_paths, &partial_roots);
    }
    // ==================== END NEW CODE ====================

    // EXISTING code - Return root path plus all child paths
    let mut result = vec![root_mutation_path];
    result.extend(child_paths);
    result
}
```

### Phase 3: Build Partial Roots by Wrapping Children

**New Function:** `build_partial_root_examples()`

```rust
/// Build partial root examples for all unique variant chains in child paths
///
/// This function implements bottom-up building:
/// - At leaf enums: Build partial roots from scratch (nothing to wrap)
/// - At intermediate enums: Wrap child enums' already-built partial roots
/// - Each enum only does ONE level of wrapping
///
/// **Key insight**: Child paths contain FULL variant chains from root, but we only process
/// the portion relevant to this enum. We strip ancestor variants using `ctx.variant_chain.len()`.
///
/// Keys are FULL variant chains (e.g., `[WithMiddleStruct, VariantB]`) with NO stripping.
/// Uses `BTreeMap` for deterministic ordering in tests.
///
/// Returns an error if building fails, which indicates a bug in the algorithm.
fn build_partial_root_examples(
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Result<BTreeMap<Vec<VariantName>, Value>> {
    let mut partial_roots = BTreeMap::new();

    // Extract all unique FULL variant chains from child paths
    let unique_full_chains: HashSet<Vec<VariantName>> = child_paths
        .iter()
        .filter_map(|p| {
            p.enum_data.as_ref()
                .filter(|ed| !ed.variant_chain.is_empty())
                .map(|ed| extract_variant_names(&ed.variant_chain))
        })
        .collect();

    // For each unique FULL chain, build the partial root from this enum down
    for full_chain in unique_full_chains {
        // Skip chains that don't extend beyond ancestors (shouldn't happen, but defensive)
        let ancestor_len = ctx.variant_chain.len();
        if full_chain.len() <= ancestor_len {
            continue;
        }

        // Propagate errors - if building fails, the entire operation fails
        let root_example = build_partial_root_for_chain(
            &full_chain,
            enum_examples,
            child_paths,
            ctx,
        )?;

        // Store using the FULL chain as key (no stripping)
        // This allows parent enums to look up by full chains
        partial_roots.insert(full_chain, root_example);
    }

    Ok(partial_roots)
}

/// Build a partial root example for a specific variant chain
///
/// **Important**: The `chain` parameter is the FULL chain from root. We use
/// `ctx.variant_chain.len()` to determine which variant in the chain belongs to
/// this enum (the variant at index `ancestor_len`).
///
/// Returns an error if partial roots are missing, which indicates a bug in the building algorithm.
fn build_partial_root_for_chain(
    chain: &[VariantName],
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Result<Value> {
    use error_stack::Report;

    // Determine which variant in the full chain belongs to this enum
    // Example: At BottomEnum with ctx.variant_chain=[WithMiddleStruct],
    //          full_chain=[WithMiddleStruct, VariantB] → our_variant=VariantB (index 1)
    let ancestor_len = ctx.variant_chain.len();
    let our_variant = chain.get(ancestor_len).ok_or_else(|| {
        Report::new(Error::InvalidState(format!(
            "Chain {chain:?} too short for ancestor depth {ancestor_len}"
        )))
    })?;

    // Find the example for this variant from our enum_examples
    let base_example = enum_examples
        .iter()
        .find(|ex| ex.applicable_variants.contains(our_variant))
        .map(|ex| ex.example.clone())
        .ok_or_else(|| {
            Report::new(Error::InvalidState(format!(
                "No example found for variant {our_variant:?} in enum {}",
                ctx.type_name()
            )))
        })?;

    // If chain has more levels (nested enums), wrap the child's partial root
    if chain.len() > ancestor_len + 1 {
        // Find child enum root path that has partial roots
        for child in child_paths {
            // Look for enum root paths with partial_root_examples
            if let Some(child_partial_roots) = &child.partial_root_examples {
                // Check if child has a partial root for the FULL chain
                // Children store their partial roots with FULL chains as keys
                if let Some(nested_partial_root) = child_partial_roots.get(chain) {
                    // Wrap the nested partial root into our base example
                    // This is ONE level of wrapping
                    if let Some(wrapped) = wrap_nested_example(
                        &base_example,
                        nested_partial_root,
                        child,
                    ) {
                        return Ok(wrapped);
                    }
                    // If wrapping failed, continue searching other children
                }
            }
        }

        // If we reach here, no child had the required partial root
        // This is an InvalidState - the child should have built partial roots during its recursion
        Err(Report::new(Error::InvalidState(format!(
            "Missing partial root for variant chain {remaining_chain:?}. \
             Bottom-up building failed for enum {} - child enum did not build required partial roots. \
             This indicates a bug in the building algorithm.",
            ctx.type_name()
        ))))
    } else {
        // Chain length is 1 - no more nesting, just return our example
        Ok(base_example)
    }
}

/// Wrap a nested partial root into a parent example at the correct field
///
/// Returns None if wrapping fails (invalid path kind or parent isn't an object).
/// This allows the caller to continue searching for valid wrapping opportunities.
fn wrap_nested_example(
    parent_example: &Value,
    nested_partial_root: &Value,
    child_path: &MutationPathInternal,
) -> Option<Value> {
    // Extract the field name from the child path's PathKind
    let field_name = match &child_path.path_kind {
        PathKind::StructField { field_name, .. } => field_name.as_str(),
        PathKind::RootValue { .. } => {
            // Root value paths don't have a field name to wrap into
            tracing::debug!(
                "Cannot wrap into RootValue path - no field name available"
            );
            return None;
        }
        PathKind::IndexedElement { .. } | PathKind::ArrayElement { .. } => {
            // Indexed/array paths need special handling or may not be valid wrapping targets
            tracing::warn!(
                "Wrapping into indexed/array paths not currently supported"
            );
            return None;
        }
    };

    // Clone parent and replace the nested field
    match parent_example.as_object() {
        Some(parent_obj) => {
            let mut result = parent_obj.clone();
            result.insert(field_name.to_string(), nested_partial_root.clone());
            Some(Value::Object(result))
        }
        None => {
            tracing::warn!(
                "Parent example is not a JSON object, cannot wrap field '{field_name}'. \
                 Parent type: {}",
                match parent_example {
                    Value::Array(_) => "Array",
                    Value::String(_) => "String",
                    Value::Number(_) => "Number",
                    Value::Bool(_) => "Bool",
                    Value::Null => "Null",
                    _ => "Unknown",
                }
            );
            None
        }
    }
}

/// Populate root_example on all paths (root level only)
fn populate_root_examples(
    paths: &mut [MutationPathInternal],
    partial_roots: &BTreeMap<Vec<VariantName>, Value>,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_data {
            if !enum_data.variant_chain.is_empty() {
                let chain = extract_variant_names(&enum_data.variant_chain);
                if let Some(root_example) = partial_roots.get(&chain) {
                    enum_data.root_example = Some(root_example.clone());
                } else {
                    tracing::debug!(
                        "No root example found for variant chain: {:?}",
                        chain
                    );
                }
            }
        }
    }
}

/// Helper to extract variant names from variant path chain
fn extract_variant_names(variant_chain: &[VariantPath]) -> Vec<VariantName> {
    variant_chain.iter().map(|vp| vp.variant.clone()).collect()
}
```

### Phase 4: Update Output Structure

**Location:** `types.rs`

**Changes needed:**

1. **Add `root_variant_example` and `applicable_variants` to `PathInfo`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind: PathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name: BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind: TypeKind,
    /// Status of whether this path can be mutated
    pub mutation_status: MutationStatus,
    /// Reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_status_reason: Option<Value>,
    /// Instructions for setting variants required for this mutation path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions: Option<String>,
    /// Ordered list of variant requirements from root to this path (optional)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enum_variant_path: Vec<VariantPath>,

    /// NEW: List of variants where this path is valid
    /// Example: [VariantName("BottomEnum::VariantB")]
    /// VariantName serializes as a string in JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants: Option<Vec<VariantName>>,

    /// NEW: Complete root example for single-step mutation
    /// Only present for paths nested in enums
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example: Option<Value>,
}
```

**Serialization Verification:**

The `applicable_variants` field uses `Vec<VariantName>` where `VariantName` is a newtype wrapper around `String`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VariantName(String);
```

Serde's default behavior for single-field tuple structs is transparent serialization, so `VariantName` automatically serializes as a plain string. This produces the expected JSON format:

```json
"applicable_variants": ["BottomEnum::VariantB", "BottomEnum::VariantA"]
```

**Verification:** This behavior is already validated by `ExampleGroup.applicable_variants` in the existing codebase (types.rs:254), which produces correct JSON output in TestVariantChainEnum.json. No code changes to `VariantName` are required.

**Optional improvement:** Adding `#[serde(transparent)]` to `VariantName` would make this behavior explicit in the code, but it's not required since single-field tuple structs already have this behavior by default.

2. **Update `MutationPath::from_mutation_path_internal()` to populate new fields**

```rust
impl MutationPath {
    /// Create from `MutationPathInternal` with proper formatting logic
    pub fn from_mutation_path_internal(
        path: &MutationPathInternal,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema);

        // Generate description - override for partially_mutable paths
        let description = match path.mutation_status {
            MutationStatus::PartiallyMutable => {
                "This path is not mutable due to some of its descendants not being mutable"
                    .to_string()
            }
            _ => path.path_kind.description(&type_kind),
        };

        let (examples, example) = match path.mutation_status {
            MutationStatus::PartiallyMutable | MutationStatus::NotMutable => {
                // PartiallyMutable and NotMutable: no example at all (not even null)
                (vec![], None)
            }
            MutationStatus::Mutable => {
                path.enum_example_groups.as_ref().map_or_else(
                    || {
                        // Mutable paths: use the example value
                        (vec![], Some(path.example.clone()))
                    }
                    |enum_examples| {
                        // Enum root: use the examples array
                        (enum_examples.clone(), None)
                    },
                )
            }
        };

        // NEW: Extract applicable_variants and root_example from enum_data
        let (applicable_variants, root_example) = path
            .enum_data
            .as_ref()
            .map(|enum_data| {
                let variants = if !enum_data.applicable_variants.is_empty() {
                    Some(enum_data.applicable_variants.clone())
                } else {
                    None
                };
                (variants, enum_data.root_example.clone())
            })
            .unwrap_or((None, None));

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
                enum_instructions: path
                    .enum_data
                    .as_ref()
                    .and_then(|ed| ed.enum_instructions.clone()),
                enum_variant_path: path
                    .enum_data
                    .as_ref()
                    .map(|ed| ed.variant_chain.clone())
                    .unwrap_or_default(),
                // NEW: Add the two new fields
                applicable_variants,
                root_example,
            },
            examples,
            example,
        }
    }
}
```

3. **Update `generate_enum_instructions()` for single-step guidance**

```rust
fn generate_enum_instructions(_enum_data: &EnumPathData) -> String {
    // Note: Don't duplicate applicable_variants in the instructions - it's already a separate field
    "First, set the root component to 'root_example', then mutate this path. See 'applicable_variants' for which variants support this field.".to_string()
}
```

### Phase 5: Populate `applicable_variants`

**Location:** `enum_path_builder.rs` - Update `process_children()`

The `applicable_variants` field needs to be populated during path processing. This tells the user which variants make a particular path valid.

**Important:** The current `process_children` signature already has access to `variant_groups`, which contains the variant information we need. We do NOT need to add `enum_examples` as a parameter (which wouldn't work anyway since `enum_examples` is created AFTER `process_children` returns).

```rust
fn process_children(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(
    HashMap<MutationPathDescriptor, Value>,
    Vec<MutationPathInternal>,
)> {
    let mut child_examples = HashMap::new();
    let mut all_child_paths = Vec::new();

    // Process each variant group
    for (signature, variants_in_group) in variant_groups {
        let applicable_variants: Vec<VariantName> = variants_in_group
            .iter()
            .map(|v| v.variant_name().clone())
            .collect();

        // Create paths for this signature group
        let paths = create_paths_for_signature(signature, ctx);

        // Process each path
        for path in paths.into_iter().flatten() {
            let mut child_ctx = ctx.create_recursion_context(path.clone(), PathAction::Create);

            // Set up enum context for children
            if let Some(representative_variant) = applicable_variants.first() {
                child_ctx.variant_chain.push(VariantPath {
                    full_mutation_path: ctx.full_mutation_path.clone(),
                    variant: representative_variant.clone(),
                    instructions: String::new(),
                    variant_example: json!(null),
                });
            }

            // Recursively process child and collect paths
            let child_descriptor = path.to_mutation_path_descriptor();
            let child_schema = child_ctx.require_registry_schema()?;
            let child_type_kind = TypeKind::from_schema(child_schema);

            let mut child_paths =
                builder::recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

            // ==================== NEW: POPULATE applicable_variants ====================
            // Track which variants make these child paths valid
            for child_path in &mut child_paths {
                if let Some(enum_data) = &mut child_path.enum_data {
                    // Add all variants from this signature group
                    // (all variants in a group share the same signature/structure)
                    for variant_name in &applicable_variants {
                        enum_data.applicable_variants.push(variant_name.clone());
                    }
                }
            }
            // ==================== END NEW CODE ====================

            // Extract example from first path
            let child_example = child_paths
                .first()
                .map_or(json!(null), |p| p.example.clone());

            child_examples.insert(child_descriptor, child_example);
            all_child_paths.extend(child_paths);
        }
    }

    Ok((child_examples, all_child_paths))
}
```

**Key points:**

1. We use the existing `variant_groups` structure that `process_children` already receives
2. For each variant group (variants with the same signature), we extract the list of `VariantName` values
3. After recursing into child paths, we populate each child's `enum_data.applicable_variants` with all variants from the group
4. This happens during the existing recursion flow - no signature changes needed
5. Paths that appear in multiple variant groups will accumulate variants from each group they appear in

## Key Advantages of Bottom-Up Approach

1. **No Recursion:** Each enum only wraps ONE level (its immediate children's partial roots)
2. **Efficient:** Work is done once during recursion up, not traversed again
3. **Scalable:** Works for arbitrary nesting depth without recursive search
4. **Right-Sized:** Each path gets exactly the root example it needs:
   - Short chains → Small root examples
   - Long chains → Large root examples

## Testing

Use `extras_plugin::TestVariantChainEnum`:

```bash
cargo build && cargo +nightly fmt
cargo install --path mcp
# User: /mcp reconnect brp
# Test: Run type guide
```

### Expected Results

**Test Case 1: Shallow Path `.middle_struct`**

Should have small root example (only 1 enum level):

```json
{
  "path": ".middle_struct",
  "example": { "nested_enum": { "VariantA": 123 }, "value": 42 },
  "path_info": {
    "applicable_variants": ["TestVariantChainEnum::WithMiddleStruct"],
    "root_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": { "VariantA": 123 },
          "value": 42
        }
      }
    },
    "enum_instructions": "This field is nested within enum variants. Use the 'root_example' for single-step mutation: First set root to 'root_example', then mutate this path. Applicable variants: TestVariantChainEnum::WithMiddleStruct"
  }
}
```

**Test Case 2: Deep Path `.middle_struct.nested_enum.name`**

Should have large root example (2 enum levels: TestVariantChainEnum + BottomEnum):

```json
{
  "path": ".middle_struct.nested_enum.name",
  "example": "Hello, World!",
  "path_info": {
    "applicable_variants": ["BottomEnum::VariantB"],
    "root_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": {
            "VariantB": {
              "name": "Hello, World!",
              "value": 3.14
            }
          },
          "value": 42
        }
      }
    },
    "enum_instructions": "This field is nested within enum variants. Use the 'root_example' for single-step mutation: First set root to 'root_example', then mutate this path. Applicable variants: BottomEnum::VariantB"
  }
}
```

**Test Case 3: Root-Level Enum Path (empty variant chain)**

Should NOT have `root_variant_example` since it's already at root:

```json
{
  "path": "",
  "example": { "WithMiddleStruct": { "middle_struct": { ... } } },
  "path_info": null
}
```

## Potential Issues and Solutions

### Issue 1: Circular References in Root Examples

**Problem**: If enum A contains enum B which contains enum A, building root examples could create infinite structures.

**Solution**: The recursion context already tracks depth and prevents infinite recursion during schema traversal. The `partial_root_examples` are built during the return path of existing recursion, so they inherit the same depth limits.

### Issue 2: Memory Usage with Deep Nesting

**Problem**: Deep nesting (5+ enum levels) creates large root examples stored on every path.

**Solution**:
1. Root examples are only stored on enum root paths (one per enum level)
2. Leaf paths reference these via `root_example` (shared, not duplicated)
3. If memory becomes an issue, consider adding a config option to limit root example depth

### Issue 3: BTreeMap Key Ordering

**Problem**: `Vec<VariantName>` as BTreeMap key requires `Ord` implementation.

**Solution**: Ensure `VariantName` type implements `Ord`, or use a wrapper type. If `VariantName` is a type alias for `String`, it already implements `Ord`.

### Issue 4: Missing Partial Roots During Lookup

**Problem**: Parent enum looks for child's partial root but doesn't find it.

**Solution**: Return `Error::InvalidState` to propagate the failure. This is an impossible state - if the bottom-up building works correctly, child enums always build partial roots before their parents need them. An InvalidState error indicates a bug in the building algorithm itself.

**Error propagation**: The error bubbles up through `build_partial_root_for_chain()` → `build_partial_root_examples()` → `create_result_paths()`, causing the entire type guide generation to fail with a clear error message identifying which enum and variant chain failed.

## Implementation Bug Fixes

### BUG-1: Incorrect variant chain handling in `build_partial_root_for_chain`
- **Discovered**: During Step 6 validation testing with `TestVariantChainEnum`
- **Symptom**: `InvalidState` error "No example found for variant TestVariantChainEnum::WithMiddleStruct in enum BottomEnum"
- **Root Cause**: The original plan passed local chains to `build_partial_root_for_chain`, but when looking up child partial roots, it used `remaining_chain` which didn't match the full keys that children stored
- **Fix**: Changed to pass FULL chains throughout, using `ctx.variant_chain.len()` to determine which variant belongs to the current enum (via index, not slicing). When looking up child partial roots, use the full `chain` as the key, not a sliced `remaining_chain`
- **Key Insight**: Children store partial roots with FULL chain keys. Parents must look them up with the same FULL chain keys
- **Status**: ✅ FIXED - Implemented and tested

### BUG-2: Incorrect wrapping level in `wrap_nested_example`
- **Discovered**: During Step 6 validation - wrong variants in root examples and malformed structure
- **Symptom**: For `.middle_struct.nested_enum.name`, the `root_example` shows `VariantA` instead of `VariantB`, plus an extra malformed `nested_enum` field at root level
- **Root Cause**: `wrap_nested_example` inserts child partial roots at the wrong nesting level. It extracts field name "nested_enum" from `PathKind` but doesn't navigate through the parent structure (e.g., `["WithMiddleStruct"]["middle_struct"]`) before replacing. It inserts at root level instead of the nested location where the field actually exists.
- **Fix**: Rewrite `wrap_nested_example` to:
  1. Unwrap the variant wrapper from parent example
  2. Parse child's `full_mutation_path` into navigation segments
  3. Navigate through JSON tree recursively using `navigate_and_replace` helper
  4. Replace target field at correct nested location
  5. Re-wrap result with variant name
- **Key Insight**: Enum examples have structure `{"VariantName": {...}}`. To replace nested fields, must unwrap, navigate path, replace, then rewrap.
- **Status**: ✅ FIXED - Implemented and tested

### BUG-3: `generate_enum_instructions()` not updated for single-step guidance
- **Discovered**: Post-Step 6 validation - reviewing TestVariantChainEnum_fixed.json output
- **Symptom**: `enum_instructions` field still shows old multi-step guidance like "mutation path requires 2 variant selections. Follow the instructions in variant_path array" instead of new single-step guidance referencing `root_example`
- **Root Cause**: Step 5 was marked complete but the `generate_enum_instructions()` function update from Phase 4 Section 3 (lines 1009-1027) was never actually implemented. The function still uses the old signature `fn generate_enum_instructions(ctx: &RecursionContext)` and generates old multi-step instructions, instead of the new signature `fn generate_enum_instructions(enum_data: &EnumPathData)` that generates single-step guidance
- **Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:452-469`
- **Fix Required**:
  1. Change function signature from `pub fn generate_enum_instructions(ctx: &RecursionContext) -> Option<String>` to `fn generate_enum_instructions(enum_data: &EnumPathData) -> String`
  2. Replace implementation with plan's Phase 4 Section 3 code that generates single-step instructions
  3. Update call site in enum path creation to pass `enum_data` instead of `ctx`
  4. Expected output: `"This field is nested within enum variants. Use the 'root_example' for single-step mutation: First set root to 'root_example', then mutate this path. Applicable variants: BottomEnum::VariantB"`
- **Key Insight**: The `root_example` field is being populated correctly, but users are still being told to use the old multi-step approach. This defeats the entire purpose of the plan - to enable single-step mutations.
- **Additional Issue**: The old `enum_variant_path` field is still being output alongside the new fields, contradicting the plan's goal to "Replace" (not "add to") the old approach.
- **Status**: ✅ FIXED - Instructions updated, but `enum_variant_path` removal still pending (see BUG-4)

### BUG-4: Old `enum_variant_path` field still present in output
- **Discovered**: Post-BUG-3 fix validation - reviewing TestVariantChainEnum_fixed.json output
- **Symptom**: Output JSON contains both the old `enum_variant_path` array AND the new `root_example` field, when it should only have the new field
- **Root Cause**: Plan's Goal section (line 402) says "Replace multi-step `enum_variant_path` arrays with single-step `root_variant_example` fields" but Step 5 implementation doesn't explicitly remove the old field. The old field is still defined in `PathInfo` struct and still being populated in `from_mutation_path_internal()`
- **Locations requiring changes**:

  **PathInfo struct and serialization:**
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:260-262` - Remove `enum_variant_path` field from PathInfo struct
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:401-405` - Remove `enum_variant_path` population in from_mutation_path_internal()

  **EnumPathData struct and initialization:**
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:310-311` - Remove `enum_instructions` field from EnumPathData struct definition
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:926` - Remove `enum_instructions: None,` from EnumPathData initialization
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs:296` - Remove `enum_instructions: None,` from EnumPathData initialization

- **Fix Required** (5 steps):
  1. Remove `enum_variant_path` field from `PathInfo` struct definition (types.rs)
  2. Remove `enum_variant_path` population in `from_mutation_path_internal()` (types.rs)
  3. Remove `enum_instructions` field from `EnumPathData` struct definition (types.rs)
  4. Remove `enum_instructions: None,` from `EnumPathData` initialization in enum_path_builder.rs
  5. Remove `enum_instructions: None,` from `EnumPathData` initialization in builder.rs

  **Note:** Keep `variant_chain` in `EnumPathData` (internal use for building), but don't expose `enum_variant_path` in output
- **Key Insight**: "Replace" means remove the old, not keep both. The new `root_example` makes the old multi-step `enum_variant_path` redundant and confusing. Instructions are generated on-the-fly by calling `generate_enum_instructions(enum_data)` in `from_mutation_path_internal()`, so there's no need to store them in `EnumPathData`.
- **Status**: ⏳ PENDING - Needs implementation

### BUG-5: `applicable_variants` accumulating from multiple enum levels
- **Discovered**: Post-BUG-4 fix validation - reviewing TestVariantChainEnum_fixed.json output
- **Symptom**: `applicable_variants` contains variants from ALL enum levels in the variant chain, not just the innermost enum. For example, `.middle_struct.nested_enum.0` shows `["BottomEnum::VariantA", "TestVariantChainEnum::WithMiddleStruct"]` instead of just `["BottomEnum::VariantA"]`
- **Root Cause**: In `process_children()`, the code populates `applicable_variants` for ALL child paths with `enum_data`, regardless of nesting depth. As recursion unwinds, parent enums add their variants to paths that are actually grandchildren (nested deeper)
- **Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:546-554`
- **Execution Flow** for `.middle_struct.nested_enum.0`:
  1. BottomEnum level (`ctx.variant_chain.len() = 1`): Returns path with `variant_chain = [WithMiddleStruct, VariantA]`, adds "VariantA" → `applicable_variants = ["VariantA"]` ✓
  2. TestVariantChainEnum level (`ctx.variant_chain.len() = 0`): Receives same path with `variant_chain.len() = 2`, INCORRECTLY also adds "WithMiddleStruct" → `applicable_variants = ["VariantA", "WithMiddleStruct"]` ✗
- **Fix Required**:
  Only populate `applicable_variants` for **direct children** of the current enum level. A path is a direct child if:
  ```rust
  enum_data.variant_chain.len() == ctx.variant_chain.len() + 1
  ```

  **Current code (enum_path_builder.rs:546-554):**
  ```rust
  for child_path in &mut child_paths {
      if let Some(enum_data) = &mut child_path.enum_data {
          for variant_name in &applicable_variants {
              enum_data.applicable_variants.push(variant_name.clone());
          }
      }
  }
  ```

  **Fixed code:**
  ```rust
  for child_path in &mut child_paths {
      if let Some(enum_data) = &mut child_path.enum_data {
          // Only populate for direct children, not grandchildren nested deeper
          if enum_data.variant_chain.len() == ctx.variant_chain.len() + 1 {
              for variant_name in &applicable_variants {
                  enum_data.applicable_variants.push(variant_name.clone());
              }
          }
      }
  }
  ```
- **Validation**: After fix, for `.middle_struct.nested_enum.0`:
  - At BottomEnum: `2 == 1 + 1` → ✓ Add "VariantA"
  - At TestVariantChainEnum: `2 != 0 + 1` → ✗ Skip (don't add "WithMiddleStruct")
  - Result: `applicable_variants = ["BottomEnum::VariantA"]` ✓
- **Key Insight**: `applicable_variants` should answer "Which variants of the **containing** enum support this field?" not "Which variants from the entire chain are involved?" Parent enum variants are already represented in `root_example`.
- **Impact**: Output-only field. No internal logic depends on it. Safe to fix.
- **Status**: ⏳ PENDING - Needs implementation

## Design Review Skip Notes

### DESIGN-1: Missing explanation for handling IndexedElement and ArrayElement in wrapping logic - **Verdict**: REJECTED
- **Status**: REJECTED - Finding was incorrect
- **Location**: Phase 3: Build Partial Roots by Wrapping Children - wrap_nested_example function
- **Issue**: Original finding claimed the plan doesn't explain whether wrapping should work for IndexedElement paths (created by tuple variants)
- **Reasoning**: Investigation revealed the plan correctly implements two separate complementary mechanisms: (1) Field-based wrapping for struct variants (wrap_nested_example), and (2) Index-based assembly for tuple variants (build_variant_example). IndexedElement paths are intentionally excluded from wrapping because they participate in a different construction mechanism. The match arm that rejects IndexedElement in wrap_nested_example is not a gap - it's defensive programming that catches architectural violations. The code is self-documenting through its structure.
- **Decision**: User agreed with rejection - plan correctly handles both struct and tuple variants through appropriate separate mechanisms

### QUALITY-1: Inconsistent terminology: 'root_example' vs 'root_variant_example' - **Verdict**: CONFIRMED
- **Status**: APPROVED - Implemented
- **Location**: Multiple sections - Summary, Goal, Phase 1c, Phase 4, Test Cases
- **Issue**: Plan inconsistently used 'root_example' (existing field in EnumPathData) and 'root_variant_example' (proposed new name for PathInfo field)
- **Resolution**: Updated plan to use 'root_example' consistently throughout, since this is the existing field name in the codebase (types.rs:287). Changed all references in PathInfo, from_mutation_path_internal, generate_enum_instructions, and test examples to use the consistent name.
- **Decision**: User requested consistency check and approved using the existing field name throughout

### DESIGN-2: Unclear error handling when child partial roots are missing during lookup - **Verdict**: CONFIRMED
- **Status**: APPROVED - Implemented
- **Location**: Phase 3: build_partial_root_for_chain function, build_partial_root_examples function, create_result_paths function
- **Issue**: Original plan used warning + fallback pattern which could hide bugs where partial roots are missing
- **Resolution**: Changed to use `Error::InvalidState` pattern consistent with rest of codebase. Functions now return `Result<Value>` and `Result<BTreeMap<...>>` instead of `Option`. Missing partial roots cause immediate failure with clear error message identifying which enum and variant chain failed. This is appropriate because missing partial roots indicate a bug in the building algorithm - child enums should always build partial roots before parents need them.
- **Decision**: User identified this should use InvalidState error pattern like other impossible states in the codebase

### IMPLEMENTATION-GAP-1: Missing PathInfo serialization for new fields - **Verdict**: CONFIRMED
- **Status**: APPROVED - Documentation added
- **Location**: Phase 4: Update Output Structure - between PathInfo definition and from_mutation_path_internal
- **Issue**: Plan didn't document how `Vec<VariantName>` serializes to JSON - concern that newtype wrapper might serialize as objects instead of plain strings
- **Resolution**: Added "Serialization Verification" section documenting that serde's default behavior for single-field tuple structs is transparent serialization. `VariantName(String)` automatically serializes as a plain string without needing `#[serde(transparent)]`. This is already validated by `ExampleGroup.applicable_variants` in the existing codebase which produces correct JSON output.
- **Decision**: User requested documentation to prevent confusion about serialization behavior

### TYPE-SYSTEM-3: Missing Hash and Ord derives on VariantName - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Phase 1: Add Storage for Partial Root Examples - Section 1a
- **Issue**: The plan proposes adding Hash, PartialOrd, and Ord derives to VariantName to support BTreeMap and HashSet usage. Current code only has Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize.
- **Reasoning**: This is a REDUNDANT finding. The plan document we're reviewing is a FUTURE plan that hasn't been implemented yet, so it's expected that the current code doesn't have these derives. What matters is whether the PLAN addresses this issue - and it does, identically. The redundancy_check correctly identified this as "REDUNDANT" with "plan_addresses_this: YES_IDENTICAL".
- **Decision**: User elected to skip this recommendation
