# Plan: Build Root Examples Bottom-Up During Enum Recursion

## Problem

Currently, mutation paths for deeply nested enum fields require multi-step mutations. For example, to mutate `.middle_struct.nested_enum.name`, the agent must:

1. First mutate root to `{"WithMiddleStruct": {"middle_struct": {"nested_enum": {"VariantA": ...}}}}`
2. Then mutate `.middle_struct.nested_enum` to `{"VariantB": {...}}`

**The Issue:** Step 1 uses the wrong variant (VariantA) because we only build one example per enum level during recursion. The field `.name` only exists in VariantB, not VariantA.

## Summary of Changes

This plan fixes the multi-step mutation requirement for deeply nested enum fields by building complete root examples during recursion. The implementation adds:

**New Fields:**
- `MutationPathInternal.partial_root_examples`: Stores partial roots at each enum level
- `EnumData.applicable_variants`: Tracks which variants make a path valid
- `EnumData.variant_chain_root_example`: Complete root example for the path
- `PathInfo.applicable_variants`: Exposed to user
- `PathInfo.root_variant_example`: Exposed to user

**New Functions:**
- `build_partial_root_examples()`: Builds partial roots for all variant chains
- `build_partial_root_for_chain()`: Builds partial root for specific chain
- `wrap_nested_example()`: Wraps child partial roots into parent examples
- `populate_variant_chain_root_examples()`: Copies roots to paths at root level
- `extract_variant_names()`: Helper to extract variant names

**Modified Functions:**
- `create_result_paths()`: Calls new building functions
- `process_children()`: Populates `applicable_variants`
- `generate_enum_instructions()`: Provides single-step guidance
- `MutationPath::from_mutation_path_internal()`: Exposes new fields

**Key Algorithm:** Bottom-up building where each enum wraps its children's already-built partial roots (one level of wrapping per enum). By the time we reach root, all work is done - just copy results to paths.

## Goal

Enable **single-step mutations** by providing the correct complete root structure:

```json
{
  "path": ".middle_struct.nested_enum.name",
  "example": "Hello, World!",
  "path_info": {
    "applicable_variants": ["BottomEnum::VariantB"],
    "root_variant_example": {
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
    },
    "enum_instructions": "Set root to 'root_variant_example', then mutate this path"
  }
}
```

## Solution: Bottom-Up Building

**Key Insight:** Build partial root examples at EACH enum level during recursion UP. Each enum wraps its children's already-built partial roots. By the time we reach the root, all work is done - just copy the results to paths.

**Path-Specific Root Example Sizes:**
- Shallow paths (`.middle_struct`) → Small root examples (1 enum level)
- Deep paths (`.middle_struct.nested_enum.name`) → Large root examples (2+ enum levels)

### Data Flow (Bottom-Up)

```
[Depth 3] String ".name" field
  → Returns with variant_chain=[WithMiddleStruct, VariantB]
  ↑ No partial roots to build (not an enum)

[Depth 2] BottomEnum
  → Sees child variant_chains: [WithMiddleStruct, VariantB], [WithMiddleStruct, VariantA], etc.
  → Children are primitives (no partial roots to wrap)
  → Builds partial roots:
      [WithMiddleStruct, VariantB] → {"VariantB": {"name": "...", "value": ...}}
      [WithMiddleStruct, VariantA] → {"VariantA": 123}
  → Stores these in a HashMap on its root path
  ↑ Returns to parent

[Depth 1] MiddleStruct (struct, not enum)
  → Just passes paths through unchanged
  ↑ Returns to parent

[Depth 0] TestVariantChainEnum (ROOT)
  → Sees child variant_chains: [WithMiddleStruct, VariantB], [WithMiddleStruct], etc.
  → Finds BottomEnum root path with partial roots already built
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
  → Stores complete roots
  → Populates variant_chain_root_example on all matching paths
```

## Implementation

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

**1a. Add `partial_root_examples` field to `MutationPathInternal`:**

```rust
pub struct MutationPathInternal {
    // ... existing fields ...

    /// For enum root paths only: Maps FULL variant chains to partial root examples
    /// built from this enum level down through all descendants.
    ///
    /// Example at BottomEnum with ctx.variant_chain=[WithMiddleStruct]:
    ///   [WithMiddleStruct, VariantB] => {"VariantB": {"name": "...", "value": ...}}
    ///   [WithMiddleStruct, VariantA] => {"VariantA": 123}
    ///
    /// Example at TestVariantChainEnum (root):
    ///   [WithMiddleStruct, VariantB] => {"WithMiddleStruct": {"middle_struct": {"nested_enum": {"VariantB": {...}}}}}
    ///   [WithMiddleStruct] => {"WithMiddleStruct": {"middle_struct": {...}}}
    ///
    /// None for non-enum paths.
    pub partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
}
```

**Why:** Each enum needs to store partial root examples indexed by the FULL variant chain (no prefix stripping - keeps code simple and readable). Parent enums look up child's partial roots using the same full chain keys.

**1b. Add `applicable_variants` and `variant_chain_root_example` fields to `EnumData`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumData {
    /// The chain of variant selections from root to this point
    pub variant_chain: Vec<VariantPath>,

    /// NEW: Set of variant full names where this path is valid
    /// Example: {"BottomEnum::VariantB", "BottomEnum::VariantA"}
    /// Populated during path processing in Phase 5
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    pub applicable_variants: HashSet<String>,

    /// NEW: Complete root example for single-step mutation
    /// Only populated at root level (when ctx.variant_chain is empty)
    /// Copied from partial_root_examples in Phase 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_chain_root_example: Option<Value>,
}
```

**Note:** Ensure `EnumData` initialization in `enum_path_builder.rs` includes:
```rust
EnumData {
    variant_chain: ctx.variant_chain.clone(),
    applicable_variants: HashSet::new(),  // NEW
    variant_chain_root_example: None,      // NEW
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
    // This happens at EVERY enum level (not just root)
    let partial_roots = build_partial_root_examples(
        &enum_examples,
        &child_paths,
        ctx,
    );

    // Store partial roots on the enum root path so parent can access them
    let mut root_mutation_path = root_mutation_path;
    root_mutation_path.partial_root_examples = Some(partial_roots.clone());

    // If we're at the actual root level (empty variant chain),
    // populate variant_chain_root_example on all paths
    if ctx.variant_chain.is_empty() {
        populate_variant_chain_root_examples(&mut child_paths, &partial_roots);
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
/// Keys are FULL variant chains (e.g., [WithMiddleStruct, VariantB]) with NO stripping.
/// Uses BTreeMap for deterministic ordering in tests.
fn build_partial_root_examples(
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> BTreeMap<Vec<VariantName>, Value> {
    let mut partial_roots = BTreeMap::new();

    // Extract all unique FULL variant chains from child paths
    let unique_chains: HashSet<Vec<VariantName>> = child_paths
        .iter()
        .filter_map(|p| {
            p.enum_data.as_ref()
                .filter(|ed| !ed.variant_chain.is_empty())
                .map(|ed| extract_variant_names(&ed.variant_chain))
        })
        .collect();

    // For each unique FULL chain, build the partial root from this enum down
    for full_chain in unique_chains {
        if let Some(root_example) = build_partial_root_for_chain(
            &full_chain,
            enum_examples,
            child_paths,
            ctx,
        ) {
            // Store using the FULL chain as key (no stripping)
            partial_roots.insert(full_chain, root_example);
        }
    }

    partial_roots
}

/// Build a partial root example for a specific variant chain
fn build_partial_root_for_chain(
    chain: &[VariantName],
    enum_examples: &[ExampleGroup],
    child_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> Option<Value> {
    // Get the first variant in the chain (this is OUR variant)
    let first_variant = chain.first()?;

    // Find the example for this variant from our enum_examples
    let base_example = enum_examples
        .iter()
        .find(|ex| ex.applicable_variants.contains(first_variant))
        .map(|ex| ex.example.clone())?;

    // If chain has more levels (nested enums), wrap the child's partial root
    if chain.len() > 1 {
        // Find child enum root path that has partial roots
        let remaining_chain = &chain[1..];

        for child in child_paths {
            // Look for enum root paths with partial_root_examples
            if let Some(child_partial_roots) = &child.partial_root_examples {
                // Check if child has a partial root for the remaining chain
                if let Some(nested_partial_root) = child_partial_roots.get(remaining_chain) {
                    // Wrap the nested partial root into our base example
                    // This is ONE level of wrapping
                    if let Some(wrapped) = wrap_nested_example(
                        &base_example,
                        nested_partial_root,
                        child,
                    ) {
                        return Some(wrapped);
                    }
                    // If wrapping failed, continue searching other children
                }
            }
        }

        // If we couldn't find child's partial root, log a warning and return base example
        // This can happen if child enum didn't build partial roots (shouldn't happen in normal flow)
        tracing::warn!(
            "Could not find partial root for chain {:?} in any child path. \
             Using base example without nested wrapping.",
            remaining_chain
        );
        Some(base_example)
    } else {
        // Chain length is 1 - no more nesting, just return our example
        Some(base_example)
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
        PathKind::EnumRoot { .. } => {
            // Enum root paths don't have a field name to wrap into
            tracing::debug!(
                "Cannot wrap into EnumRoot path - no field name available"
            );
            return None;
        }
        other => {
            tracing::warn!(
                "Unexpected path kind for wrapping: {:?}. Expected StructField.",
                other
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

/// Populate variant_chain_root_example on all paths (root level only)
fn populate_variant_chain_root_examples(
    paths: &mut [MutationPathInternal],
    partial_roots: &BTreeMap<Vec<VariantName>, Value>,
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_data {
            if !enum_data.variant_chain.is_empty() {
                let chain = extract_variant_names(&enum_data.variant_chain);
                if let Some(root_example) = partial_roots.get(&chain) {
                    enum_data.variant_chain_root_example = Some(root_example.clone());
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
    pub path_type: String,
    pub is_optional: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_path: Option<Vec<VariantPath>>,

    /// NEW: List of variants where this path is valid
    /// Example: ["BottomEnum::VariantB"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants: Option<Vec<String>>,

    /// NEW: Complete root example for single-step mutation
    /// Only present for paths nested in enums
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_variant_example: Option<Value>,
}
```

2. **Update `MutationPath::from_mutation_path_internal()` to populate new fields**

```rust
impl MutationPath {
    pub fn from_mutation_path_internal(internal: MutationPathInternal) -> Self {
        let path_info = if let Some(enum_data) = &internal.enum_data {
            Some(PathInfo {
                path_type: "enum_variant".to_string(),
                is_optional: internal.is_optional,
                enum_instructions: Some(generate_enum_instructions(&enum_data)),
                variant_path: Some(enum_data.variant_chain.clone()),

                // NEW: Populate applicable_variants
                applicable_variants: if !enum_data.applicable_variants.is_empty() {
                    Some(
                        enum_data.applicable_variants
                            .iter()
                            .map(|v| (*v).to_string())
                            .collect()
                    )
                } else {
                    None
                },

                // NEW: Populate root_variant_example
                root_variant_example: enum_data.variant_chain_root_example.clone(),
            })
        } else {
            None
        };

        MutationPath {
            path: internal.path,
            example: internal.example,
            path_info,
        }
    }
}
```

3. **Update `generate_enum_instructions()` for single-step guidance**

```rust
fn generate_enum_instructions(enum_data: &EnumData) -> String {
    if enum_data.variant_chain_root_example.is_some() {
        // We have a complete root example - provide single-step instructions
        format!(
            "This field is nested within enum variants. \
             Use the 'root_variant_example' for single-step mutation: \
             First set root to 'root_variant_example', then mutate this path. \
             Applicable variants: {}",
            if !enum_data.applicable_variants.is_empty() {
                enum_data.applicable_variants
                    .iter()
                    .map(|v| (*v).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "unknown".to_string()
            }
        )
    } else if !enum_data.variant_chain.is_empty() {
        // Fallback: multi-step instructions for backward compatibility
        format!(
            "This field requires setting parent enum variants. \
             Variant chain: {}",
            enum_data.variant_chain
                .iter()
                .map(|vp| format!("{}::{}", vp.enum_name, vp.variant))
                .collect::<Vec<_>>()
                .join(" → ")
        )
    } else {
        "This field is in an enum variant".to_string()
    }
}
```

### Phase 5: Populate `applicable_variants`

**Location:** `enum_path_builder.rs` - Update `process_children()`

The `applicable_variants` field needs to be populated during path processing. This tells the user which variants make a particular path valid.

```rust
/// Process children for each variant and collect all child paths
fn process_children(
    ctx: &RecursionContext,
    type_info: &TypeInfo,
    enum_examples: &[ExampleGroup],
) -> Vec<MutationPathInternal> {
    let mut all_child_paths = Vec::new();

    for example_group in enum_examples {
        for variant_name in &example_group.applicable_variants {
            // Get the variant value from the example
            if let Some(variant_value) = example_group.example.get(variant_name) {
                // Create new recursion context with this variant
                let mut new_ctx = ctx.clone();
                new_ctx.variant_chain.push(VariantPath {
                    enum_name: type_info.short_name.clone(),
                    variant: variant_name.clone(),
                });

                // Recurse into the variant's value
                let mut child_paths = crate::builder::build_mutation_paths_internal(
                    variant_value,
                    type_info,
                    &new_ctx,
                );

                // POPULATE applicable_variants: Track which variant made these paths valid
                for child_path in &mut child_paths {
                    if let Some(enum_data) = &mut child_path.enum_data {
                        // Add this variant to the list of variants that make this path valid
                        let variant_full_name = format!(
                            "{}::{}",
                            type_info.short_name,
                            variant_name
                        );
                        enum_data.applicable_variants.insert(variant_full_name);
                    }
                }

                all_child_paths.extend(child_paths);
            }
        }
    }

    // Deduplicate paths with the same path string
    // When paths appear in multiple variants, merge their applicable_variants
    let mut path_map: BTreeMap<String, MutationPathInternal> = BTreeMap::new();

    for path in all_child_paths {
        path_map
            .entry(path.path.clone())
            .and_modify(|existing| {
                // Merge applicable_variants from duplicate paths
                if let (Some(existing_data), Some(new_data)) =
                    (&mut existing.enum_data, &path.enum_data)
                {
                    existing_data.applicable_variants.extend(
                        new_data.applicable_variants.iter().cloned()
                    );
                }
            })
            .or_insert(path);
    }

    path_map.into_values().collect()
}
```

**Key points:**

1. As we process each variant, we add its full name (`EnumName::VariantName`) to `applicable_variants` on all child paths
2. When deduplicating paths, we merge the `applicable_variants` sets
3. This ensures paths that appear in multiple variants show all applicable variants in the output

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
    "root_variant_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": { "VariantA": 123 },
          "value": 42
        }
      }
    },
    "enum_instructions": "This field is nested within enum variants. Use the 'root_variant_example' for single-step mutation: First set root to 'root_variant_example', then mutate this path. Applicable variants: TestVariantChainEnum::WithMiddleStruct"
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
    "root_variant_example": {
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
    "enum_instructions": "This field is nested within enum variants. Use the 'root_variant_example' for single-step mutation: First set root to 'root_variant_example', then mutate this path. Applicable variants: BottomEnum::VariantB"
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

### Verification Checklist

- [ ] `.middle_struct` has `root_variant_example` with only `WithMiddleStruct` wrapper
- [ ] `.middle_struct.nested_enum.name` has `root_variant_example` with both `WithMiddleStruct` and `VariantB`
- [ ] `.middle_struct.nested_enum.name` shows `applicable_variants: ["BottomEnum::VariantB"]`
- [ ] `.middle_struct.nested_enum.value` shows `applicable_variants: ["BottomEnum::VariantA", "BottomEnum::VariantB"]` (appears in both)
- [ ] Root examples use correct variants (VariantB for `.name`, not VariantA)
- [ ] No recursive/infinite structures in root examples
- [ ] Root-level paths have `path_info: null` (not nested in enums)

## Implementation Order

Implement phases in order to maintain working code at each step:

1. **Phase 1**: Add `partial_root_examples` field to `MutationPathInternal` (data structure only, not used yet)
2. **Phase 3**: Implement helper functions (`build_partial_root_examples`, `build_partial_root_for_chain`, `wrap_nested_example`, `populate_variant_chain_root_examples`, `extract_variant_names`)
3. **Phase 2**: Update `create_result_paths()` to call the new helper functions
4. **Phase 5**: Update `process_children()` to populate `applicable_variants`
5. **Phase 4**: Update output structures in `types.rs` to expose new fields
6. **Test**: Run against `TestVariantChainEnum` and verify all checklist items

**Why this order?**

- Phase 1 adds storage without breaking existing code
- Phase 3 adds helper functions that aren't called yet (safe to add)
- Phase 2 connects the helpers into the main flow
- Phase 5 populates `applicable_variants` needed by Phase 4's output
- Phase 4 exposes everything to the user
- Testing validates the complete implementation

## Potential Issues and Solutions

### Issue 1: Circular References in Root Examples

**Problem**: If enum A contains enum B which contains enum A, building root examples could create infinite structures.

**Solution**: The recursion context already tracks depth and prevents infinite recursion during schema traversal. The `partial_root_examples` are built during the return path of existing recursion, so they inherit the same depth limits.

### Issue 2: Memory Usage with Deep Nesting

**Problem**: Deep nesting (5+ enum levels) creates large root examples stored on every path.

**Solution**:
1. Root examples are only stored on enum root paths (one per enum level)
2. Leaf paths reference these via `variant_chain_root_example` (shared, not duplicated)
3. If memory becomes an issue, consider adding a config option to limit root example depth

### Issue 3: BTreeMap Key Ordering

**Problem**: `Vec<VariantName>` as BTreeMap key requires `Ord` implementation.

**Solution**: Ensure `VariantName` type implements `Ord`, or use a wrapper type. If `VariantName` is a type alias for `String`, it already implements `Ord`.

### Issue 4: Missing Partial Roots During Lookup

**Problem**: Parent enum looks for child's partial root but doesn't find it.

**Current handling**: Logs warning and uses base example without nesting (lines 248-254).

**Why acceptable**: This indicates a bug in the building process (child should have built roots). The warning makes it visible during testing.
