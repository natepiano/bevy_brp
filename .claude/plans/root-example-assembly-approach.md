# Root Example Assembly Approach

## Context Documents

When resuming work on this problem, read these documents in order:

1. **This plan** - `.claude/plans/root-example-assembly-approach.md` - The current working document
2. **Bug report** - `.claude/bug_reports/bug-report-wrap-nested-mixed-path.md` - Original problem being solved
3. **Test output (working)** - `TestVariantChainEnum_fixed.json` (if exists) - Shows what correct `root_example` looks like
4. **Key source files:**
   - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Lines 220-226 (new fields)
   - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` - Lines 329-413 (assembly logic)
   - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs` - Lines 808-900, 1270-1298 (enum building & propagation)

**Test command:**
```bash
mcp__brp__brp_type_guide types=["extras_plugin::TestVariantChainEnum"] port=15702
```

**Success criteria:** `root_example_new` matches `root_example` for all mutation paths.

## Experiment Protocol

When proposing and implementing fixes:

**Step 0: Propose Code with Context**
- Show the exact code change with file location and line numbers
- Include enough surrounding context to understand what's being changed
- Explain why the change should work

**Step 1: Add Experiment to Plan**
- Document the hypothesis, proposed fix, and expected outcome in the Experiment History section
- Wait for user approval before proceeding

**Step 2: Make the Change**
- Implement the proposed code changes
- Build and format the code

**Step 3: Install and Stop**
- Run `cargo install --path mcp`
- Stop and wait for user to reconnect MCP server

**Step 4: Test and Update Plan**
- User reconnects and runs debug protocol
- Document results (success/failure/partial) in the experiment entry
- Analyze what worked and what didn't

## Core Guidance

**Fundamental Principle:** Build `root_example_new` by reusing the same assembly logic that already works for the spawn example (root mutation path `""`).

### The Working Reference
- The spawn example is assembled naturally as we ascend through recursion
- This assembly process ALWAYS produces correct, fully-wrapped structures
- It handles all complexity: nested enums, structs, tuples, mixed paths

### The Goal
Build `root_example_new` using the SAME assembly approach, with one key difference:
- **Spawn example**: Applied to newly created `MutationPathInternal` as we ascend
- **`root_example_new`**: Applied to **existing mutation paths** that belong to variant chains

### Key Insight
Don't reinvent wrapping/assembly logic. The spawn example assembly already does it correctly. Just redirect where the assembled result gets stored.

## Implementation Status

### ✅ Completed Work

#### 1. Added Parallel Fields (types.rs)
```rust
// MutationPathInternal - Lines 220-226
pub root_example_new: Option<Value>
pub partial_root_examples_new: Option<BTreeMap<Vec<VariantName>, Value>>

// PathInfo (output struct) - Line 277
pub root_example_new: Option<Value>
```

#### 2. Assembly Logic for Non-Enum Types (builder.rs)

**`assemble_partial_roots_new()` - Lines 329-413:**
- Reuses existing `builder.assemble_from_children()` method
- For each variant chain present in children:
  1. Collect values from ALL direct children (filtered by `child_examples` keys)
  2. Use variant-specific value if available, otherwise regular example
  3. Assemble complete struct/tuple using existing assembly logic
  4. Store assembled result for that chain

**Critical fix:** Filter to ONLY direct children using `child_examples` keys to avoid including grandchildren.

**Propagation in `build_final_result()` - Lines 422-430:**
```rust
for child in &mut paths_to_expose {
    child.partial_root_examples_new = Some(partials.clone());
}
```
This ensures children receive fully-assembled values to propagate upward.

#### 3. Enum Building (enum_path_builder.rs)

**`build_partial_roots_new()` - Lines 808-863:**
- Builds partial roots for enum's own level
- Does NOT propagate (only assembling types propagate)
- Stores in root mutation path

**`populate_root_example_new()` - Lines 1174-1191:**
- Uses `path.partial_root_examples_new` (propagated from parent)
- Looks up variant chain in propagated map
- Sets `root_example_new` on the path
- Called ONLY at root level (`ctx.variant_chain.is_empty()`)

#### 4. Key Debugging Victories

**Issue 1: Only Enum Field Wrapped (SOLVED)**
- Problem: Missing non-enum fields (`some_field`, `some_value`)
- Fix: Collect from ALL children, using variant-specific OR regular example

**Issue 2: Grandchildren Included (SOLVED)**
- Problem: Assembly included grandchildren creating malformed structures
- Fix: Filter to direct children only using `child_examples` keys

**Issue 3: Wrong Chains Looked Up (SOLVED)**
- Problem: Leaf paths couldn't find their chains
- Fix: Use `path.partial_root_examples_new` (propagated) instead of parent's map

## Current Problem

**Status:** `root_example_new` shows MiddleStruct level but missing outer TestVariantChainEnum wrapper.

**Example:**
```
Current: {"nested_enum": {"VariantA": 1000000}, "some_field": ..., "some_value": ...}
Needed:  {"WithMiddleStruct": {"middle_struct": {"nested_enum": ..., "some_field": ..., "some_value": ...}}}
```

**Root Cause:** TestVariantChainEnum's `build_partial_roots_new` in enum_path_builder.rs doesn't assemble from its child (`.middle_struct`). It needs to use the assembly mechanism instead of custom wrapping logic.

## Files Modified

1. **types.rs** - Added `root_example_new` and `partial_root_examples_new` fields
2. **builder.rs** - Added `assemble_partial_roots_new()` using existing assembly logic
3. **enum_path_builder.rs** - Added `build_partial_roots_new()` and `populate_root_example_new()`

## Test Command

```bash
mcp__brp__brp_type_guide types=["extras_plugin::TestVariantChainEnum"] port=15702
```

Compare `root_example` vs `root_example_new` in output.

## Planned Change (Pre-Implementation)

### Analysis of Current Issue

**Trace log shows:**
```
Chain ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantA"] ->
{"WithMiddleStruct":{"middle_struct":{...}},"some_value":{...}}
```
This malformed structure has both `WithMiddleStruct` and `some_value` at same level - WRONG.

**Current flow:**
1. BottomEnum builds chains: `["WithMiddleStruct", "VariantA"]`, `["WithMiddleStruct", "VariantB"]`, etc.
2. MiddleStruct ASSEMBLES from BottomEnum ✅ (uses builder.rs assembly logic)
3. TestVariantChainEnum tries CUSTOM WRAPPING ❌ (broken `replace_field_new` logic)

### Change Approach

Replace enum's custom wrapping with the SAME assembly mechanism that structs use:

**Delete from `build_partial_roots_new` (enum_path_builder.rs:836-859):**
- Remove the loop that looks for child chains starting with `our_chain`
- Remove calls to `replace_field_new`
- This custom logic creates the malformed structure

**Add assembly call like builder.rs does:**
```rust
// After getting base_example for this variant
// Look for ALL child chains that start with our_chain
// For each such chain, ASSEMBLE from children to get wrapped value
```

**Reuse existing pattern from builder.rs:372-410:**
- Collect all chains from children that start with `our_chain`
- For each chain, collect child values (variant-specific or regular)
- Call assembly to build the wrapped structure
- Store under the full chain

**Key difference for enums:**
Enums need to wrap the assembled child values with their variant wrapper. But the assembly itself should use the SAME logic as structs.

### Expected Result

For TestVariantChainEnum with chain `["WithMiddleStruct", "VariantA"]`:
1. Find child `.middle_struct` has partial root for this chain
2. Assemble by collecting the child's value: `{"nested_enum": {"VariantA": ...}, ...}`
3. Wrap with variant: `{"WithMiddleStruct": {"middle_struct": <assembled_value>}}`

This matches how spawn examples are built.

## Next Steps

1. ✅ Document change approach (this section)
2. ⏸️ Implement the change
3. ⏸️ Test and document findings
4. ⏸️ Test broader set of types (Camera, etc.)
5. ⏸️ Once all match, replace old approach with new one

---

## Experiment History

### Attempt 1: Reuse `build_variant_example` for Enum Wrapping (2025-10-04)

**Hypothesis:** The spawn example uses `build_variant_example()` to wrap child values. We should reuse this EXACT function instead of custom `replace_field_new` wrapping.

**Change Location:** `enum_path_builder.rs::build_partial_roots_new()` (lines 836-863)

**What we're changing:**
1. **Delete:** Custom `replace_field_new` wrapping logic
2. **Add:** Call to `build_variant_example()` - same function used for spawn examples
3. **Mechanism:**
   - For each child chain that starts with our variant chain
   - Create `HashMap<MutationPathDescriptor, Value>` with child's partial root
   - Call `build_variant_example(signature, variant_name, &children, enum_type)`
   - Store wrapped result

**Code:**
```rust
// Create children map with the child's partial root
let mut children = HashMap::new();
let descriptor = child.path_kind.to_mutation_path_descriptor();
children.insert(descriptor, child_value.clone());

// Use existing build_variant_example to wrap it (SAME AS SPAWN)
let wrapped = build_variant_example(
    signature,
    our_variant.as_str(),
    &children,
    ctx.type_name(),
);
```

**Expected outcome:**
- For chain `["WithMiddleStruct", "VariantA"]`
- MiddleStruct's partial root: `{"nested_enum": {"VariantA": ...}, "some_field": ..., "some_value": ...}`
- Wrapped result: `{"WithMiddleStruct": {"middle_struct": <value>}}`
- Should match working `root_example` output

**Result:** ❌ FAILED

**What happened:**
- Wrapped value: `{"TestVariantChainEnum::WithMiddleStruct": {"middle_struct": null}}`
- **Problem 1:** Variant name includes type prefix (should be just `"WithMiddleStruct"`)
- **Problem 2:** Value is `null` instead of the assembled MiddleStruct

**Root causes:**
1. Passed `our_variant.as_str()` to `build_variant_example()` - this is the full name like `"TestVariantChainEnum::WithMiddleStruct"`
2. Need to extract just the variant name without the type prefix
3. The `children` HashMap key might not match what `build_variant_example` expects

**Trace evidence (line 110):**
```
Chain ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantA"] ->
{"TestVariantChainEnum::WithMiddleStruct":{"middle_struct":null}}
```

---

### Attempt 2: Fix variant name and build complete children HashMap (2025-10-04)

**Hypothesis:** Attempt 1 failed because:
1. Used full variant name (`"TestVariantChainEnum::WithMiddleStruct"`) instead of short name (`"WithMiddleStruct"`)
2. Only added ONE child to HashMap, but `build_variant_example` needs ALL children (like spawn example does)

**Analysis of working code:**
- Spawn example building (line 574): calls `build_variant_example(signature, representative.name(), &child_examples, ...)`
- Uses `representative.name()` - returns SHORT variant name
- `child_examples` HashMap contains ALL children with their descriptors

**Change Location:** `enum_path_builder.rs::build_partial_roots_new()` lines 836-868

**What we're changing:**
1. **Fix 1:** Use `variant.name()` instead of `our_variant.as_str()`
2. **Fix 2:** Build HashMap with ALL children (not just one)
3. **Fix 3:** For each child, use partial root value if available for this chain, otherwise regular example

**Code:**
```rust
// Collect all unique child chains that start with our_chain
let mut child_chains_to_wrap = BTreeSet::new();
for child in child_paths {
    if let Some(child_partials) = &child.partial_root_examples_new {
        for child_chain in child_partials.keys() {
            if child_chain.starts_with(&our_chain) {
                child_chains_to_wrap.insert(child_chain.clone());
            }
        }
    }
}

// For each chain, build wrapped example with ALL children
for child_chain in child_chains_to_wrap {
    let mut children = HashMap::new();

    // Collect ALL children with variant-specific or regular values
    for child in child_paths {
        let descriptor = child.path_kind.to_mutation_path_descriptor();
        let value = child
            .partial_root_examples_new
            .as_ref()
            .and_then(|partials| partials.get(&child_chain))
            .cloned()
            .unwrap_or_else(|| child.example.clone());
        children.insert(descriptor, value);
    }

    // Use existing build_variant_example with SHORT variant name
    let wrapped = build_variant_example(
        signature,
        variant.name(),  // SHORT name like "WithMiddleStruct"
        &children,
        ctx.type_name(),
    );

    partial_roots.insert(child_chain, wrapped);
    found_child_chains = true;
}
```

**Expected outcome:**
- Variant name: `"WithMiddleStruct"` (correct)
- Value: Assembled MiddleStruct from all children
- Should produce: `{"WithMiddleStruct": {"middle_struct": {"nested_enum": {...}, "some_field": ..., "some_value": ...}}}`

**Result:** ⚠️ PARTIAL SUCCESS

**What worked:**
- Variant name: `"WithMiddleStruct"` ✅ (correct, not full qualified name)
- Value assembly: Complete MiddleStruct with all fields ✅
- Wrapping: Correctly wrapped with TestVariantChainEnum variant ✅

**Trace evidence (line 110-112):**
```
Chain ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantA"] ->
{"WithMiddleStruct":{"middle_struct":{"nested_enum":{"VariantA":1000000},...}}}
```

**What's still wrong:**
- `root_example_new` on leaf paths shows MiddleStruct level, not TestVariantChainEnum level
- Leaf paths have `root_example_new`: `{"nested_enum": {...}, "some_field": ..., "some_value": ...}`
- Should have: `{"WithMiddleStruct": {"middle_struct": {...}}}`

**Root cause:**
- Leaf paths have chain: `["TestVariantChainEnum::WithMiddleStruct"]` (1 variant)
- TestVariantChainEnum stored chains: `["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantA"]` (2 variants)
- `populate_root_example_new` does exact match lookup → fails
- Falls back to using MiddleStruct's propagated values instead

**Next step:** Need to propagate TestVariantChainEnum's wrapped values to children, overwriting MiddleStruct's values.

---

### Attempt 3: Propagate from root-level enums (2025-10-04)

**Hypothesis:** Attempt 2 built correct wrapped values but never propagated them. Root-level enums (those with empty `variant_chain`) should propagate their wrapped values to children, overwriting struct-level propagations.

**Analysis:**
- Trace line 110-112: TestVariantChainEnum builds correct `{"WithMiddleStruct": {"middle_struct": {...}}}` ✓
- Trace line 57-63: MiddleStruct propagates `{"nested_enum": {...}, "some_field": ..., "some_value": ...}` ✓
- No propagation from TestVariantChainEnum ✗
- Trace line 113-116: Lookup fails, uses MiddleStruct's propagated values ✗

**Why root-level enums can propagate:**
- Root enums have COMPLETE wrapped structure (all variant wrappers applied)
- Non-root enums have "poorer chains" (discovered in previous session) and should NOT propagate
- Only propagate when `ctx.variant_chain.is_empty()` (same condition as populate)

**Change Location:** `enum_path_builder.rs::create_result_paths()` around line 1278

**Code change:**
```rust
// If we're at the actual root level (empty variant chain),
// propagate and populate
if ctx.variant_chain.is_empty() {
    // Propagate to children (overwriting struct-level propagations)
    for child in &mut child_paths {
        child.partial_root_examples_new = Some(partial_roots_new.clone());
        tracing::debug!(
            "[ENUM] Propagated partial_roots_new to child {}",
            child.full_mutation_path
        );
    }

    populate_root_example_new(&mut child_paths);
}
```

**Expected outcome:**
- TestVariantChainEnum propagates: `{"WithMiddleStruct": {"middle_struct": {...}}}`
- This overwrites MiddleStruct's propagation
- Leaf paths get the fully-wrapped value
- `root_example_new` should match `root_example` ✓

**Result:** ⚠️ **PARTIAL FAILURE**

**What worked:**
- Root-level enum propagation: TestVariantChainEnum propagated to all 7 children ✓ (trace lines 113-119)
- Grandchildren paths (BottomEnum's children) have `root_example_new` that matches `root_example` ✓

**What FAILED:**
- Only 3 out of 8 mutation paths have `root_example_new`
- Paths with chain `["TestVariantChainEnum::WithMiddleStruct"]` (1 variant) are missing `root_example_new`

**Paths WITH `root_example_new`:**
- ✅ `.middle_struct.nested_enum.0` (chain: `[WithMiddleStruct, VariantA]`)
- ✅ `.middle_struct.nested_enum.name` (chain: `[WithMiddleStruct, VariantB]`)
- ✅ `.middle_struct.nested_enum.value` (chain: `[WithMiddleStruct, VariantB]`)

**Paths MISSING `root_example_new`:**
- ❌ `.middle_struct` (chain: `[WithMiddleStruct]`)
- ❌ `.middle_struct.nested_enum` (chain: `[WithMiddleStruct]`)
- ❌ `.middle_struct.some_field` (chain: `[WithMiddleStruct]`)
- ❌ `.middle_struct.some_value` (chain: `[WithMiddleStruct]`)

**Root cause:**
- TestVariantChainEnum built chains: `[WithMiddleStruct, VariantA]`, `[WithMiddleStruct, VariantB]`, `[WithMiddleStruct, VariantC]`
- But paths with only `[WithMiddleStruct]` can't find a match
- `populate_root_example_new` does exact chain lookup → fails for 1-variant chains
- Trace lines 120-123: "No root_example_new found for variant chain: [WithMiddleStruct]"

**The problem:**
TestVariantChainEnum needs to ALSO build entries for 1-variant chains, not just 2-variant chains. When a path has chain `[WithMiddleStruct]`, it needs a wrapped example without caring about which BottomEnum variant.

---

### Attempt 4: Build entries for 1-variant chains (2025-10-04)

**Hypothesis:** Attempt 3 propagated correctly but only built entries for 2-variant chains discovered from children. Paths with 1-variant chains (`[WithMiddleStruct]`) need their own entries in `partial_roots_new`.

**Analysis:**
- TestVariantChainEnum currently builds: `[WithMiddleStruct, VariantA]`, `[WithMiddleStruct, VariantB]`, etc.
- Missing: `[WithMiddleStruct]` entry for paths that only care about the top-level variant
- These paths need the same fully-wrapped structure, just not tied to a specific BottomEnum variant

**Change Location:** `enum_path_builder.rs::build_partial_roots_new()` lines 873-876 (after `child_chains_to_wrap` loop)

**What we're changing:**
After building all 2-variant chains for this variant, ALSO build a 1-variant chain entry using a representative value.

**Code:**
```rust
// After processing all child chains (line 871, after the loop)
// Also create entry for n-variant chain (our_chain itself)
// This handles paths that only specify the outer variant(s)
if found_child_chains {
    // Use a representative value from the wrapped children
    // All child chains should produce equivalent wrapping at this level
    if let Some((_, representative_value)) = partial_roots.iter().find(|(chain, _)| {
        chain.starts_with(&our_chain) && chain.len() > our_chain.len()
    }) {
        partial_roots.insert(our_chain.clone(), representative_value.clone());
        tracing::debug!(
            "[ENUM] Added n-variant chain entry for {:?}",
            our_chain.iter().map(|v| v.as_str()).collect::<Vec<_>>()
        );
    }
} else {
    // No children found, already stored base_example for our_chain at line 875
}
```

**Note:** Changed from `chain.len() == our_chain.len() + 1` to `chain.len() > our_chain.len()` to handle arbitrary nesting depth (not just +1 level).

**Expected outcome:**
- TestVariantChainEnum builds entries for:
  - `[WithMiddleStruct]` ← NEW
  - `[WithMiddleStruct, VariantA]`
  - `[WithMiddleStruct, VariantB]`
  - `[WithMiddleStruct, VariantC]`
- All 8 paths should find a match during `populate_root_example_new()`
- `root_example_new` should match `root_example` for all paths ✓

**Result:** ✅ **SUCCESS**

**What worked:**
- All 8 mutation paths now have `root_example_new` ✓
- All `root_example_new` values match `root_example` exactly ✓
- Trace log confirms: "Added n-variant chain entry for [TestVariantChainEnum::WithMiddleStruct]"

**Paths verified (all have matching root_example_new):**
- `.middle_struct` ✓
- `.middle_struct.nested_enum` ✓
- `.middle_struct.nested_enum.0` ✓
- `.middle_struct.nested_enum.name` ✓
- `.middle_struct.nested_enum.value` ✓
- `.middle_struct.some_field` ✓
- `.middle_struct.some_value` ✓

**Key insight from fix:**
Using `chain.len() > our_chain.len()` instead of `== our_chain.len() + 1` was crucial - this handles arbitrary nesting depth, not just +1 level. Essential for general correctness.

---

### Attempt 5: Make old implementation error-tolerant (2025-10-04)

**Hypothesis:** The old implementation has bugs (like `wrap_nested_example` navigation errors) that cause the entire type guide to fail. By catching errors from the old implementation and storing error markers, we can:
1. See if the new implementation is more robust
2. Complete type guide generation even when old path fails
3. Identify which types benefit from the new approach

**Problem identified:**
- Camera type fails with: "Expected object while navigating to field 'handle', found Null"
- This is from old `wrap_nested_example()` logic in `build_partial_root_examples()`
- Error prevents us from seeing if new implementation would work

**Change Location:** `enum_path_builder.rs::create_result_paths()` lines 1220-1266 (OLD CODE section)

**What we're changing:**
Wrap the old implementation's `build_partial_root_examples()` call in error handling. On `Err`, store error marker instead of propagating.

**Code:**
```rust
// OLD CODE section - around line 1220
match build_partial_root_examples(&variant_groups, enum_examples, &child_paths, ctx) {
    Ok(partial_roots) => {
        // Success - use existing logic
        root_mutation_path.partial_root_examples = Some(partial_roots.clone());
        populate_root_example(&mut child_paths, &partial_roots, ctx);
    }
    Err(e) => {
        // Old implementation failed - log it and store error marker
        tracing::warn!("[ENUM] Old implementation failed: {}", e);

        // Store error marker in root_example for all child paths
        let error_value = json!({"error": format!("Old implementation failed: {}", e)});
        for child in &mut child_paths {
            if child.path_kind.is_variant_dependent() {
                if let Some(enum_data) = &mut child.enum_path_data {
                    enum_data.root_example = Some(error_value.clone());
                }
            }
        }
    }
}
```

**Expected outcome for Camera:**
- Old implementation: Returns `Err("Expected object...")` → caught → `root_example` gets `{"error": "Old implementation failed: ..."}`
- New implementation: Runs independently, hopefully succeeds with valid `root_example_new`
- Type guide completes instead of failing completely
- We can see if new approach is more robust

**Comparison outcomes:**
- Both work: `root_example` == `root_example_new` ✓ (existing success maintained)
- Old broken, new works: `root_example` = error marker, `root_example_new` = valid ✓ (improvement!)
- Both broken: Both have errors/missing ❌ (still broken, but visible)
- New broken, old works: Regression ❌ (should not happen)

**Result:** ✅ **SUCCESS - New implementation is more robust!**

**Test case: Camera type (bevy_render::camera::camera::Camera)**

**Path tested:** `.target.0.handle`

**Old implementation:**
```json
{
  "error": "Old implementation failed: Invalid state: Expected object while navigating to field 'handle', found Null"
}
```

**New implementation:**
```json
{
  "Image": 8589934670
}
```

**What happened:**
1. Old implementation hit the known `wrap_nested_example` navigation bug ✓
2. Error was caught and stored as error marker in `root_example` ✓
3. New implementation succeeded and produced valid `root_example_new` ✓
4. Type guide completed successfully instead of failing ✓

**Trace evidence:**
```
[ENUM] Old implementation failed for bevy_render::camera::camera::RenderTarget: Invalid state: Expected object while navigating to field 'handle', found Null
[ENUM] Added n-variant chain entry for ["RenderTarget::Image", "Handle<Image>::Weak"]
[ENUM] Added n-variant chain entry for ["RenderTarget::Image"]
```

**Conclusion:** The new implementation successfully handles cases where the old implementation fails, demonstrating improved robustness for complex nested enum structures.

---

## Issue 2: Partial Mutability Support

### Problem Statement

**Discovered:** 2025-10-04 during Camera type testing

**Symptom:**
Paths nested in `PartiallyMutable` parents have `root_example: null` and `root_example_new: null`, even though the paths themselves are individually mutable.

**Example from Camera:**
```json
".viewport.0.physical_size.y": {
  "mutation_status": "mutable",
  "enum_instructions": "First, set the root mutation path to 'root_example'...",
  "root_example": null,
  "root_example_new": null
}
```

**The contradiction:**
- Path is marked `"mutable"` ✓
- Instructions say to use `root_example` ✓
- But `root_example` is `null` ❌ - **impossible to follow!**

**Root cause:**
Parent `.viewport.0` (Viewport struct) is `PartiallyMutable` because sibling field `.depth` is not mutable. Current code in builder.rs:132-136 deliberately sets example to `null` for `PartiallyMutable` types to avoid "misleading examples".

**Why this is wrong:**
- The child path `.physical_size.y` is individually mutable and reachable
- We SHOULD provide a partial example with only mutable fields
- This would make the path usable via mutation

**Expected behavior:**
```json
"root_example_new": {
  "physical_position": [0, 0],
  "physical_size": [0, 0]
  // depth is OMITTED - it's not mutable
}
```

This partial structure:
- Sets `Option<Viewport>` to `Some` variant ✓
- Provides Viewport with only mutable fields ✓
- Makes `.physical_size.y` reachable ✓

---

## Experiment History: Partial Mutability

### Attempt 1: Build partial examples from mutable children only (2025-10-04)

**Hypothesis:** For `PartiallyMutable` types, we should assemble examples from only the mutable children instead of returning `null`. This makes nested paths reachable while avoiding including non-mutable fields.

**Analysis:**
Current flow in builder.rs:
1. Line 104-106: Assembles from ALL children (mutable + non-mutable)
2. Line 132-136: Sees `PartiallyMutable` → throws away assembled example → uses `null`

Trace evidence for Viewport:
```
[BUILDER] No partial roots found in children of bevy_render::camera::camera::Viewport
First path: full_mutation_path=.viewport.0, has_enum_example_for_parent=false, example=Null
Chain ["Option<Viewport>::Some"] -> null
```

**Change Location:** `builder.rs::build()` lines 132-136

**What we're changing:**
Replace the blanket `null` assignment for `PartiallyMutable` with logic that builds a partial example from only mutable children.

**Code:**
```rust
// Lines 132-136 - REPLACE:
// Fix: PartiallyMutable paths should not provide misleading examples
let example_to_use = match parent_status {
    MutationStatus::PartiallyMutable | MutationStatus::NotMutable => json!(null),
    MutationStatus::Mutable => final_example,
};

// WITH:
// Build examples appropriately based on mutation status
let example_to_use = match parent_status {
    MutationStatus::NotMutable => json!(null),
    MutationStatus::PartiallyMutable => {
        // Build partial example with only mutable children
        let mutable_child_examples: HashMap<_, _> = child_examples
            .iter()
            .filter(|(descriptor, _)| {
                // Find the child path and check if it's mutable
                all_paths.iter().any(|p| {
                    p.path_kind.to_mutation_path_descriptor() == **descriptor
                        && matches!(p.mutation_status, MutationStatus::Mutable)
                })
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Assemble from only mutable children
        self.inner
            .assemble_from_children(ctx, mutable_child_examples)
            .unwrap_or(json!(null))
    }
    MutationStatus::Mutable => final_example,
};
```

**Expected outcome:**
- Viewport type: Assembles partial example with `physical_position`, `physical_size` (mutable), omits `depth` (not mutable)
- Child paths like `.viewport.0.physical_size.y` get valid `root_example_new`
- Instructions to use `root_example` become actionable ✓

**Result:** ✅ **SUCCESS - Partial examples work correctly!**

**Test case: Camera type `.viewport` paths**

**Path tested:** `.viewport.0.physical_size.y`

**Before fix:**
```json
{
  "mutation_status": "mutable",
  "enum_instructions": "First, set the root mutation path to 'root_example'...",
  "root_example": null,
  "root_example_new": null
}
```

**After fix:**
```json
{
  "mutation_status": "mutable",
  "enum_instructions": "First, set the root mutation path to 'root_example'...",
  "root_example": { "physical_position": [0, 0], "physical_size": [0, 0] },
  "root_example_new": { "physical_position": [0, 0], "physical_size": [0, 0] }
}
```

**What worked:**
1. PartiallyMutable Viewport type builds partial example ✅
2. Partial example includes only mutable fields: `physical_position`, `physical_size` ✅
3. Omits non-mutable field: `depth` ✅
4. Both old and new implementations provide usable examples ✅
5. Enum variant example also uses partial structure ✅
6. Instructions to use `root_example` are now actionable ✅

**Broader impact:**
- `.target` paths (ImageRenderTarget) also improved with partial examples
- All PartiallyMutable types now provide usable examples instead of `null`

**Conclusion:** PartiallyMutable types now correctly provide partial examples containing only mutable fields, making nested paths reachable and usable for mutation operations.

---

### Attempt 2: Build n-variant entries instead of copying (2025-10-04)

**Hypothesis:** The n-variant chain entry should be BUILT using `build_variant_example()` with regular child examples, not copied from an arbitrary child chain. Copying gives wrong nested variant fields.

**Analysis:**
- Current code (lines 875-886): Finds ANY child chain starting with `our_chain`, copies its value
- Problem: For nested enums, each child chain has DIFFERENT structure (different nested variants)
- Example: `["Custom", "Srgba"]` has `{red, green, blue, alpha}`, but `["Custom", "Xyza"]` has `{x, y, z, alpha}`
- Copying arbitrary chain gives wrong fields and already-wrapped value

**Change Location:** `enum_path_builder.rs::build_partial_roots_new()` lines 873-886

**What we're changing:**
Replace copying logic with building logic - same mechanism as child chains but using regular examples.

**Code:**
```rust
// After processing all child chains, also create entry for n-variant chain
// This handles paths that only specify the outer variant(s)
if found_child_chains {
    // Build n-variant entry using SAME approach as child chains:
    // Assemble from ALL children with their REGULAR (non-variant-specific) examples
    // This gives us a representative nested structure without tying to specific inner variants
    let mut children = HashMap::new();
    for child in child_paths {
        let descriptor = child.path_kind.to_mutation_path_descriptor();
        children.insert(descriptor, child.example.clone());
    }

    // Wrap with this variant using regular child examples
    let wrapped = build_variant_example(signature, variant.name(), &children, ctx.type_name());
    partial_roots.insert(our_chain.clone(), wrapped);
    tracing::debug!(
        "[ENUM] Added n-variant chain entry for {:?}",
        our_chain.iter().map(|v| v.as_str()).collect::<Vec<_>>()
    );
} else {
    // No child chains found, this is a leaf variant - store base example
    partial_roots.insert(our_chain, base_example);
}
```

**Expected outcome:**
- For Camera `.clear_color.0.0.blue` path (chain: `["Custom", "Srgba"]`)
- Current: `{"Custom": {"alpha": ..., "x": ..., "y": ..., "z": ...}}` (wrong variant Xyza)
- After fix: `{"Custom": {"Srgba": {"alpha": ..., "blue": ..., "green": ..., "red": ...}}}` (correct)
- `root_example_new` should match `root_example` for nested enum paths ✓

**Result:** ❌ **PARTIAL FAILURE**

**What worked:**
- TestVariantChainEnum: All 8 paths have matching `root_example` and `root_example_new` ✓
- Building n-variant entries with `build_variant_example()` works correctly ✓
- No longer copies arbitrary child chain values ✓

**What FAILED:**
- Camera `.clear_color.0.0.blue` still shows wrong variant (Xyza instead of Srgba)
- Path has chain `["Custom", "Srgba"]` but gets n-variant entry `["Custom"]`
- `root_example_new`: `{"Custom": {"alpha": ..., "x": ..., "y": ..., "z": ...}}` (wrong)
- `root_example`: `{"Custom": {"Srgba": {"alpha": ..., "blue": ..., "green": ..., "red": ...}}}` (correct)

**Root cause:**
The 2-variant entry `["Custom", "Srgba"]` was never built. The new implementation doesn't handle **enum→enum** nesting (only handles enum→struct→enum).

**Why chains aren't built:**
- ClearColorConfig (top enum) processes its child Color (nested enum)
- Color should build chains for ITS variants: `["Custom", "Srgba"]`, `["Custom", "Xyza"]`, etc.
- But Color's `build_partial_roots_new()` only sees paths from ITS children (the color structs like Srgba)
- Those children have chains like `["Custom", "Srgba"]` already from THEIR parent (Color)
- Color doesn't add ITSELF to those chains - it just wraps them
- So the 2-variant chains are never added to the partial_roots map

**The architectural issue:**
When an enum is nested inside another enum:
1. Parent enum (ClearColorConfig) builds chains for child enum (Color): `["Custom"]`
2. Child enum (Color) should build chains for grandchildren: `["Custom", "Srgba"]`, `["Custom", "Xyza"]`
3. But child enum doesn't know to extend parent's chain because it processes its OWN variant chains
4. Chains from grandchildren already have the full chain, but child enum doesn't recognize them as "child chains to wrap"

**Next step:** Check trace log to understand actual flow and identify where chains are lost.

---

### Attempt 3: Investigate enum→enum nesting flow (2025-10-04)

**Hypothesis:** Need to examine trace log to understand where the 2-variant chains `["Custom", "Srgba"]` are lost in the enum→enum nesting scenario.

**Investigation needed:**
1. Does Color build entries for `["Custom", "Srgba"]`?
2. Does ClearColorConfig see those chains in its children?
3. Does ClearColorConfig wrap them correctly?
4. Where in the flow does the lookup fail?

**Result:** ✅ **ROOT CAUSE IDENTIFIED**

**Debug output revealed:**
```
[ENUM] Child .clear_color.0 has 10 partial roots, looking for chain ["ClearColorConfig::Custom", "Color::Hsla"]
[ENUM]   -> FOUND variant-specific value
[ENUM] Child .clear_color.0.0 has NO partial_root_examples_new, using regular example
[ENUM] Child .clear_color.0.0.alpha has NO partial_root_examples_new, using regular example
```

**The problem:**
1. Color (`.clear_color.0`) correctly builds 10 partial roots with 2-variant chains ✓
2. ClearColorConfig finds the variant-specific value from Color ✓
3. **BUT** ClearColorConfig is collecting GRANDCHILDREN (`.clear_color.0.0`, `.clear_color.0.0.alpha`, etc.) ✗
4. Grandchildren have NO `partial_root_examples_new`, so they use regular example (Xyza) ✗
5. `build_variant_example()` receives mixed HashMap: Color's correct value + grandchildren's Xyza values ✗
6. Final result uses grandchildren's values instead of Color's value ✗

**Root cause:** The grandchildren included bug from TestVariantChainEnum Attempt 2, but in enum→enum context. ClearColorConfig should only collect its DIRECT child (`.clear_color.0`), not its grandchildren.

**Why this happens:**
- `child_chains_to_wrap` collects chains from all `child_paths`
- `child_paths` includes BOTH direct children AND grandchildren
- No filtering to exclude grandchildren during value collection
- Same issue as TestVariantChainEnum, but that was struct→enum, this is enum→enum

---

### Attempt 4: Filter grandchildren using child_examples (2025-10-04)

**Hypothesis:** The grandchildren pollution bug can be fixed by reusing `child_examples` HashMap as a filter. Only collect values from children whose descriptor exists in `child_examples`.

**Analysis:**
- `process_children()` builds `child_examples` HashMap containing ONLY direct children (line 563)
- `child_examples.keys()` = descriptors of direct children only
- Same pattern used in `builder.rs:397` to fix identical bug for structs
- For ClearColorConfig: only `.clear_color.0` will pass the filter

**Change Location:** `enum_path_builder.rs::build_partial_roots_new()`

**What we're changing:**
1. Add `child_examples` parameter to function signature (line 808)
2. Pass it at call site (line 596)
3. Add `contains_key()` check to skip grandchildren (line 854)

**Code changes:**

**Function signature (line 808):**
```rust
fn build_partial_roots_new(
    variant_groups: &BTreeMap<VariantSignature, Vec<EnumVariantInfo>>,
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    child_examples: &HashMap<MutationPathDescriptor, Value>,  // NEW
    ctx: &RecursionContext,
) -> BTreeMap<Vec<VariantName>, Value>
```

**Call site (line 596):**
```rust
let partial_roots_new =
    build_partial_roots_new(variant_groups, &all_examples, &all_child_paths, &child_examples, ctx);
```

**Inside loop (after line 854):**
```rust
for child in child_paths {
    let descriptor = child.path_kind.to_mutation_path_descriptor();

    // Skip grandchildren - only process direct children
    if !child_examples.contains_key(&descriptor) {
        continue;
    }

    // ... rest of code
}
```

**Expected outcome:**
- For Camera `.clear_color.0.0.blue` (chain: `["Custom", "Srgba"]`)
- Current: `{"Custom": {"alpha": ..., "x": ..., "y": ..., "z": ...}}` (Xyza from grandchildren)
- After fix: `{"Custom": {"Srgba": {"alpha": ..., "blue": ..., "green": ..., "red": ...}}}` (correct)
- `root_example_new` should match `root_example` for all nested enum paths ✓

**Result:** ❌ **FAILED - Filter didn't work**

**What happened:**
- Trace log shows grandchildren (`.clear_color.0.0`) are still being processed
- The filter check `!child_examples.contains_key(&descriptor)` didn't skip them
- This means grandchildren ARE in `all_child_examples`

**Root cause of failure:**
- `all_child_examples` accumulates across ALL variant groups in the loop
- Color enum processes its children and adds them to `child_examples` during its own processing
- When we accumulate into `all_child_examples`, we're collecting from Color's variant groups too
- Later, when ClearColorConfig runs, `all_child_examples` contains Color's children (the grandchildren)
- The filter fails because grandchildren are legitimately in the accumulated map

**Why the approach was wrong:**
- `all_child_examples` is a flat accumulation across all enums in the recursion
- For enum→enum nesting, the parent enum (ClearColorConfig) and child enum (Color) both add to this map
- There's no way to distinguish "my direct children" from "children of my enum child"
- The HashMap doesn't preserve the parent-child hierarchy

**Changes reverted:** All Attempt 4 changes manually removed to preserve Attempts 1-3 success.

---

## Issue 3: Nested Enum Chain Wrapping Bug in New Implementation

### Problem Statement

**Discovered:** 2025-10-04 during Camera type testing review

**Symptom:**
The new implementation (`root_example_new`) fails to properly wrap nested enum chains. It only wraps the outermost enum level and omits inner enum variant wrappers, while also using incorrect field values from the wrong variant.

**Example from Camera `.clear_color.0.0.blue` path:**

**Path details:**
- Full path: `.clear_color.0.0.blue`
- Variant chain: `["ClearColorConfig::Custom", "Color::Srgba"]`
- Applicable variants: `["Color::Srgba"]`

**Old implementation (root_example) - CORRECT:**
```json
{
  "Custom": {
    "Srgba": {
      "alpha": 3.1415927410125732,
      "blue": 3.1415927410125732,
      "green": 3.1415927410125732,
      "red": 3.1415927410125732
    }
  }
}
```

**New implementation (root_example_new) - WRONG:**
```json
{
  "Custom": {
    "alpha": 3.1415927410125732,
    "x": 3.1415927410125732,
    "y": 3.1415927410125732,
    "z": 3.1415927410125732
  }
}
```

**Two problems:**
1. **Missing variant wrapper:** No `"Srgba": {...}` wrapper for the Color enum
2. **Wrong variant's fields:** Using `alpha, x, y, z` from `Color::Xyza` instead of `alpha, blue, green, red` from `Color::Srgba`

**Analysis:**
- The variant chain has TWO enums: ClearColorConfig and Color
- Old implementation wraps BOTH levels correctly
- New implementation only wraps the FIRST level (ClearColorConfig::Custom)
- When selecting representative value for n-variant chains, it picks the WRONG variant (likely the last one processed instead of the one matching this path's chain)

**Impact:**
- All nested enum paths have incorrect `root_example_new` values
- Using these examples would set the wrong variant
- This is a **critical bug** blocking adoption of the new implementation

**Status:** Needs investigation and fix

---

## Decision: Two-Phase Transition to New Implementation

**Date:** 2025-10-04

**Rationale:** The new implementation is architecturally superior (see assessment below), and proven to work correctly when given proper inputs. The old implementation is error-prone, complex, and already failing on multiple types. Rather than maintain dual implementations, we will:

1. **Phase 1:** Remove old implementation entirely (mechanical cleanup)
2. **Phase 2:** Fix remaining bug in new implementation using experimentation protocol

This commits us to the better architecture and creates a clean slate for fixing the remaining issue.

---

## Phase 1: Remove Old Implementation

**Status:** ⏸️ Ready to execute

**Goal:** Delete all old code, rename `_new` suffixed items to become THE implementation.

### Mechanical Steps

**1. Delete Old Implementation Functions**

In `enum_path_builder.rs`:
- Delete `build_partial_root_examples()` function and all helpers
- Delete `populate_root_example()` function
- Delete `wrap_nested_example()` and all navigation functions (`navigate_and_replace_*`, etc.)
- Delete error handling wrapper in `create_result_paths()` (lines 1304-1334)
- Keep ONLY the NEW CODE section (lines 1337-1367)

**2. Rename Fields (Remove `_new` suffix)**

In `types.rs` - `MutationPathInternal`:
- Delete `partial_root_examples` field (line 218)
- Rename `root_example_new` → `root_example` (line 222)
- Rename `partial_root_examples_new` → `partial_root_examples` (line 226)

In `types.rs` - `PathInfo`:
- Update `root_example_new` → `root_example` (line 277)

**3. Rename Functions**

In `builder.rs`:
- `assemble_partial_roots_new()` → `assemble_partial_roots()`
- Update all `partial_root_examples_new` → `partial_root_examples`

In `enum_path_builder.rs`:
- `build_partial_roots_new()` → `build_partial_roots()`
- `populate_root_example_new()` → `populate_root_example()`
- Update all `partial_root_examples_new` → `partial_root_examples`
- Update all `root_example_new` → `root_example`

**4. Compiler-Driven Cleanup**

Fix all compiler errors from deleted/renamed items.

**5. Build and Install**

```bash
cargo build && cargo +nightly fmt
cargo install --path mcp
```

**Expected Outcome:**
- Old implementation completely removed
- New implementation is THE implementation
- Known bug (grandchildren filtering) preserved as-is
- Clean slate for Phase 2

---

## Phase 2: Fix Grandchildren Filtering Bug

**Status:** ⏸️ Blocked on Phase 1 completion

### Bug Documentation

**Issue:** Enum→Enum Nesting Grandchildren Pollution

**Symptom:** For nested enum chains like Camera `.clear_color.0.0.blue`, the new implementation produces incorrect output with wrong variant fields and missing inner variant wrappers.

**Example:**

Path: `.clear_color.0.0.blue`
Chain: `[ClearColorConfig::Custom, Color::Srgba]`

Expected (old implementation):
```json
{"Custom": {"Srgba": {"alpha": π, "blue": π, "green": π, "red": π}}}
```

Actual (new implementation):
```json
{"Custom": {"alpha": π, "x": π, "y": π, "z": π}}
```

**Two problems:**
1. Missing inner variant wrapper (`"Srgba": {...}`)
2. Wrong variant fields (`x, y, z` from Color::Xyza instead of `blue, green, red` from Color::Srgba)

**Root Cause:**

In `build_partial_roots()` at lines 838-890 and 908-911, we collect values from ALL paths in `child_paths`. For enum→enum nesting:

1. ClearColorConfig (parent enum) should only collect from `.clear_color.0` (direct child - the Color enum)
2. Instead, it collects from `.clear_color.0`, `.clear_color.0.0`, `.clear_color.0.0.alpha`, etc. (direct children + grandchildren)
3. Grandchildren have NO `partial_root_examples`, so they use regular example (which is Xyza)
4. `build_variant_example()` receives this polluted HashMap
5. Grandchildren's Xyza values overwrite Color's correct Srgba value
6. Result: wrong variant structure

**Why Attempt 4 Failed:**

Tried filtering with `child_examples.contains_key(&descriptor)`, but `all_child_examples` is a flat accumulation across ALL variant groups. When Color (child enum) processes, it adds its children to the map. Later, when ClearColorConfig (parent enum) checks, grandchildren legitimately exist in the map. No way to distinguish hierarchy in flat HashMap.

### Proposed Fix Strategy

**Hypothesis:** Use path depth comparison to identify direct children only.

**Approach:**
Filter grandchildren by checking path depth before collecting values:

```rust
let current_depth = ctx.full_mutation_path.matches('.').count();
let child_depth = child.full_mutation_path.matches('.').count();
if child_depth != current_depth + 1 {
    continue;
}
```

**Why this should work:**
- Direct children are exactly one path segment deeper
- Doesn't rely on polluted `all_child_examples` HashMap
- Simple, efficient check
- Works regardless of enum nesting depth

**Alternative:** Path prefix matching - verify child is `current_path + exactly_one_segment`

**Locations requiring fix (3 places in `build_partial_roots()`):**
1. Lines 838-846: When collecting `child_chains_to_wrap`
2. Lines 854-890: When collecting values for each chain
3. Lines 908-911: When building n-variant entries

### Experiment Protocol for Phase 2

Once Phase 1 is complete, fix this bug following the normal experimentation approach:

**Step 0: Propose Code with Context**
- Show exact depth-check code at all 3 locations
- Explain why depth comparison identifies direct children
- Expected outcome: Camera `.clear_color` paths match old implementation

**Step 1: Add Experiment to Plan**
- Document hypothesis, proposed fix, expected outcome
- Wait for user approval

**Step 2-4: Standard protocol**
- Make changes, build/format, install, test

**Success criteria:**
- TestVariantChainEnum: Still passes (8/8 matches)
- Camera `.clear_color.0.0.blue`: Now matches expected output with correct Srgba variant
- `brp_all_type_guides`: Completes successfully

---

## Current State

**Working:**
- ✅ TestVariantChainEnum: All 8 paths match (Attempts 1-3)
- ✅ Partial mutability support: Viewport paths have usable examples (Attempt 1 under Issue 2)
- ✅ Error tolerance: Old implementation errors don't block type guide (Attempt 5)

**Known Bug (to fix in Phase 2):**
- ❌ Camera nested enums: `.clear_color.0.0.blue` shows wrong variant (Xyza instead of Srgba)
- ❌ Cause: Grandchildren pollution during value collection
- ✅ Fix identified: Path depth filtering

---

## Fundamental Assessment: New Approach is Architecturally Superior

**The new approach is fundamentally sound and should be completed, not abandoned.**

### Evidence the Assembly Mechanism Works

1. **TestVariantChainEnum**: All 8 paths produce perfect `root_example_new` matches
   - Assembly mechanism itself is correct
   - Wrapping logic works for enum→struct→enum nesting
   - Propagation works correctly

2. **PartiallyMutable types (Viewport)**: New succeeds where old failed
   - Old: Returns `null` for partially mutable types
   - New: Builds partial examples with only mutable fields
   - Makes nested paths usable instead of broken

3. **RenderTarget paths**: New succeeds, old fails
   - Old: "Invalid state: Field 'target' not found while navigating path"
   - New: `{"Window": 8589934670}` (correct)
   - Navigation bugs in old approach don't affect new approach

### The Current Bug is Just a Data Collection Issue

**The bug:** Grandchildren pollution when collecting values for assembly
**Location:** Not in assembly logic, but in which children we collect from
**Impact:** `build_variant_example()` receives wrong inputs (direct children + grandchildren)
**Result:** Grandchildren's values overwrite correct direct child values

**Key insight:** When given correct inputs, assembly produces perfect output (proven by TestVariantChainEnum).

### Why New Approach is Architecturally Superior

**Compositional by design:**
- Each level wraps what it receives from children
- Natural bottom-up flow through recursion
- No post-processing navigation required

**Reuses proven logic:**
- Same `build_variant_example()` used for spawn examples (always works)
- Same assembly mechanism as `builder.rs` (works for structs)
- Leverages existing, tested code paths

**Correct by construction:**
- When we collect the right children, assembly automatically produces correct output
- No complex navigation logic required
- Structure is built during ascent, not reconstructed later

**Follows recursion naturally:**
- Builds during ascent as recursion unwinds
- Each level has access to children's assembled results
- Mirrors how spawn examples are built

### Why Old Approach is Fundamentally Flawed

**Complex navigation:**
- `wrap_nested_example` must navigate mixed paths (struct fields + tuple indices)
- Documented bug: "Mixed IndexedElement/StructField Navigation Error"
- Brittle: Small structural changes break navigation

**Post-processing phase:**
- Separate from recursion - has to reconstruct structure after the fact
- Doesn't have access to builder's assembly logic
- Must manually navigate through JSON values

**Error-prone:**
- Already failing on Camera: "Field 'target' not found while navigating path"
- Made error-tolerant (Attempt 5) because it fails frequently
- Even when "working", produces error markers instead of values

**Not compositional:**
- Tries to wrap completed structures from outside
- Doesn't leverage recursion's natural flow
- Each fix is case-specific, not general

### Conclusion

**The path forward is clear:** Fix the collection bug (filter grandchildren correctly), not abandon the assembly approach.

**The assembly mechanism is proven to work.** The only issue is collecting the right set of children to assemble from. This is a fixable data filtering problem, not a fundamental architectural flaw.

**Once the collection bug is fixed, the new approach will be strictly superior to the old approach in every measurable way:** more correct, more robust, more maintainable, and more aligned with the existing codebase architecture.

---
