# Fix Enum Child Overwrite Bug

## Problem Statement

When building `root_example` for deeply nested enum paths like `.image.0.uuid` in `Skybox`, the value is incorrectly set to `{"Weak": null}` instead of the proper nested structure `{"Weak": {"Uuid": {"uuid": "550e8400-..."}}}`.

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

**Hypothesis:** By logging the type, variant_chain, applicable_variants, and insert operations for each child, we can see exactly which child is overwriting which, and understand if the two `.image.0` children are from different variant contexts.

**Analysis:**
We need to understand:
1. Why there are two `.image.0` children in `child_paths`
2. What their `variant_chain` and `applicable_variants` values are
3. Which one has `partial_root_examples` and which doesn't
4. The exact sequence showing the overwrite

The current debug statements don't show the child's type or which variant context we're building for, making it hard to understand why the collision happens.

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
The trace log will show:
- Two separate `.image.0` children being processed
- Their different types (AssetId<Image> vs Arc<StrongHandle>)
- Their different variant_chain and applicable_variants values
- The exact overwrite with old value (correct nested structure) and new value (null)
- Clear evidence of which variant context each child belongs to

**Test approach:**
- Agent can test: After MCP reload, run `mcp__brp__brp_type_guide` with types `["bevy_core_pipeline::skybox::Skybox"]`
- User must verify: Check trace log for the new debug output showing child types, variant chains, and overwrites

**Result:** ⏸️ Awaiting testing
