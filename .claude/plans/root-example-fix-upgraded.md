# Implementation Plan: root_example_unavailable_reason (Collaborative Mode)

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
   For Python changes:
   ```bash
   ~/.local/bin/basedpyright .claude/scripts/mutation_test/prepare.py
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

### Step 1: Core Type System + Initialization ⏳ PENDING

**Objective:** Add `root_example_unavailable_reason` field to both `EnumPathData` and `PathInfo` structs, and initialize the field in all `EnumPathData` construction sites.

**Why this is atomic:** Adding the field without initializing it at construction sites will cause compilation errors. Must be done together.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` (Phase 1.1, 1.2)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (Phase 6.1 Site 1)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` (Phase 6.1 Site 2)

**Changes:**
1. Add field to `EnumPathData` struct (Phase 1.1)
2. Add field to `PathInfo` struct with serde attribute (Phase 1.2)
3. Initialize field in `build_enum_root_path` (Phase 6.1 Site 1)
4. Initialize field in `build_mutation_path_internal` (Phase 6.1 Site 2)

**Build command:**
```bash
cargo build
```

**Expected result:** Clean compilation, new field added and initialized everywhere

---

### Step 2: Add Analysis Function ⏳ PENDING

**Objective:** Create `analyze_variant_constructibility` function that determines if a variant can be constructed via BRP.

**Why this is safe:** New function, doesn't modify existing code or break anything.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (Phase 2.1)

**Changes:**
Add new function after `build_variant_example_for_chain` (around line 628) that returns `Result<(), String>`.

**Build command:**
```bash
cargo build
```

**Expected result:** Clean compilation, function available but not yet called

---

### Step 3: PartialRootExample Struct ⏳ PENDING

**Objective:** Add `PartialRootExample` struct definition to group example + unavailability reason.

**Why this is safe:** New struct definition, not yet used anywhere.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (Phase 3.0)

**Changes:**
Add struct definition near the top of the file after type aliases (around line 76-80).

**Build command:**
```bash
cargo build
```

**Expected result:** Clean compilation, struct available but not yet used

---

### Step 4: Update build_partial_root_examples ⏳ PENDING

**Objective:** Change `build_partial_root_examples` to return `HashMap<Vec<VariantName>, PartialRootExample>` instead of `HashMap<Vec<VariantName>, Value>`.

**Why this breaks:** Changes the return type, which breaks `ProcessChildrenResult` contract. Step 5 will fix it.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (Phase 3.1, 3.2)

**Changes:**
1. Update return type signature (Phase 3.1)
2. Remove fallback logic (Phase 3.2 DELETE)
3. Add variant constructibility analysis (Phase 3.2 REPLACE)
4. Use hierarchical reason selection for nested chains

**Build command:**
```bash
cargo build
```

**Expected result:** ❌ Compilation errors (type mismatch in `ProcessChildrenResult`) - this is expected, Step 5 will fix

---

### Step 5: Propagate Through Call Stack ⏳ PENDING

**Objective:** Update 6 function signatures to use `PartialRootExample`, completing the atomic group started in Step 4.

**Why this fixes Step 4:** Completes the type propagation chain, fixing all compilation errors.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (Phase 4.1-4.5)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` (Phase 4.6)

**Changes:**
Follow Phase 4.0 call flow diagram to update:
1. `ProcessChildrenResult` type (4.1)
2. `process_signature_groups` (4.2)
3. `process_enum` (4.3)
4. `create_enum_mutation_paths` (4.4)
5. `propagate_partial_root_examples_to_children` (4.5)
6. `support::populate_root_examples_from_partials` (4.6)

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, type propagation complete

**Verification:** Use Phase 4.0 checklist to confirm all 6 functions updated

---

### Step 6: JSON Serialization Updates ⏳ PENDING

**Objective:** Update serialization functions to expose `root_example_unavailable_reason` through JSON API.

**Why this is safe:** All callers are in the same file, changes are localized.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` (Phase 5.1, 5.2)

**Changes:**
1. Update `resolve_enum_data_mut` to return 4-tuple with new field (5.1)
2. Update `into_mutation_path_external` to extract and use new field (5.2)

**Build command:**
```bash
cargo build
```

**Expected result:** Clean compilation, new field exposed in JSON

---

### Step 7: Python Integration ⏳ PENDING

**Objective:** Add Python TypedDict field and filtering logic to exclude unconstructible paths from mutation tests.

**Why this is independent:** Python changes don't affect Rust compilation.

**Files to modify:**
- `.claude/scripts/mutation_test/prepare.py` (Phase 7.0, 7.1)

**Changes:**
1. Add `root_example_unavailable_reason` to `PathInfo` TypedDict (7.0)
2. Add filtering logic after line 1022 (7.1)

**Build command:**
```bash
~/.local/bin/basedpyright .claude/scripts/mutation_test/prepare.py
```

**Expected result:** Zero errors, zero warnings

---

### Step 8: Complete Validation ⏳ PENDING

**Objective:** Run comprehensive testing to verify all changes work correctly.

**Files to test:**
- Manual verification with type guide generation (8.1)
- Python type checking (8.2)
- Mutation test validation (8.3)
- Regression testing (8.4)

**Validation steps:**
1. Launch test app: `mcp__brp__brp_launch_bevy_example --target=extras_plugin --profile=debug`
2. Get type guide: `mcp__brp__brp_type_guide --types='["extras_plugin::TestMixedMutabilityEnum"]'`
3. Verify `.value` path shows correct `root_example` and `root_example_unavailable_reason`
4. Run `/create_mutation_test_json` to regenerate test plans
5. Run `python3 .claude/scripts/mutation_test/prepare.py` and verify filtering output
6. Run mutation tests: `.claude/commands/mutation_test.sh`
7. Test regression with other enum types (Option, Result, Handle)

**Expected result:** All tests pass, no regressions

---

## IMPLEMENTATION DETAILS

### Phase 1: Core Type System Changes

#### 1.1 Update `EnumPathData` struct
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:213-226`

```rust
#[derive(Debug, Clone)]
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<Value>,
    /// Explanation for why root_example cannot be used to construct this variant via BRP.
    /// Only populated for PartiallyMutable/NotMutable variants.
    pub root_example_unavailable_reason: Option<String>,  // NEW
}
```

#### 1.2 Update `PathInfo` struct
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs:172-197`

Add after `root_example` field (line 196):
```rust
/// Explanation for why root_example cannot be used to construct the required variant
#[serde(skip_serializing_if = "Option::is_none")]
pub root_example_unavailable_reason: Option<String>,  // NEW
```

**IMPORTANT:** The `#[serde(skip_serializing_if = "Option::is_none")]` attribute is required to match the existing pattern for all other `Option` fields in this struct. Without this attribute, the field would serialize as `null` in JSON when None, instead of being omitted entirely. This maintains API consistency - consumers expect optional fields to be absent (not present with null value) when they don't apply.

**Architecture Note:** The field exists in both structs following the existing pattern for `root_example`:
- **EnumPathData** (1.1): Internal representation during type guide generation
- **PathInfo** (1.2): External API representation serialized to JSON

The field is populated in `EnumPathData` during processing (Phase 4), then mapped to `PathInfo` during serialization (Phase 5). This separation allows internal processing logic to remain strongly typed while the external API provides the JSON schema expected by MCP clients.

---

### Phase 2: Variant Constructibility Analysis

#### 2.1 Create analysis function
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Add after `build_variant_example_for_chain` (around line 628):

```rust
/// Analyze if a variant can be constructed via BRP and build detailed reason if not
///
/// Returns `Ok(())` if variant is constructible (Mutable variants, Unit variants)
/// Returns `Err(reason)` if variant cannot be constructed, with human-readable explanation
///
/// For PartiallyMutable variants, collects actual reasons from NotMutable child fields.
/// For NotMutable variants, indicates all fields are problematic.
fn analyze_variant_constructibility(
    variant_name: &VariantName,
    signature: &VariantSignature,
    mutability: Mutability,
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Result<(), String> {
    // Unit variants are always constructible (no fields to serialize)
    if matches!(signature, VariantSignature::Unit) {
        return Ok(());
    }

    // Fully Mutable variants are constructible
    if matches!(mutability, Mutability::Mutable) {
        return Ok(());
    }

    // NotMutable variants - all fields are problematic
    if matches!(mutability, Mutability::NotMutable) {
        return Err(format!(
            "Cannot construct {} variant via BRP - all fields are non-mutable. \
            This variant cannot be mutated via BRP.",
            variant_name.short_name()
        ));
    }

    // PartiallyMutable variants - collect NotMutable field reasons
    // NOTE: mutability_reason is now Option<NotMutableReason> (typed enum after prerequisite).
    let not_mutable_details: Vec<String> = child_paths
        .iter()
        .filter(|p| p.is_direct_child_at_depth(*ctx.depth))
        .filter(|p| matches!(p.mutability, Mutability::NotMutable))
        .map(|p| {
            let descriptor = p.path_kind.to_mutation_path_descriptor();
            let type_name = p.type_name.short_name();

            // Extract the actual reason from mutability_reason if available
            // With typed enum, we can simply format it directly
            let reason_detail = p.mutability_reason
                .as_ref()
                .map(|reason| format!("{reason}"))
                .unwrap_or_else(|| "unknown reason".to_string());

            format!("{descriptor} ({type_name}): {reason_detail}")
        })
        .collect();

    if not_mutable_details.is_empty() {
        // Shouldn't happen for PartiallyMutable, but handle gracefully
        return Ok(());
    }

    let field_list = not_mutable_details.join("; ");

    Err(format!(
        "Cannot construct {} variant via BRP due to non-mutable fields: {}. \
        This variant's mutable fields can only be mutated if the entity is \
        already set to this variant by game code.",
        variant_name.short_name(),
        field_list
    ))
}
```

**Rationale:**
- Collects actual NotMutable reasons from child paths
- Handles all mutability cases: Unit, Mutable, PartiallyMutable, NotMutable
- Provides detailed, actionable error messages
- No assumptions about Arc fields - uses actual mutability_reason
- Uses typed `NotMutableReason` enum for type-safe extraction (from completed prerequisite)

**Design Choice - Result<(), String> vs Option<String>:**
The function returns `Result<(), String>` instead of `Option<String>` for idiomatic Rust error handling:
- **Result** semantics: The function represents an analysis operation that succeeds (variant is constructible) or fails with a reason (variant is not constructible)
- **Consistency**: Matches Rust convention of using Result for fallible operations
- **Call site clarity**: `.err()` at the call site (Phase 3.2 line 283) explicitly shows we're extracting the error case to use as documentation
- **Type safety**: Result<(), String> makes it clear at the type level that Ok means "no reason needed" while Err contains the explanation
- The alternative `Option<String>` would work but loses the semantic distinction between "analysis succeeded" (Ok(())) vs "analysis found issues" (Err(reason))

---

### Phase 3: Remove Fallback and Build Reasons

#### 3.0 Add PartialRootExample struct

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Add this struct definition near the top of the file, after the type aliases (around line 76-80):

```rust
/// Data for a partial root example including construction feasibility
///
/// Stores both the JSON example for a variant chain and an optional explanation
/// for why that variant cannot be constructed via BRP spawn/insert operations.
#[derive(Debug, Clone)]
struct PartialRootExample {
    /// Complete root example for this variant chain
    example: Value,
    /// Explanation for why this variant cannot be constructed via BRP.
    /// Only populated for PartiallyMutable/NotMutable variants.
    unavailable_reason: Option<String>,
}
```

**Rationale:** Grouping related data (example + its unavailability reason) in a struct is more idiomatic than two parallel HashMaps. This ensures keys cannot diverge, simplifies call sites (single hash lookup instead of two), and makes the code more maintainable.

#### 3.1 Update `build_partial_root_examples` signature
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs:549-609`

Change return type (line 549):
```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, PartialRootExample>
```

#### 3.2 Remove fallback and always build variant-specific examples
**File:** Same file

**Understanding the loop structure** (lines 558-606):
```rust
for (signature, variants) in variant_groups.sorted() {      // Line 558 - OUTER LOOP
    for variant_name in variants {                          // Line 559 - INNER LOOP
        let mut this_variant_chain = ctx.variant_chain.clone();
        this_variant_chain.push(variant_name.clone());

        // Lines 565-570: BUGGY FALLBACK (DELETE THIS)
        // Lines 575-604: Nested chain logic (REPLACE THIS)

    } // End inner loop
} // End outer loop
```

**DELETE lines 565-570** (the incorrect fallback - inside inner loop):
```rust
// REMOVE THIS:
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone())
    .or_else(|| select_preferred_example(enum_examples))
    .unwrap_or(json!(null));
```

**REPLACE lines 565-604** (all code after `this_variant_chain` creation) with:
```rust
// Find this variant's mutability status
// DEFENSIVE: This lookup should always succeed because enum_examples is built by
// iterating over variant_groups (see process_signature_groups line 408-449), so
// every variant in variant_groups is guaranteed to exist in enum_examples.
// The unwrap_or fallback handles theoretical future refactoring errors by treating
// unknown variants as NotMutable (safest choice - prevents construction attempts).
let variant_mutability = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .map(|ex| ex.mutability)
    .unwrap_or(Mutability::NotMutable);

// Determine if this variant can be constructed via BRP
let unavailable_reason = analyze_variant_constructibility(
    variant_name,
    signature,
    variant_mutability,
    child_mutation_paths,
    ctx,
).err();

// Find all deeper nested chains that extend this variant
let nested_enum_chains =
    collect_child_chains_to_wrap(child_mutation_paths, &this_variant_chain, ctx);

// Build root examples for each nested enum chain
for nested_chain in &nested_enum_chains {
    let example = build_variant_example_for_chain(
        signature,
        variant_name,
        child_mutation_paths,
        nested_chain,
        ctx,
    );

    // Determine unavailability reason using hierarchical selection
    // RATIONALE: We need to capture the ACTUAL blocking issue:
    // 1. If parent is unconstructible → parent's reason is the blocker (child is unreachable)
    // 2. If parent IS constructible → check if nested chain has its OWN unavailability reason
    //
    // Example: Parent "Multiple" variant has Arc fields (unconstructible), nested chains
    // inherit this reason because you can't reach them via spawn/insert.
    //
    // Counter-example: Parent "Good" variant is Mutable (constructible), but contains a
    // child enum with an unconstructible variant (e.g., Arc fields). The nested chain
    // needs the CHILD's unavailability reason, not None from the constructible parent.
    let nested_chain_reason = if unavailable_reason.is_some() {
        // Parent is unconstructible → child is unreachable, use parent's reason
        unavailable_reason.clone()
    } else {
        // Parent is constructible → check if THIS nested chain is unconstructible
        // The child enum was already processed recursively and its enum_path_data
        // was populated with root_example_unavailable_reason. Look it up.
        child_mutation_paths
            .iter()
            .find_map(|child| {
                child.enum_path_data
                    .as_ref()
                    .filter(|data| data.variant_chain == *nested_chain)
                    .and_then(|data| data.root_example_unavailable_reason.clone())
            })
    };

    partial_root_examples.insert(
        nested_chain.clone(),
        PartialRootExample {
            example,
            unavailable_reason: nested_chain_reason,
        },
    );
}

// Build root example for this variant's chain itself
let example = build_variant_example_for_chain(
    signature,
    variant_name,
    child_mutation_paths,
    &this_variant_chain,
    ctx,
);

partial_root_examples.insert(
    this_variant_chain,
    PartialRootExample {
        example,
        unavailable_reason,
    },
);
```

Initialize HashMap at start (line 555):
```rust
let mut partial_root_examples = HashMap::new();
```

Return at end (line 607):
```rust
partial_root_examples
```

**Rationale:**
- Removes incorrect fallback causing the bug
- Always builds variant-specific root_example
- Analyzes each variant's constructibility
- Stores both examples and reasons in single struct (type-safe, cannot diverge)
- Propagates reasons to nested chains
- Uses idiomatic Rust pattern (grouped data in struct, not parallel collections)

---

### Phase 4: Propagation Through Call Stack

#### 4.0 Overview: Call Flow and Verification

**Purpose:** Thread the new `PartialRootExample` type through the entire call stack, replacing `HashMap<Vec<VariantName>, Value>` with `HashMap<Vec<VariantName>, PartialRootExample>`.

**Call Flow Diagram:**
```
build_partial_root_examples (Phase 3)
    ↓ returns HashMap<Vec<VariantName>, PartialRootExample>
    ↓
process_signature_groups (4.2)
    ↓ returns ProcessChildrenResult (updated in 4.1)
    ↓        = (Vec<ExampleGroup>, Vec<MutationPathInternal>, HashMap<..., PartialRootExample>)
    ↓
process_enum (4.3)
    ↓ destructures ProcessChildrenResult, extracts partial_root_examples
    ↓ passes to ↓
    ↓
create_enum_mutation_paths (4.4)
    ↓ accepts HashMap<..., PartialRootExample> parameter
    ↓ passes to ↓
    ↓
propagate_partial_root_examples_to_children (4.5)
    ↓ accepts HashMap<..., PartialRootExample> parameter
    ↓ passes to ↓
    ↓
support::populate_root_examples_from_partials (4.6)
    ↓ accepts HashMap<..., PartialRootExample> parameter
    ↓ looks up PartialRootExample, extracts .example and .unavailable_reason
    ↓ populates both fields in EnumPathData
```

**Breaking Change:** The type alias `ProcessChildrenResult` changes its third element from `HashMap<Vec<VariantName>, Value>` to `HashMap<Vec<VariantName>, PartialRootExample>`. This cascades through 6 functions.

**Verification Checklist:**
After implementing Phase 4:
1. ✅ `ProcessChildrenResult` type alias updated (4.1)
2. ✅ `process_signature_groups` return statement updated (4.2)
3. ✅ `process_enum` destructuring and call updated (4.3)
4. ✅ `create_enum_mutation_paths` parameter and call updated (4.4)
5. ✅ `propagate_partial_root_examples_to_children` parameter and call updated (4.5)
6. ✅ `support::populate_root_examples_from_partials` parameter and field access updated (4.6)
7. ✅ Run `cargo build` - should compile with no type errors
8. ✅ Check for any other functions that reference `ProcessChildrenResult` or `partial_root_examples`

**Expected Compilation Behavior:**
- If ANY function signature is missed: Type mismatch error at call site
- If field access is wrong in 4.6: Compilation error (cannot access `.example` on `Value`)
- When all updates complete: Clean compilation with no errors

#### 4.1 Update `ProcessChildrenResult` type
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs:76-80`

**Current:**
```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, Value>,  // OLD: Just Value
);
```

**Change to:**
```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, PartialRootExample>,  // NEW: PartialRootExample struct
);
```

**IMPORTANT:** This is a breaking change. The HashMap value type changes from `Value` (JSON) to `PartialRootExample` (struct containing both example and unavailability reason). All functions that return or consume `ProcessChildrenResult` must be updated accordingly (covered in phases 4.2-4.6).

#### 4.2 Update `process_signature_groups`
**File:** Same file, lines 400-460

Change line 456:
```rust
let partial_root_examples =
    build_partial_root_examples(variant_groups, &examples, &child_mutation_paths, ctx);
```

Change return (line 459):
```rust
Ok((examples, child_mutation_paths, partial_root_examples))
```

#### 4.3 Update `process_enum`
**File:** Same file, lines 87-128

Change line 101:
```rust
let (enum_examples, child_mutation_paths, partial_root_examples) =
    process_signature_groups(&variants_grouped_by_signature, ctx)?;
```

Change line 121:
```rust
Ok(create_enum_mutation_paths(
    ctx,
    enum_examples,
    default_example,
    child_mutation_paths,
    partial_root_examples,
))
```

#### 4.4 Update `create_enum_mutation_paths`
**File:** Same file, lines 724-766

Update parameter (line 724):
```rust
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED
) -> Vec<MutationPathInternal>
```

Update call (line 756):
```rust
propagate_partial_root_examples_to_children(
    &mut child_mutation_paths,
    &partial_root_examples,
    ctx,
);
```

#### 4.5 Update `propagate_partial_root_examples_to_children`
**File:** Same file, lines 707-721

Update parameter:
```rust
fn propagate_partial_root_examples_to_children(
    child_paths: &mut [MutationPathInternal],
    partial_root_examples: &HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED
    ctx: &RecursionContext,
)
```

Update call (line 719):
```rust
support::populate_root_examples_from_partials(
    child_paths,
    partial_root_examples,
);
```

#### 4.6 Update `support::populate_root_examples_from_partials`
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs:158-176`

```rust
pub fn populate_root_examples_from_partials(
    paths: &mut [MutationPathInternal],
    partials: &HashMap<Vec<VariantName>, PartialRootExample>,  // UPDATED
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
        {
            // Populate both fields from the struct (single lookup!)
            if let Some(data) = partials.get(&enum_data.variant_chain) {
                enum_data.root_example = Some(data.example.clone());
                enum_data.root_example_unavailable_reason = data.unavailable_reason.clone();
            }
        }
    }
}
```

---

### Phase 5: JSON Serialization

#### 5.1 Update `resolve_enum_data_mut`
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs:179-205`

Change return type (line 181):
```rust
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<Value>,            // root_example
    Option<String>,           // root_example_unavailable_reason (NEW)
)
```

Change early return (line 186):
```rust
return (None, None, None, None);
```

Change map_or (line 189-204):
```rust
self.enum_path_data
    .take()
    .map_or((None, None, None, None), |enum_data| {
        let instructions = Some(format!(
            "First, set the root mutation path to 'root_example', then you can mutate the '{}' path. See 'applicable_variants' for which variants support this field.",
            &self.mutation_path
        ));

        let variants = if enum_data.applicable_variants.is_empty() {
            None
        } else {
            Some(enum_data.applicable_variants)
        };

        (
            instructions,
            variants,
            enum_data.root_example,
            enum_data.root_example_unavailable_reason,  // NEW
        )
    })
```

#### 5.2 Update `into_mutation_path_external`
**File:** Same file, lines 76-110

Update extraction (line 94):
```rust
let (enum_instructions, applicable_variants, root_example, root_example_unavailable_reason) =
    self.resolve_enum_data_mut();
```

Update struct creation (lines 96-109):
```rust
MutationPathExternal {
    description,
    path_info: PathInfo {
        path_kind: self.path_kind,
        type_name: self.type_name,
        type_kind,
        mutability: self.mutability,
        mutability_reason: self.mutability_reason,
        enum_instructions,
        applicable_variants,
        root_example,
        root_example_unavailable_reason,  // NEW
    },
    path_example,
}
```

---

### Phase 6: Initialization

#### 6.1 Initialize new field in EnumPathData construction

**IMPORTANT:** There are TWO construction sites that must be updated.

##### Site 1: enum_path_builder.rs
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Find `build_enum_root_path` (around line 679-687):
```rust
let enum_path_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain:       ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        root_example:        None,
        root_example_unavailable_reason: None,  // NEW
    })
};
```

##### Site 2: path_builder.rs
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Find `build_mutation_path_internal` (around line 416-424):
```rust
let enum_path_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain:       ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        root_example:        None,
        root_example_unavailable_reason: None,  // NEW
    })
};
```

**Rationale:** Both functions construct `EnumPathData` for nested enum scenarios. Site 1 handles enum-within-enum cases, Site 2 handles all other types nested within enums (structs, tuples, etc.). Both must initialize the new field or compilation will fail after Phase 1.1.

---

### Phase 7: Mutation Test Integration

#### 7.0 Update `PathInfo` TypedDict
**File:** `.claude/scripts/mutation_test/prepare.py:51-55`

Add new field to TypedDict:
```python
class PathInfo(TypedDict, total=False):
    """Path metadata including mutability and root examples."""

    mutability: str
    root_example: object
    root_example_unavailable_reason: str  # NEW - optional field
```

**IMPORTANT - Optional Field Semantics:**
- The `total=False` parameter makes ALL fields in this TypedDict optional (may be absent from the dict)
- `root_example_unavailable_reason` follows this pattern - it's only present for paths with unconstructible variants
- When a field is optional, you MUST use `"field_name" in dict` to check for existence before accessing
- DO NOT use `if dict.get("field_name"):` or `if dict["field_name"]:` - these will fail if the key is missing

**Rationale:** TypedDict must include all fields that will be accessed in the filtering code. Without this, basedpyright will report type errors when accessing the new field. The field is optional (not always present) because only paths with unconstructible variants will have this field populated.

#### 7.1 Add path filtering logic
**File:** `.claude/scripts/mutation_test/prepare.py`

Add filtering after excluded types removal (after line 1022):

```python
# Filter out paths with unavailable root examples from mutation testing
print("Filtering paths with unavailable root examples...", file=sys.stderr)

for type_name, type_data in list(data["type_guide"].items()):
    mutation_paths: dict[str, MutationPathData] = type_data.get("mutation_paths", {})
    available_paths: dict[str, MutationPathData] = {}
    excluded_count: int = 0

    for path, path_data in mutation_paths.items():
        path_info: PathInfo = path_data.get("path_info", {})

        # Check if root_example is unavailable
        # NOTE: Use 'in' operator to check for key existence because path_info is a TypedDict
        # with total=False (all fields optional). The field is only present for unconstructible
        # variants. Using path_info.get() or direct access would fail for missing keys.
        # This filters ANY path with unconstructibility marker, including nested variant
        # chains that inherit unconstructibility from their parent. This is conservative
        # but correct: we exclude nested mutations even if they would work once game code
        # sets the parent variant, because mutation tests rely on spawn/insert operations.
        if "root_example_unavailable_reason" in path_info:
            excluded_count += 1
            reason_preview = path_info["root_example_unavailable_reason"][:80]
            print(
                f"  Excluding {type_name}{path}: {reason_preview}...",
                file=sys.stderr
            )
        else:
            available_paths[path] = path_data

    # Update type's mutation paths
    if available_paths:
        type_data["mutation_paths"] = available_paths
        if excluded_count > 0:
            print(
                f"  Kept {len(available_paths)} paths, excluded {excluded_count} for {type_name}",
                file=sys.stderr
            )
    else:
        # No testable paths remain - remove entire type
        print(
            f"  Removing {type_name} - no constructible paths remain",
            file=sys.stderr
        )
        del data["type_guide"][type_name]
```

---

### Phase 8: Testing and Validation

#### 8.1 Manual verification checklist

1. **Build and verify compilation:**
   ```bash
   cargo build
   ```

2. **Launch test app:**
   ```bash
   mcp__brp__brp_launch_bevy_example --target=extras_plugin --profile=debug
   ```

3. **Get type guide:**
   ```bash
   mcp__brp__brp_type_guide --types='["extras_plugin::TestMixedMutabilityEnum"]'
   ```

4. **Verify `.value` path shows:**
   ```json
   {
     "path": ".value",
     "applicable_variants": ["TestMixedMutabilityEnum::Multiple"],
     "root_example": {
       "Multiple": {
         "name": "Hello, World!",
         "mixed": {
           "mutable_string": "Hello, World!",
           "mutable_float": 1.0,
           "partially_mutable_nested": {"nested_mutable_value": 1.0}
         },
         "value": 1.0
       }
     },
     "root_example_unavailable_reason": "Cannot construct Multiple variant via BRP due to non-mutable fields: .mixed.not_mutable_arc (Arc<String>): Type bevy_platform::sync::Arc<alloc::string::String> is a leaf type registered in the schema but has no hardcoded example value available for mutations. This variant's mutable fields can only be mutated if the entity is already set to this variant by game code."
   }
   ```

5. **Verify WithMixed variant similarly** - paths like `.0.mutable_float` should have variant-specific root_example

#### 8.2 Python type checking

Verify TypedDict changes pass type checking:
```bash
~/.local/bin/basedpyright .claude/scripts/mutation_test/prepare.py
```

Expected: Zero errors, zero warnings. If you see `reportAny` errors about `PathInfo` or `root_example_unavailable_reason`, the TypedDict update in Phase 7.0 was not applied correctly.

#### 8.3 Mutation test validation

1. Run `/create_mutation_test_json` to regenerate test plans
2. Run prepare.py and verify filtering output:
   ```bash
   python3 .claude/scripts/mutation_test/prepare.py
   ```

   Expected output should include:
   ```
   Filtering paths with unavailable root examples...
     Excluding extras_plugin::TestMixedMutabilityEnum.value: Cannot construct Multiple variant via BRP...
     Kept 3 paths, excluded 2 for extras_plugin::TestMixedMutabilityEnum
   ```

3. Verify TestMixedMutabilityEnum paths for Multiple/WithMixed are filtered
4. Run batch 15 mutation tests:
   ```bash
   .claude/commands/mutation_test.sh
   ```
5. Verify no failures related to variant construction

#### 8.4 Regression testing

Test with other enum types to ensure no regressions:
- `Option` (Mutable variants)
- `Result` (Mutable variants)
- `Handle` (may have PartiallyMutable variants)
- Regular enums without Arc fields

---

## SUPPORTING SECTIONS

### Dependencies

**✅ COMPLETED:** This plan depended on `mutability-reason-type-safety.md`, which has been completed.

That plan refactored `MutationPathInternal.mutability_reason` from `Option<Value>` (JSON) to `Option<NotMutableReason>` (typed enum), which makes Phase 2.1 of this plan significantly simpler and type-safe.

**This plan is now ready for implementation.**

---

### Problem Statement

Enum variants that are PartiallyMutable or NotMutable cannot be constructed via BRP, but their mutable fields should still be documented for entities already in that variant. Currently, these paths show `root_example: "None"` (fallback to wrong variant) causing:

1. **Misleading documentation** - shows wrong variant structure
2. **Mutation test failures** - tries to mutate fields on wrong variant
3. **User confusion** - instructions don't match reality

#### Root Cause

Lines 565-570 in `enum_path_builder.rs::build_partial_root_examples`:
```rust
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone())           // Returns None for PartiallyMutable
    .or_else(|| select_preferred_example(...))    // BUG: Falls back to wrong variant!
    .unwrap_or(json!(null));
```

For `TestMixedMutabilityEnum::Multiple` (PartiallyMutable due to Arc fields):
- Its `example` is `None` (line 332 - no spawn example generated)
- Falls back to `select_preferred_example()` which picks `None` (Unit variant - Mutable)
- Result: Paths like `.value` (only exist on Multiple) get `root_example: "None"`

#### Example Issue

**Current output:**
```json
{
  "path": ".value",
  "applicable_variants": ["TestMixedMutabilityEnum::Multiple"],
  "root_example": "None",  // WRONG - Unit variant, not Multiple!
  "enum_instructions": "First, set root to 'root_example'..."
}
```

**Expected output:**
```json
{
  "path": ".value",
  "applicable_variants": ["TestMixedMutabilityEnum::Multiple"],
  "root_example": {
    "Multiple": {
      "name": "Hello, World!",
      "mixed": {"mutable_string": "test", "mutable_float": 1.0},
      "value": 1.0
    }
  },
  "root_example_unavailable_reason": "Cannot construct Multiple variant via BRP due to non-mutable fields: .mixed.not_mutable_arc (Arc<String>): Type bevy_platform::sync::Arc<alloc::string::String> is a leaf type registered in the schema but has no hardcoded example value available for mutations. This variant's mutable fields can only be mutated if the entity is already set to this variant by game code."
}
```

---

### Solution Overview

1. **Remove fallback logic** - Always build variant-specific root_example
2. **Add new field** - `root_example_unavailable_reason` explaining why variant can't be constructed
3. **Collect actual reasons** - Extract from NotMutable child fields (not assume "Arc")
4. **Filter mutation tests** - Skip unconstructible paths in prepare.py

---

### Expected Outcomes

#### Type Guide Output
1. **root_example** shows correct variant structure (not fallback to wrong variant)
2. **root_example_unavailable_reason** explains why with actual field reasons extracted from `mutability_reason`
3. **Users understand** which fields are problematic and why (e.g., Arc, recursion limit, missing trait)

#### Mutation Testing
1. **Unconstructible paths filtered** during prepare.py execution
2. **No test failures** from trying to construct PartiallyMutable/NotMutable variants
3. **Clear logs** showing what was excluded and why

#### Documentation
1. **Manual users** can see partial structure even if unconstructible
2. **Clear guidance** on when paths are usable (entity already in variant)
3. **Accurate information** about field-level mutability issues

---

### Success Criteria

- [ ] Type guide shows variant-specific root_example for all variants
- [ ] root_example_unavailable_reason explains unconstructible variants with actual reasons
- [ ] Mutation tests skip unconstructible paths
- [ ] No regression in existing enum handling (Option, Result, Handle, etc.)
- [ ] Batch 15 completes without variant construction failures
- [ ] Documentation is clear and actionable for manual users

---

### Rollback Plan

If issues arise:
1. Revert `enum_path_builder.rs` changes (restore fallback at lines 565-570)
2. Remove new fields from `EnumPathData` and `PathInfo`
3. Restore original function signatures
4. Keep prepare.py changes (defensive, won't break existing code)

---

### Files Modified Summary

#### Rust (4 files):
1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Add fields
2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` - Major changes (analysis function, remove fallback, propagation)
3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` - Update serialization
4. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` - Update propagation helper

#### Python (1 file):
5. `.claude/scripts/mutation_test/prepare.py` - Filter unconstructible paths

### Total Estimate:
- Rust implementation: 4-6 hours
- Python integration: 1 hour
- Testing/validation: 2-3 hours
- **Total: 7-10 hours**
