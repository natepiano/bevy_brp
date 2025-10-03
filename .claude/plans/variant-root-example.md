# Plan: Build Root Examples Bottom-Up During Enum Recursion

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

## Problem Detail

## Summary of Changes

This plan fixes the multi-step mutation requirement for deeply nested enum fields by building complete root examples during recursion. The implementation adds:

**New Fields:**
- `MutationPathInternal.partial_root_examples`: Stores partial roots at each enum level
- `EnumPathData.applicable_variants`: Tracks which variants make a path valid
- `EnumPathData.variant_chain_root_example`: Complete root example for the path
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
  → Populates variant_chain_root_example on all matching descendant paths
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

**1a. Update `VariantName` to support BTreeMap and HashSet usage:**

The plan uses `BTreeMap<Vec<VariantName>, Value>` for partial root examples and `HashSet<Vec<VariantName>>` for collecting unique chains. For these to work, `VariantName` must implement both `Ord` (for BTreeMap) and `Hash` (for HashSet).

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct VariantName(String);
```

**Why:** `BTreeMap` requires keys to implement `Ord`, and `HashSet` requires `Hash`. The plan uses BTreeMap for deterministic ordering in tests and HashSet for collecting unique variant chains (Phase 3, line 303). This matches the pattern for `StructFieldName` in the codebase, which is also a newtype wrapper around `String` and derives both `Hash` and `Ord`.

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

**1c. Add `applicable_variants` and `variant_chain_root_example` fields to `EnumPathData`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumPathData {
    /// The chain of variant selections from root to this point
    pub variant_chain: Vec<VariantPath>,

    /// NEW: Variant names where this path is valid
    /// Example: [VariantName("VariantB"), VariantName("VariantA")]
    /// Populated during path processing in Phase 5
    /// Converted to fully-qualified names during serialization in Phase 4
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applicable_variants: Vec<VariantName>,

    /// NEW: Complete root example for single-step mutation
    /// Only populated at root level (when ctx.variant_chain is empty)
    /// Copied from partial_root_examples in Phase 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_chain_root_example: Option<Value>,
}
```

**Note:** Ensure `EnumPathData` initialization in `enum_path_builder.rs` includes:
```rust
EnumPathData {
    variant_chain: ctx.variant_chain.clone(),
    applicable_variants: Vec::new(),  // NEW
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
    // This happens at EVERY enum root path (paths where enum_example_groups exists)
    // - For path "" (TestVariantChainEnum): builds roots for all descendants
    // - For path ".middle_struct.nested_enum" (BottomEnum): builds roots for its children
    let partial_roots = build_partial_root_examples(
        &enum_examples,
        &child_paths,
        ctx,
    );

    // Store partial roots on this enum's root path so parent enums can access them
    // Parent finds these by searching child_paths for paths with partial_root_examples.is_some()
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
    pub root_variant_example: Option<Value>,
}
```

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

        // NEW: Extract applicable_variants and root_variant_example from enum_data
        let (applicable_variants, root_variant_example) = path
            .enum_data
            .as_ref()
            .map(|enum_data| {
                let variants = if !enum_data.applicable_variants.is_empty() {
                    Some(enum_data.applicable_variants.clone())
                } else {
                    None
                };
                (variants, enum_data.variant_chain_root_example.clone())
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
                root_variant_example,
            },
            examples,
            example,
        }
    }
}
```

3. **Update `generate_enum_instructions()` for single-step guidance**

```rust
fn generate_enum_instructions(enum_data: &EnumPathData) -> String {
    let applicable_str = if !enum_data.applicable_variants.is_empty() {
        // VariantName already contains fully qualified names - just convert to strings
        enum_data.applicable_variants
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        "unknown".to_string()
    };

    format!(
        "This field is nested within enum variants. \
         Use the 'root_variant_example' for single-step mutation: \
         First set root to 'root_variant_example', then mutate this path. \
         Applicable variants: {applicable_str}"
    )
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

## Design Review Skip Notes

### DESIGN-1: Missing explanation for handling IndexedElement and ArrayElement in wrapping logic - **Verdict**: REJECTED
- **Status**: REJECTED - Finding was incorrect
- **Location**: Phase 3: Build Partial Roots by Wrapping Children - wrap_nested_example function
- **Issue**: Original finding claimed the plan doesn't explain whether wrapping should work for IndexedElement paths (created by tuple variants)
- **Reasoning**: Investigation revealed the plan correctly implements two separate complementary mechanisms: (1) Field-based wrapping for struct variants (wrap_nested_example), and (2) Index-based assembly for tuple variants (build_variant_example). IndexedElement paths are intentionally excluded from wrapping because they participate in a different construction mechanism. The match arm that rejects IndexedElement in wrap_nested_example is not a gap - it's defensive programming that catches architectural violations. The code is self-documenting through its structure.
- **Decision**: User agreed with rejection - plan correctly handles both struct and tuple variants through appropriate separate mechanisms
