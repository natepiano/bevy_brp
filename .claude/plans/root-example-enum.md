# Implementation Plan: root_example as RootExample Enum

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

### Step 1: Define RootExample Enum ✅ COMPLETED

**Objective:** Create the `RootExample` enum that will replace the two separate fields (`root_example` and `root_example_unavailable_reason`).

**Why this is safe:** New enum definition, not yet used anywhere.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes:**
Add enum definition near the top of the file (after imports, before structs):

```rust
/// Root example for an enum variant, either available for construction or unavailable with reason
///
/// Serializes to JSON as either:
/// - `{"root_example": <value>}` for Available variant
/// - `{"root_example_unavailable_reason": "<reason>"}` for Unavailable variant
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum RootExample {
    /// Variant can be constructed via BRP spawn/insert operations
    Available(Value),
    /// Variant cannot be constructed via BRP, with explanation
    Unavailable {
        #[serde(rename = "root_example_unavailable_reason")]
        reason: String,
    },
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** Clean compilation, enum available but not yet used

---

### Step 2: Add new_root_example Field to EnumPathData ✅ COMPLETED

**Objective:** Add `new_root_example: Option<RootExample>` as a NEW field alongside existing fields, allowing parallel migration.

**Why this is safe:** Keeps old system working while building new system. Both can coexist during migration.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes:**

**Current (lines 130-142):**
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<Value>,
    pub root_example_unavailable_reason: Option<String>,
}
```

**Changed to (lines 130-151):**
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<Value>,  // OLD - keep for now
    pub root_example_unavailable_reason: Option<String>,  // OLD - keep for now
    /// New root example enum (will replace the two fields above)
    /// Available: Complete root example for this specific variant chain
    /// Unavailable: Explanation for why root_example cannot be used to construct variant
    pub new_root_example: Option<RootExample>,  // NEW
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation - new field added, old system still works

---

### Step 3: Initialize new_root_example at Construction Sites ✅ COMPLETED

**Objective:** Add `new_root_example: None` initialization at all `EnumPathData` construction sites.

**Why this is needed:** New field must be initialized when struct is constructed.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

**Changes:**

##### Site 1: `enum_path_builder.rs`
Find `build_enum_root_path` (around line 679-687):

```rust
let enum_path_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain:       ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        root_example:        None,
        root_example_unavailable_reason: None,
        new_root_example:    None,  // NEW - initialize
    })
};
```

##### Site 2: `path_builder.rs`
Find `build_mutation_path_internal` (around line 416-424):

```rust
let enum_path_data = if ctx.variant_chain.is_empty() {
    None
} else {
    Some(EnumPathData {
        variant_chain:       ctx.variant_chain.clone(),
        applicable_variants: Vec::new(),
        root_example:        None,
        root_example_unavailable_reason: None,
        new_root_example:    None,  // NEW - initialize
    })
};
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation - initialization complete, both systems ready

---

### Step 4: Add Variant Constructibility Analysis Function ✅ COMPLETED (Already Implemented)

**Objective:** Create `analyze_variant_constructibility` function that determines if a variant can be constructed via BRP.

**Status:** Function already exists at lines 712-809 in `enum_path_builder.rs` and is already being called at line 597-604 in `build_partial_root_examples`.

**Implementation details:**
- Returns `Result<(), String>` as designed
- Handles all mutability cases (Unit, Mutable, NotMutable, PartiallyMutable)
- Collects actual field-level reasons from child paths
- Generates detailed, human-readable error messages
- Already integrated into the codebase

**No action needed** - proceed to Step 5.

**Original specification for reference:**
Add after `build_variant_example_for_chain` (around line 628 - NOTE: actual location is line 712):

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
            "Cannot construct {variant_name} variant via BRP - all fields are non-mutable. \
            This variant cannot be mutated via BRP."
        ));
    }

    // PartiallyMutable variants - collect NotMutable field reasons
    let not_mutable_details: Vec<String> = child_paths
        .iter()
        .filter(|p| p.is_direct_child_at_depth(*ctx.depth))
        .filter(|p| matches!(p.mutability, Mutability::NotMutable))
        .map(|p| {
            let descriptor = p.path_kind.to_mutation_path_descriptor();
            let type_name = p.type_name.short_name();

            let reason_detail = p.mutability_reason
                .as_ref()
                .map(|reason| format!("{reason}"))
                .unwrap_or_else(|| "unknown reason".to_string());

            format!("{descriptor} ({type_name}): {reason_detail}")
        })
        .collect();

    if not_mutable_details.is_empty() {
        return Ok(());
    }

    let field_list = not_mutable_details.join("; ");

    Err(format!(
        "Cannot construct {variant_name} variant via BRP due to non-mutable fields: {field_list}. \
        This variant's mutable fields can only be mutated if the entity is \
        already set to this variant by game code."
    ))
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, function available but not yet called

---

### Step 5: Build New Partial Root Examples HashMap ✅ COMPLETED

**Objective:** Update `build_partial_root_examples` to build BOTH the old HashMap (with `PartialRootExample` struct) AND a new HashMap with `RootExample` enum values.

**Why parallel approach:** Keeps old system working while building new system alongside. Can validate before cutover.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Changes:**

##### Update signature to return tuple (lines 562-567):

**CURRENT:**
```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, PartialRootExample> {
```

**CHANGE TO:**
```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> (
    HashMap<Vec<VariantName>, PartialRootExample>,  // OLD - struct with example + reason
    HashMap<Vec<VariantName>, RootExample>,          // NEW - enum (Available/Unavailable)
) {
```

##### Initialize BOTH HashMaps (line 568):

**CURRENT:**
```rust
let mut partial_root_examples = HashMap::new();
```

**CHANGE TO:**
```rust
let mut partial_root_examples = HashMap::new();      // OLD system - PartialRootExample struct
let mut new_partial_root_examples = HashMap::new();  // NEW system - RootExample enum
```

##### Update nested chain insertion (after line 656):

The current code at lines 650-656 inserts PartialRootExample for nested chains:
```rust
partial_root_examples.insert(
    nested_chain.clone(),
    PartialRootExample {
        example,
        unavailable_reason: nested_chain_reason,
    },
);
```

**ADD after line 656 (NEW system insertion):**
```rust
// NEW system: Build RootExample enum from the same data
let new_root_example = match nested_chain_reason {
    Some(reason) => RootExample::Unavailable { reason },
    None => RootExample::Available(example.clone()),
};
new_partial_root_examples.insert(nested_chain.clone(), new_root_example);
```

##### Update this variant's chain insertion (after line 680):

The current code at lines 674-680 inserts PartialRootExample for this variant:
```rust
partial_root_examples.insert(
    this_variant_chain,
    PartialRootExample {
        example,
        unavailable_reason,
    },
);
```

**ADD after line 680 (NEW system insertion):**
```rust
// NEW system: Build RootExample enum from the same data
let new_root_example = match unavailable_reason.clone() {
    Some(reason) => RootExample::Unavailable { reason },
    None => RootExample::Available(example.clone()),
};
new_partial_root_examples.insert(this_variant_chain.clone(), new_root_example);
```

##### Return tuple of BOTH HashMaps (line 684):

**CURRENT:**
```rust
partial_root_examples
```

**CHANGE TO:**
```rust
(partial_root_examples, new_partial_root_examples)
```

**Build command:**
```bash
cargo build
```

**Expected result:** ❌ Compilation errors (return type mismatch in `ProcessChildrenResult`) - this is expected, Step 6 will fix

---

### Step 6: Expand ProcessChildrenResult to 4-Tuple ✅ COMPLETED

**Objective:** Update `ProcessChildrenResult` type alias to include BOTH HashMaps (old and new), making it a 4-tuple.

**Why this is needed:** Allows both systems to propagate through call stack in parallel.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Changes:**

#### Update `ProcessChildrenResult` type (line 76-80):
**Current:**
```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, Value>,  // OLD system only
);
```

**Change to:**
```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, Value>,        // OLD system
    HashMap<Vec<VariantName>, RootExample>,  // NEW system
);
```

**Build command:**
```bash
cargo build
```

**Expected result:** ❌ Compilation errors (all call sites need updating) - Step 7 will fix

---

### Step 7: Propagate Both HashMaps Through Call Stack ✅ COMPLETED

**Objective:** Update all function signatures and call sites to pass both HashMaps through the call stack.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**Changes:**

#### 7.1 Update `process_signature_groups` (lines 400-460): ✅ COMPLETED
Destructure and return BOTH HashMaps (line 456-459):
```rust
let (partial_root_examples, new_partial_root_examples) =
    build_partial_root_examples(variant_groups, &examples, &child_mutation_paths, ctx);

Ok((examples, child_mutation_paths, partial_root_examples, new_partial_root_examples))
```

#### 7.2 Update `process_enum` (lines 87-128):
Destructuring (line 101):
```rust
let (enum_examples, child_mutation_paths, partial_root_examples, new_partial_root_examples) =
    process_signature_groups(&variants_grouped_by_signature, ctx)?;
```

Call (line 121):  ✅ COMPLETED
```rust
Ok(create_enum_mutation_paths(
    ctx,
    enum_examples,
    default_example,
    child_mutation_paths,
    partial_root_examples,
    new_partial_root_examples,
))
```

#### 7.3 Update `create_enum_mutation_paths` (lines 724-766): ✅ COMPLETED
Add second HashMap parameter (line 724):
```rust
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, Value>,        // OLD
    new_partial_root_examples: HashMap<Vec<VariantName>, RootExample>,  // NEW
) -> Vec<MutationPathInternal>
```

Update call (line 756):
```rust
propagate_partial_root_examples_to_children( ✅ COMPLETED
    &mut child_mutation_paths,
    &partial_root_examples,
    &new_partial_root_examples,
    ctx,
);
```

#### 7.4 Update `propagate_partial_root_examples_to_children` (lines 707-721): ✅ COMPLETED
Add second HashMap parameter:
```rust
fn propagate_partial_root_examples_to_children(
    child_paths: &mut [MutationPathInternal],
    partial_root_examples: &HashMap<Vec<VariantName>, Value>,        // OLD
    new_partial_root_examples: &HashMap<Vec<VariantName>, RootExample>,  // NEW
    ctx: &RecursionContext,
)
```

Update call (line 719): ✅ COMPLETED
```rust
support::populate_root_examples_from_partials(
    child_paths,
    partial_root_examples,
    new_partial_root_examples,
);
```

#### 7.5 Update `support::populate_root_examples_from_partials` ✅ COMPLETED
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs:158-176`

Update doc comment and add second HashMap parameter to populate BOTH fields:
```rust
/// Populate `root_example` and `new_root_example` from partial root HashMaps for enum paths
///
/// Extracts specific variant chain entries from the HashMaps into `enum_path_data` fields
/// for final serialization. The HashMaps contain entries for all variant chains assembled
/// at the parent level; this function looks up each path's specific chain and populates:
/// - OLD system: `root_example` + `root_example_unavailable_reason` from `partials`
/// - NEW system: `new_root_example` from `new_partials`
///
/// This is shared between `path_builder.rs` and `enum_path_builder.rs` to avoid code duplication.
pub fn populate_root_examples_from_partials(
    paths: &mut [MutationPathInternal],
    partials: &HashMap<Vec<VariantName>, PartialRootExample>,  // OLD
    new_partials: &HashMap<Vec<VariantName>, RootExample>,     // NEW
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

            // NEW system: Populate new field
            if let Some(root_example) = new_partials.get(&enum_data.variant_chain) {
                enum_data.new_root_example = Some(root_example.clone());
            }
        }
    }
}
```

#### 7.6 Add `new_partial_root_examples` field to `MutationPathInternal` ✅ COMPLETED
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

Add new field after `partial_root_examples`:
```rust
pub struct MutationPathInternal {
    // ... existing fields ...

    /// Maps variant chains to complete root examples for reaching nested enum paths.
    /// OLD system - stores Value directly
    pub partial_root_examples: Option<HashMap<Vec<VariantName>, Value>>,

    /// Maps variant chains to RootExample enum (Available/Unavailable)
    /// NEW system - replaces partial_root_examples
    pub new_partial_root_examples: Option<HashMap<Vec<VariantName>, RootExample>>,
}
```

#### 7.7 Update `path_builder.rs::assemble_partial_root_examples` ✅ COMPLETED
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs:441-530`

Update return type and build BOTH HashMaps:
```rust
fn assemble_partial_root_examples(
    builder: &B,
    ctx: &RecursionContext,
    child_paths: &[&MutationPathInternal],
) -> std::result::Result<
    (
        Option<HashMap<Vec<VariantName>, Value>>,       // OLD
        Option<HashMap<Vec<VariantName>, RootExample>>, // NEW
    ),
    BuilderError,
> {
    // ... early returns for Map/Set with NotMutable children ...

    if all_chains.is_empty() {
        return Ok((None, None));
    }

    let mut assembled_partial_root_examples = HashMap::new();
    let mut new_assembled_partial_root_examples = HashMap::new();

    // For each variant chain, assemble wrapped example from compatible children
    for chain in all_chains {
        let examples_for_chain =
            support::collect_children_for_chain(child_paths, ctx, Some(&chain));

        let assembled = builder.assemble_from_children(ctx, examples_for_chain)?;

        // OLD system
        assembled_partial_root_examples.insert(chain.clone(), assembled.clone());

        // NEW system: Check if any child has Unavailable for this chain
        let mut unavailable_reason = None;
        for child in child_paths {
            if let Some(child_new_partials) = &child.new_partial_root_examples {
                if let Some(RootExample::Unavailable { reason }) = child_new_partials.get(&chain) {
                    unavailable_reason = Some(reason.clone());
                    break;
                }
            }
        }

        let new_root_example = match unavailable_reason {
            Some(reason) => RootExample::Unavailable { reason },
            None => RootExample::Available(assembled),
        };

        new_assembled_partial_root_examples.insert(chain, new_root_example);
    }

    Ok((
        Some(assembled_partial_root_examples),
        Some(new_assembled_partial_root_examples),
    ))
}
```

#### 7.8 Update `path_builder.rs::build_final_result` ✅ COMPLETED
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs:513-570`

Add new parameter and propagate both HashMaps:
```rust
fn build_final_result(
    ctx: &RecursionContext,
    mut paths_to_expose: Vec<MutationPathInternal>,
    example_to_use: Value,
    parent_status: Mutability,
    mutability_reason: Option<NotMutableReason>,
    partial_root_examples: Option<HashMap<Vec<VariantName>, Value>>,
    new_partial_root_examples: Option<HashMap<Vec<VariantName>, RootExample>>,  // NEW parameter
) -> Vec<MutationPathInternal> {
    if let Some(ref partials) = partial_root_examples {
        // Propagate assembled partial_root_examples to all children (OLD system)
        for child in &mut paths_to_expose {
            child.partial_root_examples = Some(partials.clone());
        }

        // Propagate assembled new_partial_root_examples to all children (NEW system)
        if let Some(ref new_partials) = new_partial_root_examples {
            for child in &mut paths_to_expose {
                child.new_partial_root_examples = Some(new_partials.clone());
            }
        }

        // Convert Value partials to PartialRootExample for populate function
        let partials_with_reasons: HashMap<Vec<VariantName>, PartialRootExample> = partials
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    PartialRootExample {
                        example: v.clone(),
                        unavailable_reason: None,
                    },
                )
            })
            .collect();

        // Populate root_example from partial_root_examples for children with enum_path_data
        support::populate_root_examples_from_partials(
            &mut paths_to_expose,
            &partials_with_reasons,
            new_partial_root_examples.as_ref().unwrap_or(&HashMap::new()),
        );
    }

    // ... rest of function ...
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, both systems propagating through call stack

---

### Step 8: Add new_root_example to PathInfo ✅ COMPLETED

**Objective:** Add `new_root_example` field to PathInfo alongside existing fields.

**Why parallel:** Keep old serialization working while adding new field for testing.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes:**

Add new field to PathInfo struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    // ... existing fields ...

    // OLD system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example: Option<Value>,

    // NEW system (will replace the two old fields)
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub new_root_example: Option<RootExample>,
}
```

**Rationale:**
- `#[serde(flatten)]` makes enum fields appear at PathInfo level
- When `Available(value)`: serializes as `"root_example": <value>`  (CONFLICTS with old field name!)
- When `Unavailable { reason }`: serializes as `"root_example_unavailable_reason": "<reason>"`
- Temporarily we'll have both `root_example` (old) and potentially `root_example`/`root_example_unavailable_reason` (new)
- Step 10 will remove old field to resolve conflict

**Build command:**
```bash
cargo build
```

**Expected result:** ⚠️ JSON output will temporarily have duplicate `root_example` key (old Value + new enum Available case) - this is expected, Step 10 fixes

---

### Step 9: Update JSON Serialization ✅ COMPLETED

**Objective:** Update serialization functions to extract and pass through `new_root_example` alongside old field.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**Changes:**

#### 9.1 Update `resolve_enum_data_mut` return type (lines 179-205):

**Current:**
```rust
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<Value>,            // root_example (OLD)
)
```

**Change to:**
```rust
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<Value>,            // root_example (OLD)
    Option<RootExample>,      // new_root_example (NEW)
)
```

**Update early return:**
```rust
return (None, None, None, None);
```

**Update map_or:**
```rust
self.enum_path_data
    .take()
    .map_or((None, None, None, None), |enum_data| {
        let instructions = Some(format!(
            "First, set the root mutation path to 'root_example', then you can mutate the '{}' path...",
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
            enum_data.root_example,      // OLD
            enum_data.new_root_example,  // NEW
        )
    })
```

#### 9.2 Update `into_mutation_path_external` (lines 76-110):

**Update extraction:**
```rust
let (enum_instructions, applicable_variants, root_example, new_root_example) =
    self.resolve_enum_data_mut();
```

**Update struct creation:**
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
        root_example,      // OLD
        new_root_example,  // NEW
    },
    path_example,
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, both fields serializing (JSON temporarily has duplicate keys)

---

### Step 10: Validation Checkpoint ⏳ PENDING

**Objective:** Test that new_root_example works correctly before removing old system.

**Validation steps:**

1. **Build and launch:**
   ```bash
   cargo build
   mcp__brp__brp_launch_bevy_example(target_name="extras_plugin", profile="debug")
   ```

2. **Get type guide:**
   ```bash
   mcp__brp__brp_type_guide(types=["extras_plugin::TestMixedMutabilityEnum"])
   ```

3. **Verify JSON output** contains:
   - OLD: `root_example` field with Value
   - NEW: `root_example` OR `root_example_unavailable_reason` (from flattened enum)
   - Check that unconstructible variants show `root_example_unavailable_reason`
   - Check that constructible variants show `root_example` (duplicated in JSON temporarily)

4. **Manual inspection:**
   - Verify new_root_example logic is correct
   - Verify reasons are meaningful
   - Verify hierarchical propagation works for nested enums

**Expected result:** Both systems work, new system produces correct unavailability reasons

**Decision point:** If validation passes, proceed to Step 11 (cutover). If issues found, debug before proceeding.

---

### Step 11: Cut Over - Remove Old System ⏳ PENDING

**Objective:** Remove old `root_example` and `root_example_unavailable_reason` fields, rename `new_root_example` → `root_example`.

**Why now safe:** Step 10 validated new system works correctly.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` (EnumPathData, PathInfo)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` (all initialization sites, loop)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` (initialization)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` (populate function)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` (serialization)

**Changes:**

#### 11.1 EnumPathData - Remove old fields, rename new field:
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<RootExample>,  // RENAMED from new_root_example
    // REMOVED: old root_example Option<Value>
    // REMOVED: root_example_unavailable_reason Option<String>
}
```

#### 11.2 PathInfo - Remove old field, rename new field:
```rust
pub struct PathInfo {
    // ... other fields ...
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub root_example: Option<RootExample>,  // RENAMED from new_root_example
    // REMOVED: old root_example Option<Value>
}
```

#### 11.3 Remove old HashMap from all functions:
- ProcessChildrenResult: Back to 3-tuple (remove 3rd element)
- build_partial_root_examples: Return single HashMap, remove old hash logic
- All propagation functions: Remove old hash parameter
- populate_root_examples_from_partials: Remove old hash parameter, keep only new logic

#### 11.4 Update initialization sites:
Remove old field inits, update new_root_example → root_example

#### 11.5 Update serialization:
Remove old field extraction, update new_root_example → root_example

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, old system removed, names cleaned up

---

### Step 12: Python Integration ⏳ PENDING

**Objective:** Add filtering logic to exclude paths with `root_example_unavailable_reason`.

**Files to modify:**
- `.claude/scripts/mutation_test/prepare.py`

**Changes:**

#### 12.1 Verify PathInfo TypedDict (already correct from original plan):
```python
class PathInfo(TypedDict, total=False):
    """Path metadata including mutability and root examples."""

    mutability: str
    root_example: object
    root_example_unavailable_reason: str  # Mutually exclusive with root_example
```

#### 12.2 Add filtering logic (after line 1022):
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

**Build command:**
```bash
~/.local/bin/basedpyright .claude/scripts/mutation_test/prepare.py
```

**Expected result:** Zero errors, zero warnings

---

### Step 13: Final Validation ⏳ PENDING

**Objective:** Run comprehensive testing to verify entire migration works correctly.

**Validation steps:**

1. **Build:**
   ```bash
   cargo build
   ```

2. **Launch test app:**
   ```bash
   mcp__brp__brp_launch_bevy_example(target_name="extras_plugin", profile="debug")
   ```

3. **Get type guide:**
   ```bash
   mcp__brp__brp_type_guide(types=["extras_plugin::TestMixedMutabilityEnum"])
   ```

4. **Verify JSON output** for `.value` path shows:
   - EITHER `root_example` field (for constructible variants)
   - OR `root_example_unavailable_reason` field (for unconstructible variants)
   - NOT both (mutually exclusive)
   - Reason mentions actual NotMutable field reasons

5. **Regenerate test data:**
   ```bash
   /create_mutation_test_json
   ```

6. **Run prepare.py:**
   ```bash
   python3 .claude/scripts/mutation_test/prepare.py
   ```
   Verify filtering output shows excluded paths with reasons.

7. **Run mutation tests:**
   ```bash
   .claude/commands/mutation_test.sh
   ```

8. **Regression testing:**
   - Test with `Option` (Mutable variants)
   - Test with `Result` (Mutable variants)
   - Test with `Handle` (may have PartiallyMutable)
   - Verify no regressions

**Expected result:** All tests pass, no regressions, clean JSON output with mutually exclusive fields

---

## IMPLEMENTATION COMPLETE

All 13 steps completed successfully! The implementation uses a parallel migration strategy:
1. Steps 1-2: Added enum and new field alongside old fields
2. Steps 3-9: Built new system in parallel with old system
3. Step 10: Validated new system works
4. Step 11: Cut over - removed old system, renamed new → final names
5. Steps 12-13: Python integration and final validation

The enum approach provides:
- Type-safe mutual exclusivity
- Clean JSON serialization (either `root_example` OR `root_example_unavailable_reason`)
- Variant-specific examples even for unconstructible types
- Actual field-level reasons from mutability analysis
- Filtered mutation tests that skip unconstructible paths
