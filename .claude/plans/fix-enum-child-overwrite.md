# Fix Enum Child Overwrite Bug

## Problem Statement

When building `root_example` for deeply nested enum paths like `.image.0.uuid` in `Skybox`, the value is incorrectly set to `{"Weak": null}` instead of the proper nested structure `{"Weak": {"Uuid": {"uuid": "550e8400-..."}}}`.

### Concrete Example from Type Guide

Type: `bevy_core_pipeline::skybox::Skybox`

**The bug - `.image.0.uuid` path (actual output):**
```json
".image.0.uuid": {
  "description": "Mutate the uuid field of AssetId",
  "example": "550e8400-e29b-41d4-a716-446655440000",
  "path_info": {
    "applicable_variants": ["AssetId<Image>::Uuid"],
    "mutation_status": "mutable",
    "path_kind": "StructField",
    "root_example": {"Weak": null},  // ❌ WRONG - should be nested structure
    "type": "Uuid",
    "type_kind": "Value"
  }
}
```

**What it should be:**
```json
".image.0.uuid": {
  "description": "Mutate the uuid field of AssetId",
  "example": "550e8400-e29b-41d4-a716-446655440000",
  "path_info": {
    "applicable_variants": ["AssetId<Image>::Uuid"],
    "mutation_status": "mutable",
    "path_kind": "StructField",
    "root_example": {
      "Weak": {
        "Uuid": {
          "uuid": "550e8400-e29b-41d4-a716-446655440000"
        }
      }
    },  // ✅ CORRECT - full nested structure for Handle<Image>::Weak → AssetId<Image>::Uuid
    "type": "Uuid",
    "type_kind": "Value"
  }
}
```

Note: The same bug affects `.image.0.index`, `.image.0.index.generation`, and `.image.0.index.index` - they all show `"root_example": {"Weak": null}` instead of the proper nested structure.

## Root Cause Hypothesis

The `build_partial_roots` function processes all child paths from all variant groups. For `Handle<Image>`:
- Weak variant creates `.image.0` → `AssetId<Image>` enum (has partial_root_examples)
- Strong variant creates `.image.0` → `Arc<StrongHandle>` (no partial_root_examples)

Both children have the same descriptor "0" (tuple index), so when building the HashMap at line 792, the last insert wins, potentially overwriting good values with `null`.

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

## Experiment History

### Attempt 1: Add comprehensive debug logging (2025-10-06)

**Hypothesis:** By logging each child's type, variant_chain, applicable_variants, and the insert operations, we can understand why the HashMap collision happens and identify the most elegant approach to prevent incorrect overwrites.

**Analysis:**
The fundamental principle: **`variant_chain` uniquely identifies an enum path through recursion.**

This means when building for a specific `child_chain` (a variant chain), we should only include children whose `variant_chain` is compatible with that `child_chain`.

The question is:
1. **Why are we processing incompatible children?** The loop at line 746 processes ALL `child_paths` regardless of their `variant_chain`
2. **What is the compatibility rule?** When building for `child_chain`, which children belong and which don't?
3. **What is the elegant fix?** Filter children by their `variant_chain` before inserting into the HashMap

The debug traces will show us each child's `variant_chain` alongside the `child_chain` we're building for, making the incompatibility obvious and revealing the correct filtering logic.

**Change Location:** mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:740-793

**What we're changing:**
Adding debug statements to show:
- Which variant chain we're currently building for
- Each child's full type name
- Each child's variant_chain from enum_path_data
- Each child's applicable_variants
- Detection and logging of HashMap overwrites with old/new values

**Code:**
```rust
// For each chain, build wrapped example with ALL children
let mut found_child_chains = false;
for child_chain in child_chains_to_wrap {
    let mut children = HashMap::new();

    // NEW: Debug which variant chain we're building for
    tracing::debug!(
        "[ENUM] =========================================="
    );
    tracing::debug!(
        "[ENUM] Building partial root for our_variant: {:?}, child_chain: {:?}",
        our_variant.as_str(),
        child_chain
            .iter()
            .map(super::types::VariantName::as_str)
            .collect::<Vec<_>>()
    );

    // Collect ALL children with variant-specific or regular values
    for child in child_paths {
        // Skip grandchildren - only process direct children
        if !child.is_direct_child_at_depth(*depth) {
            continue;
        }

        let descriptor = child.path_kind.to_mutation_path_descriptor();

        // NEW: Debug full child context
        tracing::debug!(
            "[ENUM] ------------------------------------------"
        );
        tracing::debug!(
            "[ENUM] Processing child: {} (descriptor: {:?})",
            child.full_mutation_path,
            descriptor
        );
        tracing::debug!(
            "[ENUM]   Child type: {}",
            child.type_name
        );

        if let Some(child_enum_data) = &child.enum_path_data {
            let child_variant_chain = child_enum_data
                .variant_chain
                .iter()
                .map(|vp| vp.variant.as_str())
                .collect::<Vec<_>>();
            tracing::debug!(
                "[ENUM]   Child variant_chain: {:?}",
                child_variant_chain
            );
            tracing::debug!(
                "[ENUM]   Child applicable_variants: {:?}",
                child_enum_data
                    .applicable_variants
                    .iter()
                    .map(|v| v.as_str())
                    .collect::<Vec<_>>()
            );
        } else {
            tracing::debug!("[ENUM]   Child has NO enum_path_data");
        }

        // Debug: Check child's partial_root_examples
        if let Some(child_partials) = &child.partial_root_examples {
            tracing::debug!(
                "[ENUM]   Child has {} partial roots",
                child_partials.len()
            );
            if let Some(found_value) = child_partials.get(&child_chain) {
                tracing::debug!("[ENUM]   -> FOUND variant-specific value: {:?}", found_value);
            } else {
                tracing::debug!(
                    "[ENUM]   -> NOT FOUND, keys: {:?}",
                    child_partials
                        .keys()
                        .map(|k| k
                            .iter()
                            .map(super::types::VariantName::as_str)
                            .collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                );
            }
        } else {
            tracing::debug!(
                "[ENUM]   NO partial_root_examples, regular example: {:?}",
                child.example
            );
        }

        let value = child
            .partial_root_examples
            .as_ref()
            .and_then(|partials| partials.get(&child_chain))
            .cloned()
            .unwrap_or_else(|| child.example.clone());

        // NEW: Debug the insert operation to see overwrites
        if let Some(existing) = children.get(&descriptor) {
            tracing::debug!(
                "[ENUM]   ⚠️ OVERWRITE: descriptor {:?} already exists",
                descriptor
            );
            tracing::debug!(
                "[ENUM]   ⚠️ Old value: {:?}",
                existing
            );
            tracing::debug!(
                "[ENUM]   ⚠️ New value: {:?}",
                value
            );
        } else {
            tracing::debug!(
                "[ENUM]   INSERT: {:?} <- {:?}",
                descriptor,
                value
            );
        }

        children.insert(descriptor, value);
    }

    tracing::debug!(
        "[ENUM] =========================================="
    );

    // Use existing build_variant_example with SHORT variant name
    let wrapped =
        build_variant_example(signature, variant.name(), &children, ctx.type_name());

    partial_roots.insert(child_chain, wrapped);
    found_child_chains = true;
}
```

**Expected outcome:**
The trace log will show the bug in action for the `.image.0.uuid` path:

**Context:** `.image.0.uuid` has variant_chain `["Handle<Image>::Weak", "AssetId<Image>::Uuid"]` and needs `root_example: {"Weak": {"Uuid": {"uuid": "..."}}}`

**What we'll see when building partial roots:**
- At some depth (likely when building for the `.image` level to wrap into Handle<Image>)
- Building for `child_chain` like `["Handle<Image>::Weak", "AssetId<Image>::Uuid"]` or `["Handle<Image>::Weak"]`
- Processing two `.image.0` children (both have descriptor "0"):
  - One from Weak variant path: has proper nested AssetId structure in `partial_root_examples`
  - One from Strong variant path: has null because Arc<StrongHandle> has no enum nesting
- The Strong variant's `.image.0` overwrites the Weak variant's `.image.0`
- Result: The assembled structure gets null instead of the nested Uuid structure
- This propagates up, giving `.image.0.uuid` the wrong `root_example: {"Weak": null}`

**What this tells us:**
When building for a specific `child_chain`, we're incorrectly including children from incompatible variant paths. The fix: **filter children to only include those whose `variant_chain` is compatible with the `child_chain` we're building for.**

**Test approach:**
- Agent can test: After MCP reload, run `mcp__brp__brp_type_guide` with types `["bevy_core_pipeline::skybox::Skybox"]`
- User must verify: Check trace log for the new debug output showing child types, variant chains, and overwrites

**Result:** ⏸️ Awaiting testing

### Attempt 2: Filter children by variant_chain compatibility (2025-10-06)

**Hypothesis:** The overwrite bug occurs because `build_partial_roots` processes ALL children regardless of whether their `variant_chain` is compatible with the `child_chain` being built. By filtering children to only include those whose `variant_chain` is a prefix of the `child_chain`, we prevent incompatible children from overwriting correct values.

**Analysis:**
The fundamental principle: **`variant_chain` uniquely identifies an enum path through recursion.**

From debug traces, we confirmed:
- When building for `child_chain = ["WrapperEnum::WithSimpleEnum", "SimpleNestedEnum::None"]`
- The loop processes TWO `.0` children:
  - Child with `variant_chain = ["WrapperEnum::WithOptionalEnum"]` (incompatible) → inserts Null
  - Child with `variant_chain = ["WrapperEnum::WithSimpleEnum"]` (compatible) → overwrites with correct value

**Compatibility rule:**
- `["WrapperEnum::WithSimpleEnum"]` IS a prefix of `["WrapperEnum::WithSimpleEnum", "SimpleNestedEnum::None"]` ✅
- `["WrapperEnum::WithOptionalEnum"]` is NOT a prefix of `["WrapperEnum::WithSimpleEnum", "SimpleNestedEnum::None"]` ❌

When building for a specific `child_chain`, only include children whose `variant_chain` is a prefix of that `child_chain`.

**Change Location:** mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:759-765

**What we're changing:**
Add variant_chain compatibility check immediately after skipping grandchildren, filtering out children whose variant_chain is incompatible with the child_chain being built.

**Code:**
```rust
// Collect ALL children with variant-specific or regular values
for child in child_paths {
    // Skip grandchildren - only process direct children
    if !child.is_direct_child_at_depth(*depth) {
        continue;
    }

    // NEW: Filter by variant_chain compatibility
    // Only include children whose variant_chain is a prefix of child_chain
    if let Some(child_enum_data) = &child.enum_path_data {
        let child_variant_chain = extract_variant_names(&child_enum_data.variant_chain);

        // Child's variant_chain must be a prefix of the child_chain we're building for
        if child_variant_chain.len() > child_chain.len() {
            tracing::debug!(
                "[ENUM]   Skipping child {} - variant_chain too long ({} > {})",
                child.full_mutation_path,
                child_variant_chain.len(),
                child_chain.len()
            );
            continue;
        }

        // Check if child's variant_chain matches the first N elements of child_chain
        let is_compatible = child_variant_chain.iter()
            .zip(child_chain.iter())
            .all(|(child_v, chain_v)| child_v == chain_v);

        if !is_compatible {
            tracing::debug!(
                "[ENUM]   Skipping child {} - incompatible variant_chain: {:?} vs {:?}",
                child.full_mutation_path,
                child_variant_chain.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
                child_chain.iter().map(|v| v.as_str()).collect::<Vec<_>>()
            );
            continue;
        }
    }

    let descriptor = child.path_kind.to_mutation_path_descriptor();

    // [Rest of existing processing code continues unchanged...]
```

**Expected outcome:**
- No overwrites in trace log (zero instances of "⚠️ OVERWRITE")
- Skybox `.image.0.uuid` has correct `root_example: {"Weak": {"Uuid": {"uuid": "550e8400-..."}}}`
- WrapperEnum `.0` has correct nested `root_example` (not null)
- All deeply nested enum paths have properly assembled root_examples

**Test approach:**
- Agent can test: After MCP reload, run type guides for `bevy_core_pipeline::skybox::Skybox` and `extras_plugin::WrapperEnum`
- Agent can verify: Check trace log shows "Skipping child" messages and zero overwrites
- User must verify: Confirm root_examples are correct in both type guides

**Result:** ✅ **SUCCESS**

**What worked:**
- Zero overwrites in trace log (no "⚠️ OVERWRITE" messages)
- 12 children skipped due to incompatible variant_chains
- Skybox `.image.0.uuid` now has correct `root_example: {"Weak": {"Uuid": {"uuid": "550e8400-e29b-41d4-a716-446655440000"}}}`
- Skybox `.image.0.index` now has correct `root_example: {"Weak": {"Index": {"index": {"generation": 1000000, "index": 1000000}}}}`
- WrapperEnum nested paths like `.0.0.x` have correct root_examples like `{"WithSimpleEnum": {"WithVec2": [1.0, 2.0]}}`
- TestVariantChainEnum still has all correct root_examples (no regression)

**Verification:**
Tested with:
- `bevy_core_pipeline::skybox::Skybox` - All deeply nested enum paths now correct
- `extras_plugin::WrapperEnum` - All nested paths correct
- `extras_plugin::TestVariantChainEnum` - Existing correct behavior preserved

**Trace log evidence:**
- "Skipping child" messages show filtering is working
- Zero "⚠️ OVERWRITE" messages confirm no more collisions
- Incompatible variant_chains are properly filtered out

**Conclusion:**
The variant_chain compatibility filter successfully prevents incompatible children from overwriting correct values. The fix is minimal, focused, and preserves all existing correct behavior while eliminating the bug.

### Cleanup: Debug logging removed and code documented (2025-10-06)

**Actions taken:**
1. Removed all debug logging from Attempt 1 (lines 746-884)
2. Added comprehensive documentation explaining the variant_chain compatibility rule
3. Cleaned up code while preserving the filtering logic from Attempt 2

**Documentation added:**
- Detailed explanation of variant_chain compatibility principle
- Concrete example with `Handle<Image>` showing Weak vs Strong variant paths
- Explanation of HashMap collision problem and how filtering prevents it
- Comments explaining the prefix matching logic

**Final code location:** `enum_path_builder.rs:740-808`

**Status:** ✅ **COMPLETE** - Bug fixed, code cleaned, and documented
