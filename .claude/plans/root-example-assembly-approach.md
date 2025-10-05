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
