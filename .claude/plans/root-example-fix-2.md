# Root Example Fix 2: Replace PartialRootExample with RootExampleInfo Enum

## Problem Statement

The current implementation has a fundamental flaw: it allows contradictory states where `root_example` and `root_example_unavailable_reason` can both be present simultaneously.

**Current contradictory output:**
```json
{
  "path_info": {
    "root_example": {
      "Custom": {
        "Image": {...}
      }
    },
    "root_example_unavailable_reason": "Cannot construct Custom variant via BRP..."
  }
}
```

This tells users both:
1. "Here's how to construct it" (via `root_example`)
2. "You can't construct it" (via `root_example_unavailable_reason`)

**Root causes:**
1. `PartialRootExample` struct allows storing both `example` and `unavailable_reason`
2. `PathInfo` has four separate optional fields that should be mutually exclusive
3. `enum_instructions` and `applicable_variants` appear even when the variant is unconstructible

## Design Solution

### Type-Safe Enum Representation

Replace the struct-based approach with a single enum that enforces mutual exclusivity:

```rust
/// Root example information for enum variant paths
///
/// This enum provides internal type safety during building to prevent creating
/// contradictory states (having both root_example and unavailable_reason).
///
/// IMPORTANT: This is an INTERNAL type only. It is NOT serialized directly.
/// During conversion to MutationPathExternal, the enum is destructured back
/// into separate optional fields in PathInfo to maintain JSON format stability.
#[derive(Debug, Clone)]
pub enum RootExampleInfo {
    /// Root example is available - variant can be constructed via BRP
    Available {
        /// Complete root example showing how to construct this variant
        root_example: Value,
        /// Instructions for using the root_example with BRP mutations
        enum_instructions: String,
        /// Variants that share the same signature and support this field path
        applicable_variants: Vec<VariantName>,
    },
    /// Root example unavailable - variant cannot be constructed via BRP
    Unavailable {
        /// Explanation of why this variant cannot be constructed
        reason: String,
    },
}
```

### JSON Output Format (UNCHANGED from current format)

**CRITICAL**: The final JSON output format remains **identical** to the current format. The `RootExampleInfo` enum is used internally for type safety but is **extracted back to separate fields** during serialization.

**Constructible variant (Available) - extracted to flat structure:**
```json
{
  "path_info": {
    "mutability": "mutable",
    "root_example": {
      "Custom": {
        "Url": {
          "hotspot": [5000, 5000],
          "url": "Hello, World!"
        }
      }
    },
    "enum_instructions": "First, set the root mutation path to 'root_example', then you can mutate the '.0.0.url' path. See 'applicable_variants' for which variants support this field.",
    "applicable_variants": ["CustomCursor::Url"]
  }
}
```

**Unconstructible variant (Unavailable) - extracted to flat structure:**
```json
{
  "path_info": {
    "mutability": "mutable",
    "root_example_unavailable_reason": "Cannot construct Custom variant via BRP due to incomplete field data: tuple element 0 (CustomCursor): contains non-mutable descendants (see 'CustomCursor' mutation_paths for details). This variant's mutable fields can only be mutated if the entity is already set to this variant by your code."
  }
}
```

**Why this approach:**
- Internal type safety prevents contradictory states during building
- JSON output format remains stable (no breaking changes)
- Only 2 types should change in comparison (the bug fixes, not format changes)
- Consumers of the JSON see no difference except fixed data

## Implementation Plan

### Step 1: Add RootExampleInfo Enum to types.rs

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Add the enum (INTERNAL type only - not serialized):**
```rust
/// Root example information for enum variant paths
///
/// This enum provides internal type safety during building to prevent creating
/// contradictory states (having both root_example and unavailable_reason).
///
/// IMPORTANT: This is an INTERNAL type only. It is NOT serialized directly.
/// During conversion to MutationPathExternal, the enum is destructured back
/// into separate optional fields in PathInfo to maintain JSON format stability.
#[derive(Debug, Clone)]
pub enum RootExampleInfo {
    /// Root example is available - variant can be constructed via BRP
    Available {
        /// Complete root example showing how to construct this variant
        root_example: Value,
        /// Instructions for using the root_example with BRP mutations
        enum_instructions: String,
        /// Variants that share the same signature and support this field path
        applicable_variants: Vec<VariantName>,
    },
    /// Root example unavailable - variant cannot be constructed via BRP
    Unavailable {
        /// Explanation of why this variant cannot be constructed
        reason: String,
    },
}
```

**Update EnumPathData (~line 252):**
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,

    // REMOVE:
    // pub applicable_variants: Vec<VariantName>,

    // ADD:
    pub root_example_info: Option<RootExampleInfo>,

    // Note: applicable_variants moved into RootExampleInfo::Available
    // to eliminate data duplication and ensure consistency
}
```

**PathInfo struct (~line 199) - NO CHANGES:**

**IMPORTANT**: PathInfo keeps its existing 4 separate optional fields unchanged. These fields are populated by extracting from RootExampleInfo during serialization (see Step 5a).

```rust
pub struct PathInfo {
    pub path_kind: PathKind,
    pub type_name: TypeName,
    pub type_kind: TypeKind,
    pub mutability: Mutability,

    // These four fields STAY as-is (no changes):
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable_variants: Option<Vec<VariantName>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_example_unavailable_reason: Option<String>,

    // Keep other fields...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability_reason: Option<MutabilityReason>,
}
```

**Why PathInfo is unchanged:**
- Maintains JSON format stability (no breaking changes)
- resolve_enum_data_mut() extracts from RootExampleInfo back to these 4 separate fields (see Step 5a)
- Only internal building uses RootExampleInfo enum for type safety

### Step 2: Remove PartialRootExample from enum_builder

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/mod.rs`

**Remove:**
```rust
pub use enum_path_builder::PartialRootExample;
```

### Step 3: Update enum_path_builder.rs

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Remove PartialRootExample struct definition (~line 40):**
```rust
// DELETE THIS:
#[derive(Debug, Clone)]
pub struct PartialRootExample {
    pub example: Value,
    pub unavailable_reason: Option<String>,
}
```

**Update imports (~line 47-48):**

Add the `RootExampleInfo` import to the `super::super::types` import group, immediately after `EnumPathData` since they work together.

**Current imports (lines 47-48):**
```rust
use super::super::types::EnumPathData;
use super::super::types::ExampleGroup;
```

**Updated imports (add line 48):**
```rust
use super::super::types::EnumPathData;
use super::super::types::RootExampleInfo;  // ADD - new line 48
use super::super::types::ExampleGroup;      // becomes line 49
```

**Why this placement:** The `super::super::types` imports are grouped together. Adding `RootExampleInfo` after `EnumPathData` maintains logical grouping since `EnumPathData` contains a `root_example_info: Option<RootExampleInfo>` field.

**Note:** Line numbers may shift after earlier edits - search for "use super::super::types::EnumPathData;" to find the correct insertion point.

**Update ProcessChildrenResult type alias (~line 75-80):**

This type alias is used as the return type for `process_children()` function and must be updated to use `RootExampleInfo` instead of `PartialRootExample`.

```rust
// OLD:
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, PartialRootExample>,
);

// NEW:
type ProcessChildrenResult = (
    Vec<ExampleGroup>,
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, RootExampleInfo>,
);
```

**Update HashMap type signature (~line 446):**
```rust
// OLD:
HashMap<Vec<VariantName>, PartialRootExample>

// NEW:
HashMap<Vec<VariantName>, RootExampleInfo>
```

**Update EnumPathData initialization in build_enum_root_path():**

**Location:** Find the function `build_enum_root_path()`, search for the EnumPathData initialization (look for "Generate `EnumPathData` only if we have a variant chain" comment).

**Context marker:** Inside the `else` block after `if ctx.variant_chain.is_empty()`.

**OLD initialization (references removed fields):**
```rust
Some(EnumPathData {
    variant_chain:                   ctx.variant_chain.clone(),
    applicable_variants:             Vec::new(),
    root_example:                    None,
    root_example_unavailable_reason: None,
})
```

**NEW initialization (uses new struct shape):**
```rust
Some(EnumPathData {
    variant_chain:    ctx.variant_chain.clone(),
    root_example_info: None,
})
```

**Rationale:** Step 1 removes `applicable_variants`, `root_example`, and `root_example_unavailable_reason` fields from EnumPathData, replacing them with a single `root_example_info: Option<RootExampleInfo>` field. This initialization must be updated to match the new struct definition.

**Update build_partial_root_examples function (~line 545):**

Change return type:
```rust
fn build_partial_root_examples(
    // ... params ...
) -> HashMap<Vec<VariantName>, RootExampleInfo> {  // Changed
```

**CRITICAL: Delete old .err() pattern (lines 596-604):**

The current code extracts the error with `.err()` and stores it in an `unavailable_reason` variable. This must be completely removed and replaced with the match pattern below.

**DELETE these lines:**
```rust
// Determine if this variant can be constructed via BRP
let unavailable_reason = analyze_variant_constructibility(
    variant_name,
    signature,
    variant_mutability,
    child_mutation_paths,
    ctx,
)
.err();
```

**Why:** The `.err()` pattern extracts `Option<String>` but we need to create `RootExampleInfo` directly from the `Result`. The `unavailable_reason` variable is replaced by the `root_info` variable created via match expression.

**Restructure variant processing order (lines 596-680):**

The current code processes variants in this order:
1. Lines 596-604: Determine unavailability (using `.err()`) ← DELETE THIS
2. Lines 606-609: Collect nested enum chains
3. Lines 610-657: Process nested enum chains (uses `unavailable_reason`) ← MOVE THIS AFTER root_info
4. Lines 659-672: Build the variant's example
5. Lines 674-680: Insert into partial_root_examples (uses `PartialRootExample` struct) ← UPDATE THIS

This must be restructured to:
1. Build the variant's example FIRST (no dependencies on unavailability)
2. Determine constructibility using match (creates `root_info` directly)
3. Process nested enum chains (depends on `root_info` being created)
4. Insert both nested chains and main variant (using `RootExampleInfo` enum)

Update building logic (~line 596-680 - COMPLETE REPLACEMENT):
```rust
// Build root example for this variant's chain itself
let example = if nested_enum_chains.is_empty() {
    spawn_example
} else {
    build_variant_example_for_chain(
        signature,
        variant_name,
        child_mutation_paths,
        &this_variant_chain,
        ctx,
    )
};

// Analyze constructibility and create RootExampleInfo
let root_info = match analyze_variant_constructibility(
    variant_name,
    signature,
    variant_mutability,
    child_mutation_paths,
    ctx,
) {
    Ok(()) => {
        // Constructible - create Available variant with placeholder enum_instructions
        // Instructions will be filled in Step 4 when mutation_path is available
        RootExampleInfo::Available {
            root_example: example,
            enum_instructions: String::new(), // Placeholder - filled in Step 4
            // Use the FULL signature group's variants (available in outer loop as 'variants')
            // not just the single variant being processed - this ensures users see ALL
            // variants that share this signature and support this mutation path
            applicable_variants: variants.clone(),
        }
    }
    Err(reason) => {
        // Unconstructible - create Unavailable variant
        RootExampleInfo::Unavailable { reason }
    }
};

// Process nested enum chains BEFORE inserting the main variant
// (This happens earlier in the function, around line 610-657)
for nested_chain in &nested_enum_chains {
    let example = build_variant_example_for_chain(
        signature,
        variant_name,
        child_mutation_paths,
        nested_chain,
        ctx,
    );

    // Determine unavailability using hierarchical selection
    // RATIONALE: We need to capture the ACTUAL blocking issue:
    // 1. If parent is unconstructible → parent's reason is the blocker (child is unreachable)
    // 2. If parent IS constructible → check if nested chain has its OWN unavailability
    let nested_info = match &root_info {
        RootExampleInfo::Unavailable { reason } => {
            // Parent is unconstructible → child is unreachable, use parent's reason
            RootExampleInfo::Unavailable { reason: reason.clone() }
        }
        RootExampleInfo::Available { .. } => {
            // Parent is constructible → check if THIS nested chain is unconstructible
            // The child enum was already processed recursively and its enum_path_data
            // was populated with root_example_info. Look it up.

            // CRITICAL FIELD ACCESS CHANGE:
            // OLD: .and_then(|data| data.root_example_unavailable_reason.clone())
            // NEW: .and_then(|data| data.root_example_info.clone())
            child_mutation_paths
                .iter()
                .find_map(|child| {
                    child
                        .enum_path_data
                        .as_ref()
                        .filter(|data| data.variant_chain == *nested_chain)
                        .and_then(|data| data.root_example_info.clone())  // Field changed from root_example_unavailable_reason
                })
                .unwrap_or_else(|| RootExampleInfo::Available {
                    root_example: example.clone(),
                    enum_instructions: String::new(),
                    // Use full signature group variants for consistency
                    applicable_variants: variants.clone(),
                })
        }
    };

    partial_root_examples.insert(nested_chain.clone(), nested_info);
}

// Finally, insert the main variant's root_info
partial_root_examples.insert(this_variant_chain.clone(), root_info);
```

**Verification Steps After Implementation:**

After making these changes, verify the transformation is complete:

1. **Check for removed patterns:**
   ```bash
   # Should return ZERO matches (old variable removed)
   rg 'let unavailable_reason =' mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs

   # Should return ZERO matches (old Result-to-Option conversion removed)
   rg 'analyze_variant_constructibility.*\n.*\.err\(\)' mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs
   ```

   **Note:** The file may contain other `.err()` calls elsewhere (e.g., for different error handling patterns). The verification above checks specifically for the `analyze_variant_constructibility(...).err()` pattern that's being removed, not all `.err()` calls in the file.

2. **Check for new patterns:**
   ```bash
   # Should find matches (new match pattern added)
   rg 'match analyze_variant_constructibility' mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs

   # Should find matches (new variable added)
   rg 'let root_info =' mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs
   ```

3. **Check restructuring:**
   ```bash
   # Verify nested chain loop comes AFTER root_info creation
   # The nested chain loop should access &root_info, not unavailable_reason
   rg -A 5 'for nested_chain in &nested_enum_chains' mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs
   ```

**Critical Implementation Note:** The nested chain propagation loop (shown above) is essential for hierarchical unavailability. The current code at lines 610-657 uses `unavailable_reason` variable and accesses `data.root_example_unavailable_reason`. After Step 1's changes to EnumPathData, this field becomes `root_example_info`, so the nested chain logic must:
1. Match on `root_info` to determine if parent is constructible
2. Access `data.root_example_info` (not `root_example_unavailable_reason`) from child paths
3. Create `RootExampleInfo` for nested chains (not `PartialRootExample`)

**Important Changes to applicable_variants Handling:**
- `RootExampleInfo::Available.applicable_variants` now contains ALL variants from the signature group (via `variants.clone()`)
- `EnumPathData.applicable_variants` field is removed (Step 1) to eliminate data duplication
- The extraction logic in `resolve_enum_data_mut()` (Step 5a) must get `applicable_variants` from `root_example_info.Available`, not from the top-level EnumPathData field

### Code Removal: applicable_variants Population in process_signature_path()

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Location:** In `process_signature_path()` function, find the loop that starts with comment "Track which variants make these child paths valid". This appears after the `let group_paths = process_signature_path(...)` call and before the function returns.

**Context markers:**
- **Before:** `all_child_paths.extend(group_paths);`
- **After:** The loop ends, followed by other processing or function return

**DELETE the entire loop that populates applicable_variants:**
```rust
// Track which variants make these child paths valid
// Only populate for DIRECT children (not grandchildren nested deeper)
for child_path in &mut child_paths {
    if let Some(enum_data) = &mut child_path.enum_path_data {
        // Check if this path is a direct child of the current enum level
        // Direct children have variant_chain.len() == ctx.variant_chain.len() + 1
        if enum_data.variant_chain.len() == ctx.variant_chain.len() + 1 {
            // Add all variants from this signature group
            // (all variants in a group share the same signature/structure)
            for variant_name in applicable_variants {
                enum_data.applicable_variants.push(variant_name.clone());
            }
        }
    }
}
```

**Rationale:**
- This code populates `EnumPathData.applicable_variants` during recursion
- Step 1 removes the `EnumPathData.applicable_variants` field entirely (making this code invalid)
- Step 3's match expression replaces this functionality by constructing `RootExampleInfo::Available` with `applicable_variants: variants.clone()` directly during processing in `build_partial_root_examples()`
- The `variants` variable in `build_partial_root_examples()` already contains all variants for the signature group
- No other code depends on this intermediate population step
- This loop does ONLY ONE THING: populate the field being removed. It has no side effects or other dependencies.

**Verification after removal:**
```bash
# Check that the field access pattern in loops is removed
# (Note: May still match initialization in build_enum_root_path - that's addressed in a separate step)
rg -C3 'for.*child_path.*child_paths' mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs | rg 'applicable_variants'

# Should return ZERO matches - verifies the loop that populates applicable_variants is gone
```

**Note:** The `enum_instructions` field is left empty during building in Step 3 because the mutation path is not available at that point. Step 4 will fill in the instructions when copying RootExampleInfo from the partials HashMap, where path.mutation_path is accessible.

### Step 4: Update support.rs

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs`

**REPLACE the import at line 12:**

**OLD import (line 12):**
```rust
use super::enum_builder::PartialRootExample;
```

**NEW import (REPLACE line 12):**
```rust
use super::types::RootExampleInfo;
```

**Rationale:** `PartialRootExample` is only used in the function parameter type of `populate_root_examples_from_partials` and nowhere else in this file. Since the function is being completely rewritten to use `RootExampleInfo` instead, the old import should be replaced, not kept alongside.

**Update populate_root_examples_from_partials (~line 165):**

**CRITICAL:** This function must be completely rewritten. The current implementation (lines 169-180) assigns to `enum_data.root_example` and `enum_data.root_example_unavailable_reason` fields that Step 1 removes from EnumPathData. The new implementation assigns to `enum_data.root_example_info` instead.

**OLD implementation (lines 169-180) - COMPLETELY REPLACE:**
```rust
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
        {
            // Populate both fields from the struct (single lookup!)
            if let Some(data) = partials.get(&enum_data.variant_chain) {
                enum_data.root_example = Some(data.example.clone());  // Field removed in Step 1
                enum_data.root_example_unavailable_reason = data.unavailable_reason.clone();  // Field removed in Step 1
            }
        }
    }
```

**NEW implementation (COMPLETE REPLACEMENT of function body):**

```rust
/// Populate `root_example_info` from partial root examples for enum paths
///
/// This function generates complete RootExampleInfo with enum_instructions filled in
/// using the mutation_path from each MutationPathInternal.
pub fn populate_root_examples_from_partials(
    paths: &mut [MutationPathInternal],
    partials: &HashMap<Vec<VariantName>, RootExampleInfo>,  // Changed type
) {
    for path in paths {
        if let Some(enum_data) = &mut path.enum_path_data
            && !enum_data.variant_chain.is_empty()
        {
            if let Some(info) = partials.get(&enum_data.variant_chain) {
                // Generate complete RootExampleInfo with instructions filled in
                let complete_info = match info {
                    RootExampleInfo::Available {
                        root_example,
                        applicable_variants,
                        ..
                    } => RootExampleInfo::Available {
                        root_example: root_example.clone(),
                        enum_instructions: format!(
                            "First, set the root mutation path to 'root_example', then you can mutate the '{}' path. See 'applicable_variants' for which variants support this field.",
                            &path.mutation_path
                        ),
                        applicable_variants: applicable_variants.clone(),
                    },
                    RootExampleInfo::Unavailable { reason } => {
                        RootExampleInfo::Unavailable {
                            reason: reason.clone(),
                        }
                    }
                };
                enum_data.root_example_info = Some(complete_info);
            }
        }
    }
}
```

### Step 5: Verify mutation_path_internal.rs PathInfo Construction (NO CHANGES)

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**IMPORTANT**: The PathInfo construction in `into_mutation_path_external()` method (~line 98-118) requires NO CHANGES. The destructuring and field assignment remain exactly the same to maintain JSON format compatibility.

**Current code that stays unchanged:**
```rust
// Destructuring (line 98-99) - STAYS THE SAME:
let (enum_instructions, applicable_variants, root_example, root_example_unavailable_reason) =
    self.resolve_enum_data_mut();

// PathInfo construction (lines 103-116) - STAYS THE SAME:
MutationPathExternal {
    description,
    path_info: PathInfo {
        path_kind: self.path_kind,
        type_name: self.type_name,
        type_kind,
        mutability: self.mutability,
        mutability_reason: self
            .mutability_reason
            .as_ref()
            .and_then(Option::<Value>::from),
        enum_instructions,
        applicable_variants,
        root_example,
        root_example_unavailable_reason,
    },
    path_example,
}
```

**Why no changes:**
- The resolve_enum_data_mut() method signature stays the same (returns 4-tuple)
- Only the IMPLEMENTATION of resolve_enum_data_mut() changes to extract from RootExampleInfo (see Step 5a)
- This maintains exact same JSON output format - only fixing the bug in 2 types, not changing all enum types

### Step 5a: Update resolve_enum_data_mut() to Extract RootExampleInfo Back to Separate Fields

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**Update resolve_enum_data_mut() method implementation (~line 189):**

**IMPORTANT**: This method maintains the SAME return type (4-tuple of optional fields) to preserve JSON format compatibility. The method destructures the internal RootExampleInfo enum back into separate fields for serialization.

```rust
// OLD implementation (uses removed EnumPathData.applicable_variants field):
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,
    Option<Vec<VariantName>>,
    Option<Value>,
    Option<String>,
) {
    if !matches!(
        self.mutability,
        Mutability::Mutable | Mutability::PartiallyMutable
    ) {
        return (None, None, None, None);
    }

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
                enum_data.root_example_unavailable_reason,
            )
        })
}

// NEW implementation (extracts from RootExampleInfo back to separate fields):
fn resolve_enum_data_mut(
    &mut self,
) -> (
    Option<String>,           // enum_instructions
    Option<Vec<VariantName>>, // applicable_variants
    Option<Value>,            // root_example
    Option<String>,           // root_example_unavailable_reason
) {
    if !matches!(
        self.mutability,
        Mutability::Mutable | Mutability::PartiallyMutable
    ) {
        return (None, None, None, None);
    }

    self.enum_path_data
        .take()
        .map_or((None, None, None, None), |enum_data| {
            // Extract RootExampleInfo and destructure back to separate fields
            // Instructions are already filled in Step 4, so just destructure
            match enum_data.root_example_info {
                Some(root_info) => match root_info {
                    RootExampleInfo::Available {
                        root_example,
                        enum_instructions,
                        applicable_variants,
                    } => (
                        Some(enum_instructions),
                        Some(applicable_variants),
                        Some(root_example),
                        None,
                    ),
                    RootExampleInfo::Unavailable { reason } => {
                        (None, None, None, Some(reason))
                    }
                },
                None => (None, None, None, None),
            }
        })
}
```

**Why this approach:**
- **Type safety during building**: RootExampleInfo enum prevents contradictory states internally
- **JSON stability**: Same 4-tuple return type maintains exact same JSON output format
- **Backward compatibility**: Only 2 buggy types change (fixing the bug), all other types unchanged
- **Clean separation**: Internal representation (RootExampleInfo) vs serialized output (separate fields)
- **Single-phase instruction generation**: Instructions are filled immediately in Step 4 where mutation_path is available, eliminating temporal coupling and ensuring RootExampleInfo is never in an incomplete state

### Step 5b: Update path_builder.rs

**File:** `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

**Context:** This file handles non-enum types (structs, tuples, arrays) that are nested inside enum variants. The `build_final_result()` function converts assembled examples to the format expected by `populate_root_examples_from_partials()`.

**REPLACE import at line 28:**

**OLD:**
```rust
use super::super::mutation_path_builder::enum_builder::PartialRootExample;
```

**NEW:**
```rust
use super::super::types::RootExampleInfo;
```

**Update conversion logic in build_final_result() function:**

**Location:** Find the section that converts `Value` partials (look for comment "Convert Value partials to PartialRootExample").

**Context markers:**
- **Before:** The `if let Some(partials) = partial_root_examples` block
- **After:** Call to `support::populate_root_examples_from_partials()`

**OLD implementation (~lines 514-533):**
```rust
            // Convert Value partials to PartialRootExample for populate function
            // Non-enum types don't have unavailability reasons (always None)
            let partials_with_reasons: HashMap<Vec<VariantName>, PartialRootExample> = partials
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        PartialRootExample {
                            example:            v.clone(),
                            unavailable_reason: None,
                        },
                    )
                })
                .collect();

            // Populate root_example from partial_root_examples for children with enum_path_data
            support::populate_root_examples_from_partials(
                &mut paths_to_expose,
                &partials_with_reasons,
            );
```

**NEW implementation:**
```rust
            // Convert Value partials to RootExampleInfo for populate function
            // Non-enum types are always constructible (Available), never Unavailable
            let partials_with_info: HashMap<Vec<VariantName>, RootExampleInfo> = partials
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        RootExampleInfo::Available {
                            root_example:        v.clone(),
                            enum_instructions:   String::new(), // Filled by populate function
                            applicable_variants: Vec::new(),    // Non-enum types don't have variants
                        },
                    )
                })
                .collect();

            // Populate root_example_info from partial_root_examples for children with enum_path_data
            support::populate_root_examples_from_partials(
                &mut paths_to_expose,
                &partials_with_info,
            );
```

**Rationale:**
- Non-enum types (structs, tuples, arrays) nested inside enum variants need to propagate partial root examples
- These types are always constructible (no variant constructibility issues), so they use `RootExampleInfo::Available`
- Empty `enum_instructions` and `applicable_variants` because non-enum types don't have enum-specific metadata
- Step 4's `populate_root_examples_from_partials()` will fill in the instructions using the mutation path

### Step 6: Verify Python Filtering Scripts (NO CHANGES)

**File:** `.claude/scripts/mutation_test/prepare.py`

**IMPORTANT**: NO CHANGES NEEDED. The filtering logic (~line 1039) continues to work as-is because the JSON format is unchanged.

**Current code that stays unchanged:**
```python
# Filtering logic - STAYS THE SAME:
if "root_example_unavailable_reason" in path_info_raw:
    excluded_count_val += 1
    reason_str = path_info_raw["root_example_unavailable_reason"]
    reason_preview = str(reason_str)[:80]
    print(f"  Excluding {type_name}{path}: {reason_preview}...", file=sys.stderr)
```

**Why no changes:**
- The JSON output format maintains the same flat structure with 4 separate optional fields
- Python scripts continue to check for `root_example_unavailable_reason` in the top-level path_info object
- Only the Rust internal representation uses RootExampleInfo enum for type safety during building

## Testing Strategy

### 1. Unit Test - Type Construction and Extraction

Verify that `RootExampleInfo` prevents impossible states and extracts correctly:

```rust
#[test]
fn test_root_example_info_available_extraction() {
    // Create Available variant
    let info = RootExampleInfo::Available {
        root_example: json!({"Custom": {"Url": "..."}}),
        enum_instructions: String::new(), // Filled by with_instructions()
        applicable_variants: vec!["System".to_string(), "Custom".to_string()],
    };

    // Fill instructions
    let info_with_instr = info.with_instructions(".0.0.flip_x");

    // Extract to tuple (simulating resolve_enum_data_mut logic)
    match info_with_instr {
        RootExampleInfo::Available {
            root_example,
            enum_instructions,
            applicable_variants,
        } => {
            assert!(root_example.is_object());
            assert!(enum_instructions.contains("root_example"));
            assert_eq!(applicable_variants.len(), 2);
        }
        _ => panic!("Expected Available variant"),
    }
}

#[test]
fn test_root_example_info_unavailable_extraction() {
    // Create Unavailable variant
    let info = RootExampleInfo::Unavailable {
        reason: "Cannot construct Custom variant...".to_string(),
    };

    // Extract to tuple (simulating resolve_enum_data_mut logic)
    match info {
        RootExampleInfo::Unavailable { reason } => {
            assert!(reason.contains("Cannot construct"));
        }
        _ => panic!("Expected Unavailable variant"),
    }
}
```

### 2. Integration Test - CursorIcon

Test with the real-world case that exposed the bug:

```bash
# Generate type guide
mcp__brp__brp_type_guide(types=["bevy_window::cursor::CursorIcon"])

# Verify Custom::Image variant (unconstructible)
# Check .0.0.flip_x path - should have root_example_unavailable_reason ONLY
jq '.type_guide["bevy_window::cursor::CursorIcon"].mutation_paths[".0.0.flip_x"].path_info' output.json

# Expected (flat structure):
{
  "mutability": "partially_mutable",
  "root_example_unavailable_reason": "Cannot construct Custom variant via BRP due to incomplete field data..."
}

# Verify NO root_example field present:
jq '.type_guide["bevy_window::cursor::CursorIcon"].mutation_paths[".0.0.flip_x"].path_info | has("root_example")' output.json
# Expected: false

# Verify System variant (constructible)
# Check .0 path - should have root_example, enum_instructions, applicable_variants
jq '.type_guide["bevy_window::cursor::CursorIcon"].mutation_paths[".0"].path_info' output.json

# Expected (flat structure):
{
  "mutability": "mutable",
  "root_example": {"System": "Default"},
  "enum_instructions": "First, set the root mutation path to 'root_example'...",
  "applicable_variants": ["System", "Custom"]
}

# Verify NO root_example_unavailable_reason field present:
jq '.type_guide["bevy_window::cursor::CursorIcon"].mutation_paths[".0"].path_info | has("root_example_unavailable_reason")' output.json
# Expected: false
```

### 3. Integration Test - TestMixedMutabilityEnum

Verify our test type shows correct unavailability:

```bash
# Check Multiple variant (unconstructible - PartiallyMutable)
# Should have root_example_unavailable_reason ONLY
jq '.type_guide["extras_plugin::TestMixedMutabilityEnum"].mutation_paths[".0"].path_info' output.json

# Expected (flat structure):
{
  "mutability": "partially_mutable",
  "root_example_unavailable_reason": "Cannot construct Multiple variant via BRP due to incomplete field data: tuple element 0 (TestMixedMutabilityCore): contains non-mutable descendants..."
}

# Verify NO root_example field present:
jq '.type_guide["extras_plugin::TestMixedMutabilityEnum"].mutation_paths[".0"].path_info | has("root_example")' output.json
# Expected: false
```

### 4. Mutation Test JSON Generation

Run full generation and verify only expected changes:

```bash
/create_mutation_test_json init
```

**Expected outcomes:**
1. No contradictory states (root_example + unavailable_reason simultaneously)
2. Available variants show root_example + enum_instructions + applicable_variants (no unavailable_reason)
3. Unavailable variants show root_example_unavailable_reason only (no root_example, enum_instructions, or applicable_variants)
4. Python filtering correctly excludes unavailable paths (unchanged behavior)
5. JSON format identical to before (flat structure maintained)

### 5. Baseline Comparison

After implementation:
```bash
# Should show changes ONLY in the 2 buggy types
python3 .claude/scripts/create_mutation_test_json/compare.py \
  .claude/transient/all_types_baseline.json \
  .claude/transient/all_types.json
```

**Expected: ONLY 2 types should show changes (the bug fixes):**
- `bevy_window::cursor::CursorIcon`: 22 paths fixed (removal of contradictory root_example)
- `extras_plugin::TestMixedMutabilityEnum`: 12 paths fixed (removal of contradictory root_example)

All other enum types should be UNCHANGED because the JSON format is identical.

## Expected Outcomes

### Before (Contradictory - THE BUG)
```json
{
  "path_info": {
    "mutability": "partially_mutable",
    "root_example": {"Custom": {"Image": {...}}},
    "enum_instructions": "First, set the root mutation path...",
    "applicable_variants": ["System", "Custom"],
    "root_example_unavailable_reason": "Cannot construct Custom variant..."
  }
}
```

**Problem**: Both construction guidance (root_example, enum_instructions, applicable_variants) AND unavailability reason present simultaneously - contradictory!

### After (Type-Safe - FIXED)
```json
{
  "path_info": {
    "mutability": "partially_mutable",
    "root_example_unavailable_reason": "Cannot construct Custom variant via BRP due to incomplete field data: tuple element 0 (CustomCursor): contains non-mutable descendants (see 'CustomCursor' mutation_paths for details). This variant's mutable fields can only be mutated if the entity is already set to this variant by your code."
  }
}
```

**Fixed**: Only unavailability reason present - clean, unambiguous message to users.

**JSON Format**: Same flat structure maintained. RootExampleInfo enum provides internal type safety during building, but extracts back to the same 4 separate optional fields for serialization.

## Success Criteria

1. ✅ `PartialRootExample` struct completely removed
2. ✅ `RootExampleInfo` enum used internally during building (NOT in serialized JSON)
3. ✅ No paths have both `root_example` and `root_example_unavailable_reason` simultaneously
4. ✅ Available variants include `root_example` + `enum_instructions` + `applicable_variants` (no unavailable_reason)
5. ✅ Unavailable variants include ONLY `root_example_unavailable_reason` (no root_example, enum_instructions, or applicable_variants)
6. ✅ Python filtering continues to work unchanged (checks `root_example_unavailable_reason` in flat structure)
7. ✅ All tests pass
8. ✅ Mutation test JSON generation produces deterministic output
9. ✅ Baseline comparison shows ONLY 2 types changed (bevy_window::cursor::CursorIcon and extras_plugin::TestMixedMutabilityEnum) - the bug fixes, not format changes

## Files Modified Summary

1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Add RootExampleInfo enum, update EnumPathData (remove applicable_variants field), PathInfo stays unchanged
2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/mod.rs` - Remove PartialRootExample export
3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` - Remove PartialRootExample struct, update building logic to create RootExampleInfo with empty enum_instructions placeholder and full signature group variants (not single variant), update nested chain propagation logic to extract unavailability from RootExampleInfo, remove code in process_signature_path() that populates enum_data.applicable_variants, update EnumPathData initialization in build_enum_root_path()
4. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` - Replace PartialRootExample import with RootExampleInfo, completely rewrite populate_root_examples_from_partials() to generate complete RootExampleInfo with instructions filled immediately
5. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` - Update resolve_enum_data_mut() IMPLEMENTATION to extract from RootExampleInfo back to 4 separate fields (signature and PathInfo construction stay unchanged to maintain JSON format compatibility)
6. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` - Replace PartialRootExample import with RootExampleInfo, update build_final_result() to construct RootExampleInfo::Available for non-enum types

## Notes

- This is NOT a breaking change to the JSON output format - the flat structure is maintained
- RootExampleInfo is INTERNAL-ONLY for type safety during building
- The JSON output format remains unchanged (4 separate optional fields in PathInfo)
- Only 2 types will show changes in baseline comparison (the bug fixes), not all enum types
- Internal type safety prevents impossible states at compile time
- Clean separation: internal representation (RootExampleInfo enum) vs serialized output (separate fields)

## Design Review Skip Notes

## TYPE-SYSTEM-1: Missing serde attributes for proper enum_instructions generation - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Step 1: Add RootExampleInfo Enum to types.rs
- **Issue**: The RootExampleInfo::Available variant's enum_instructions field will be serialized during partial root example building (Step 3), but the field contains an empty placeholder string. The plan states instructions are filled during finalization via with_instructions(), but the intermediate JSON serialization during building will expose the empty string in debug output or if accessed before finalization.
- **Reasoning**: The finding is incorrect because it misunderstands the serialization flow in the planned implementation. The plan's architecture ensures that RootExampleInfo with empty enum_instructions never gets serialized because: (1) Step 3 creates RootExampleInfo with enum_instructions: String::new() during building, (2) Step 4 stores it in EnumPathData.root_example_info, (3) Step 5a calls with_instructions() to fill the empty string BEFORE any serialization via resolve_enum_data_mut() at line 442, (4) Only after with_instructions() is the data serialized via into_mutation_path_external(). The intermediate state exists only in memory and never escapes to serialization. Adding #[serde(skip_serializing_if = "String::is_empty")] would be defensive programming against an impossible case.
- **Decision**: User agreed with rejection - plan stays unchanged
