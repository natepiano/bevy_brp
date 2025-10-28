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

### Step 1: Define RootExample Enum ⏳ PENDING

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

### Step 2: Update EnumPathData Struct ⏳ PENDING

**Objective:** Replace two separate fields with single `root_example: Option<RootExample>` field.

**Why this changes things:** Removes `root_example_unavailable_reason` field, changes `root_example` type.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes:**

**Current (lines 213-226):**
```rust
#[derive(Debug, Clone)]
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<Value>,
}
```

**Change to:**
```rust
#[derive(Debug, Clone)]
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,
    pub root_example: Option<RootExample>,  // Changed from Option<Value>
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ❌ Compilation errors (all initialization sites need updating) - this is expected

---

### Step 3: Update EnumPathData Initialization Sites ⏳ PENDING

**Objective:** Fix compilation errors by updating all sites that construct `EnumPathData`.

**Why this fixes Step 2:** Provides initial values for the new field type.

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
        root_example:        None,  // Still None, but type changed
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
        root_example:        None,  // Still None, but type changed
    })
};
```

**Build command:**
```bash
cargo build
```

**Expected result:** Still errors in serialization and population code, but initialization sites fixed

---

### Step 4: Update PathInfo Struct ⏳ PENDING

**Objective:** Update PathInfo to use single flattened enum field instead of two separate fields.

**Why this is needed:** PathInfo is the external JSON representation that must serialize correctly.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Changes:**

**Current PathInfo (lines 172-197):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    // ... other fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example: Option<Value>,
}
```

**Change to:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    // ... other fields ...
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub root_example: Option<RootExample>,  // Changed type, added flatten
}
```

**Rationale:**
- `#[serde(flatten)]` makes the enum's fields appear at the same level as PathInfo's fields
- When `Available(value)`: serializes as `"root_example": <value>`
- When `Unavailable { reason }`: serializes as `"root_example_unavailable_reason": "<reason>"`
- `#[serde(untagged)]` on the enum prevents wrapper object

**Build command:**
```bash
cargo build
```

**Expected result:** PathInfo compiles, but serialization code still has errors

---

### Step 5: Add Variant Constructibility Analysis Function ⏳ PENDING

**Objective:** Create `analyze_variant_constructibility` function that determines if a variant can be constructed via BRP.

**Why this is safe:** New function, doesn't modify existing code.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Changes:**
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

**Expected result:** Clean compilation, function available but not yet called

---

### Step 6: Update build_partial_root_examples ⏳ PENDING

**Objective:** Change `build_partial_root_examples` to return `HashMap<Vec<VariantName>, RootExample>` and build enum variants directly.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Changes:**

##### Update signature (line 549):
```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    enum_examples: &[ExampleGroup],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, RootExample>  // Changed from Value to RootExample
```

##### Initialize HashMap (line 555):
```rust
let mut partial_root_examples = HashMap::new();
```

##### Replace variant processing loop (lines 558-606):

**DELETE lines 565-570** (the incorrect fallback).

**REPLACE lines 565-604** with:

```rust
// Find this variant's mutability status
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
    let nested_chain_reason = if let Some(ref reason) = unavailable_reason {
        // Parent is unconstructible → child inherits parent's reason
        Some(reason.clone())
    } else {
        // Parent is constructible → check if nested chain has its own unavailability
        child_mutation_paths
            .iter()
            .find_map(|child| {
                child.enum_path_data
                    .as_ref()
                    .filter(|data| data.variant_chain == *nested_chain)
                    .and_then(|data| {
                        data.root_example.as_ref().and_then(|re| match re {
                            RootExample::Unavailable { reason } => Some(reason.clone()),
                            RootExample::Available(_) => None,
                        })
                    })
            })
    };

    // Build RootExample enum variant
    let root_example = match nested_chain_reason {
        Some(reason) => RootExample::Unavailable { reason },
        None => RootExample::Available(example),
    };

    partial_root_examples.insert(nested_chain.clone(), root_example);
}

// Build root example for this variant's chain itself
let example = build_variant_example_for_chain(
    signature,
    variant_name,
    child_mutation_paths,
    &this_variant_chain,
    ctx,
);

// Build RootExample enum variant
let root_example = match unavailable_reason {
    Some(reason) => RootExample::Unavailable { reason },
    None => RootExample::Available(example),
};

partial_root_examples.insert(this_variant_chain, root_example);
```

##### Return (line 607):
```rust
partial_root_examples
```

**Build command:**
```bash
cargo build
```

**Expected result:** ❌ Compilation errors (return type mismatch in `ProcessChildrenResult`) - this is expected

---

### Step 7: Propagate Type Through Call Stack ⏳ PENDING

**Objective:** Update type alias and all function signatures to use `HashMap<Vec<VariantName>, RootExample>`.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs`

**Changes:**

#### 7.1 Update `ProcessChildrenResult` type (line 76-80):
```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, RootExample>,  // Changed from Value
);
```

#### 7.2 Update `process_signature_groups` (lines 400-460):
Return statement (line 459):
```rust
Ok((examples, child_mutation_paths, partial_root_examples))
```

#### 7.3 Update `process_enum` (lines 87-128):
Destructuring (line 101):
```rust
let (enum_examples, child_mutation_paths, partial_root_examples) =
    process_signature_groups(&variants_grouped_by_signature, ctx)?;
```

Call (line 121):
```rust
Ok(create_enum_mutation_paths(
    ctx,
    enum_examples,
    default_example,
    child_mutation_paths,
    partial_root_examples,
))
```

#### 7.4 Update `create_enum_mutation_paths` (lines 724-766):
Parameter (line 724):
```rust
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, RootExample>,  // Changed
) -> Vec<MutationPathInternal>
```

Call (line 756):
```rust
propagate_partial_root_examples_to_children(
    &mut child_mutation_paths,
    &partial_root_examples,
    ctx,
);
```

#### 7.5 Update `propagate_partial_root_examples_to_children` (lines 707-721):
Parameter:
```rust
fn propagate_partial_root_examples_to_children(
    child_paths: &mut [MutationPathInternal],
    partial_root_examples: &HashMap<Vec<VariantName>, RootExample>,  // Changed
    ctx: &RecursionContext,
)
```

Call (line 719):
```rust
support::populate_root_examples_from_partials(
    child_paths,
    partial_root_examples,
);
```

#### 7.6 Update `support::populate_root_examples_from_partials`
**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs:158-176`

```rust
pub fn populate_root_examples_from_partials(
    paths: &mut [MutationPathInternal],
    partials: &HashMap<Vec<VariantName>, RootExample>,  // Changed
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
        {
            // Single lookup, single assignment - enum already built
            if let Some(root_example) = partials.get(&enum_data.variant_chain) {
                enum_data.root_example = Some(root_example.clone());
            }
        }
    }
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, type propagation complete

---

### Step 8: Update JSON Serialization ⏳ PENDING

**Objective:** Update serialization functions to pass through the enum directly.

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**Changes:**

#### 8.1 Update `resolve_enum_data_mut` (lines 179-205):

**Current return type (line 181):**
```rust
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<Value>,            // root_example
)
```

**Change to:**
```rust
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<RootExample>,      // root_example (changed type)
)
```

**Update early return (line 186):**
```rust
return (None, None, None);
```

**Update map_or (lines 189-204):**
```rust
self.enum_path_data
    .take()
    .map_or((None, None, None), |enum_data| {
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
            enum_data.root_example,  // Just pass through the enum
        )
    })
```

#### 8.2 Update `into_mutation_path_external` (lines 76-110):

**Update extraction (line 94):**
```rust
let (enum_instructions, applicable_variants, root_example) =
    self.resolve_enum_data_mut();
```

**Update struct creation (lines 96-109):**
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
        root_example,  // Pass enum through, serde flatten handles serialization
    },
    path_example,
}
```

**Build command:**
```bash
cargo build
```

**Expected result:** ✅ Clean compilation, serialization complete

---

### Step 9: Python Integration ⏳ PENDING

**Objective:** Update Python TypedDict to handle both possible field names.

**Files to modify:**
- `.claude/scripts/mutation_test/prepare.py`

**Changes:**

#### 9.1 Update `PathInfo` TypedDict (lines 51-55):

```python
class PathInfo(TypedDict, total=False):
    """Path metadata including mutability and root examples."""

    mutability: str
    root_example: object
    root_example_unavailable_reason: str  # NEW - mutually exclusive with root_example
```

**Note:** Both fields are optional (total=False), but only one will be present at a time.

#### 9.2 Add path filtering logic (after line 1022):

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

### Step 10: Complete Validation ⏳ PENDING

**Objective:** Run comprehensive testing to verify all changes work correctly.

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
   - `root_example_unavailable_reason` field (NOT `root_example` field)
   - Reason mentions Arc fields
   - For Mutable variants: `root_example` field (NOT unavailable_reason)

5. **Regenerate test data:**
   ```bash
   /create_mutation_test_json
   ```

6. **Run prepare.py:**
   ```bash
   python3 .claude/scripts/mutation_test/prepare.py
   ```
   Verify filtering output shows excluded paths.

7. **Run mutation tests:**
   ```bash
   .claude/commands/mutation_test.sh
   ```

**Expected result:** All tests pass, no regressions

---

## IMPLEMENTATION DETAILS

### Key Design Decisions

#### Why Enum Instead of Two Fields?

The enum approach (`RootExample::Available` vs `RootExample::Unavailable`) enforces mutual exclusivity at the type level:
- **Type safety:** Can't accidentally set both fields
- **Clear semantics:** Either constructible (have example) OR unconstructible (have reason)
- **Single field:** Simpler to pass through call stack
- **JSON clarity:** Users see either `root_example` OR `root_example_unavailable_reason`, not both

#### How Serialization Works

```rust
#[derive(Serialize)]
#[serde(untagged)]
enum RootExample {
    Available(Value),
    Unavailable {
        #[serde(rename = "root_example_unavailable_reason")]
        reason: String
    },
}
```

With `#[serde(flatten)]` on PathInfo field:
- `Available(value)` → serializes as `"root_example": <value>`
- `Unavailable { reason }` → serializes as `"root_example_unavailable_reason": "<reason>"`
- No wrapper object, fields appear at PathInfo level

#### Hierarchical Reason Selection

For nested enum chains:
```rust
let nested_chain_reason = if let Some(ref reason) = unavailable_reason {
    // Parent is unconstructible → child inherits parent's reason
    Some(reason.clone())
} else {
    // Parent is constructible → check nested chain's own RootExample
    child_mutation_paths.iter().find_map(|child| {
        match &child.enum_path_data?.root_example? {
            RootExample::Unavailable { reason } => Some(reason.clone()),
            RootExample::Available(_) => None,
        }
    })
};
```

---

## SUPPORTING SECTIONS

### Problem Statement

Enum variants that are PartiallyMutable or NotMutable cannot be constructed via BRP, but their mutable fields should still be documented. Currently, these paths show `root_example: "None"` (fallback to wrong variant) causing:

1. **Misleading documentation** - shows wrong variant structure
2. **Mutation test failures** - tries to construct unconstructible variants
3. **User confusion** - instructions don't match reality

### Solution Overview

1. **Enum type** - Single `RootExample` enum with Available/Unavailable variants
2. **Single field** - `EnumPathData.root_example: Option<RootExample>`
3. **Mutual exclusivity** - Type system enforces either/or, not both
4. **Clean serialization** - Enum variants map to different JSON field names
5. **Filter tests** - Python code excludes unconstructible paths

### Expected Outcomes

#### Type Guide Output
- **Constructible variants:** Show `"root_example": {...}` with correct structure
- **Unconstructible variants:** Show `"root_example_unavailable_reason": "..."` with actual field-level reasons
- **Mutual exclusivity:** Never both fields in same path

#### Mutation Testing
- **Filtered automatically** - Unconstructible paths excluded during prepare.py
- **No failures** - Can't try to construct PartiallyMutable/NotMutable variants
- **Clear logs** - Shows what was excluded and why

### Success Criteria

- [ ] Type guide shows either `root_example` OR `root_example_unavailable_reason` per path
- [ ] Enum serialization produces correct JSON field names
- [ ] Mutation tests skip unconstructible paths
- [ ] No regression in existing enum handling
- [ ] All validation tests pass

### Files Modified Summary

#### Rust (4 files):
1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Add enum, update structs
2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` - Build enum variants, analysis function
3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` - Update serialization
4. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` - Simplified propagation

#### Python (1 file):
5. `.claude/scripts/mutation_test/prepare.py` - Filter unconstructible paths

### Dependencies

**✅ COMPLETED:** This plan depends on `mutability-reason-type-safety.md` (completed), which provides typed `NotMutableReason` enum for extracting field-level reasons in the analysis function.

**This plan is ready for implementation.**
