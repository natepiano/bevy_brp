# Plan: Consolidate Enum Fields into EnumPathData Structure

**Status**: PREREQUISITE for plan-mutation-path-root-example.md

## Executive Summary

Refactor `MutationPathInternal` to consolidate all enum-related fields into a dedicated `EnumPathData` struct. This improves code organization, makes enum-specific data explicit, and prepares the codebase for enhanced enum variant chain root example tracking.

## Motivation

Currently, `MutationPathInternal` has enum-related fields scattered throughout its structure (e.g., `enum_variant_path: Vec<VariantPath>`). This makes it difficult to:
- Identify which fields are enum-specific
- Add new enum-related metadata
- Pass enum data as a cohesive unit
- Maintain clear separation between enum and non-enum path data

Consolidating into `EnumPathData` provides:
- Clear organizational structure
- Single point of access for enum-specific data
- Foundation for variant chain root example tracking
- Better type safety and API clarity

## Current State

`MutationPathInternal` currently has:
```rust
pub struct MutationPathInternal {
    // ... other fields ...
    pub enum_variant_path: Vec<VariantPath>,
    // possibly other enum-related fields
}
```

## Target State

```rust
pub struct MutationPathInternal {
    // ... other fields ...
    pub enum_data: Option<EnumPathData>,
}

pub struct EnumPathData {
    pub variant_chain: Vec<VariantPath>,
    pub applicable_variants: Vec<VariantName>,
    pub variant_chain_root_example: Option<Value>,
    pub enum_instructions: String,
}
```

## Complete Site Analysis

Before implementation, here are ALL the sites that touch `enum_instructions` and `enum_variant_path`:

**Field Definitions (Phase 2):**
- types.rs:198 - `enum_instructions: Option<String>` in MutationPathInternal → REMOVE
- types.rs:200 - `enum_variant_path: Vec<VariantPath>` in MutationPathInternal → REMOVE
- types.rs:247 - `enum_instructions: Option<String>` in PathInfo (output struct) → KEEP (no change)
- types.rs:250 - `enum_variant_path: Vec<VariantPath>` in PathInfo (output struct) → KEEP (no change)

**Creation Sites (Phase 3):**
- builder.rs:290-309 - `build_mutation_path_internal()` function
- enum_path_builder.rs:653-668 - `create_result_paths()` function

**Read/Access Sites (Phase 4):**
- enum_path_builder.rs:617-641 - `update_child_variant_paths()` function

**Serialization Sites (Phase 5):**
- types.rs:343-344 - `MutationPath::from_mutation_path_internal()` function

**String Literals (informational only - no code change needed):**
- enum_path_builder.rs:469 - error message text contains "enum_variant_path" string

Total sites requiring code changes: **6 locations**

## Implementation Plan

### Phase 1: Create EnumPathData Structure

**File**: `mutation_path.rs` (or appropriate module)

1. Define `EnumPathData` struct with initial fields:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EnumPathData {
    /// Chain of enum variants from root to this path with full metadata
    pub variant_chain: Vec<VariantPath>,

    /// All variants that share the same signature and support this path
    pub applicable_variants: Vec<VariantName>,

    /// Complete root example for this specific variant chain
    pub variant_chain_root_example: Option<Value>,

    /// Human-readable instructions for using this enum path
    pub enum_instructions: String,
}
```

2. Add constructor and helper methods:
```rust
impl EnumPathData {
    pub fn new(variant_chain: Vec<VariantPath>, enum_instructions: String) -> Self {
        Self {
            variant_chain,
            applicable_variants: Vec::new(),
            variant_chain_root_example: None,
            enum_instructions,
        }
    }

    pub fn with_applicable_variants(mut self, variants: Vec<VariantName>) -> Self {
        self.applicable_variants = variants;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.variant_chain.is_empty()
    }
}
```

### Phase 2: Update MutationPathInternal

**File**: `mutation_path.rs`

1. Replace `enum_variant_path` field with `enum_data`:
```rust
pub struct MutationPathInternal {
    // ... existing fields ...

    // OLD: pub enum_variant_path: Vec<VariantPath>,
    // NEW:
    pub enum_data: Option<EnumPathData>,
}
```

2. Update constructor to initialize `enum_data`:
```rust
impl MutationPathInternal {
    pub fn new(/* ... */) -> Self {
        Self {
            // ... other fields ...
            enum_data: None,
        }
    }
}
```

### Phase 3: Migrate Data from Old to New Structure

**Files**: All builders that currently populate `enum_variant_path`

1. **Two main creation sites to update:**

   a. **builder.rs:288-311** - `build_mutation_path_internal()` function:
   ```rust
   // OLD (lines 290-309):
   let (enum_instructions, enum_variant_path) = if ctx.variant_chain.is_empty() {
       (None, vec![])
   } else {
       (
           enum_path_builder::generate_enum_instructions(ctx),
           ctx.variant_chain.clone(),
       )
   };
   MutationPathInternal {
       // ... other fields ...
       enum_instructions,
       enum_variant_path,
   }

   // NEW:
   let enum_data = if ctx.variant_chain.is_empty() {
       None
   } else {
       Some(EnumPathData {
           variant_chain: ctx.variant_chain.clone(),
           applicable_variants: Vec::new(),  // Migrated from ExampleGroup
           variant_chain_root_example: None,
           enum_instructions: enum_path_builder::generate_enum_instructions(ctx)
               .expect("generate_enum_instructions should return Some when variant_chain is non-empty"),
       })
   };
   MutationPathInternal {
       // ... other fields ...
       enum_data,
   }
   ```

   b. **enum_path_builder.rs:646-669** - `create_result_paths()` function:
   ```rust
   // OLD (lines 653-668):
   let enum_instructions = generate_enum_instructions(ctx);
   let enum_variant_path = populate_variant_path(ctx, &enum_examples, &default_example);
   let root_mutation_path = MutationPathInternal {
       // ... other fields ...
       enum_instructions,
       enum_variant_path,
   };

   // NEW:
   let enum_data = Some(EnumPathData {
       variant_chain: populate_variant_path(ctx, &enum_examples, &default_example),
       applicable_variants: Vec::new(),  // Migrated from ExampleGroup
       variant_chain_root_example: None,
       enum_instructions: generate_enum_instructions(ctx)
           .expect("generate_enum_instructions should return Some for enum paths"),
   });
   let root_mutation_path = MutationPathInternal {
       // ... other fields ...
       enum_data,
   };
   ```

2. **Search for any other creation sites:**
   - Run: `rg "enum_variant_path.*=" mcp/src/brp_tools/brp_type_guide`
   - Update any additional sites following the pattern above

3. Map existing `VariantPath` data to new `EnumPathData` fields:
   - Move the entire `Vec<VariantPath>` directly into `variant_chain` (preserves all metadata)
   - Initialize `variant_chain_root_example` as None (will be populated by dependent plan)
   - For `applicable_variants`: Note that this field is currently populated in `ExampleGroup` structure (see enum_path_builder.rs:429-432). During this refactoring, preserve the existing population logic by migrating it to work with the new `EnumPathData` structure. This is NOT future work - the functionality exists and must be preserved during migration

### Phase 4: Update All Access Patterns

**Files**: All code that reads `enum_variant_path`

1. **Main read site to update:**

   **enum_path_builder.rs:617-641** - `update_child_variant_paths()` function:
   ```rust
   // OLD (lines 617-641):
   if !child.enum_variant_path.is_empty() {
       // Check for matching entries
       for entry in &mut child.enum_variant_path {
           if entry.full_mutation_path == *current_path {
               entry.instructions = format!(/* ... */);
               entry.variant_example = examples.iter().find(/* ... */);
           }
       }
   }

   // NEW:
   if let Some(enum_data) = &mut child.enum_data {
       if !enum_data.is_empty() {
           // Check for matching entries
           for entry in &mut enum_data.variant_chain {
               if entry.full_mutation_path == *current_path {
                   entry.instructions = format!(/* ... */);
                   entry.variant_example = examples.iter().find(/* ... */);
               }
           }
       }
   }
   ```

2. **Search for any other read sites:**
   - Run: `rg "\.enum_variant_path" mcp/src/brp_tools/brp_type_guide`
   - Update any additional access patterns following the examples above

3. Common patterns to handle:
   - Checking if path has enum variants: `path.enum_data.is_some()`
   - Checking if non-empty: `path.enum_data.as_ref().map_or(false, |ed| !ed.is_empty())`
   - Iterating over variant chain: `enum_data.variant_chain.iter()`
   - Mutable iteration: `enum_data.variant_chain.iter_mut()`

### Phase 5: Update Serialization/Output

**File**: `types.rs`
**Function**: `MutationPath::from_mutation_path_internal()` (currently lines 297-349)

1. Update the conversion code that creates `PathInfo` struct (currently lines 343-344):

```rust
// OLD (lines 343-344):
enum_instructions: path.enum_instructions.clone(),
enum_variant_path: path.enum_variant_path.clone(),

// NEW: Extract from EnumPathData
enum_instructions: path.enum_data.as_ref().map(|ed| ed.enum_instructions.clone()),
enum_variant_path: path.enum_data.as_ref().map(|ed| ed.variant_chain.clone()).unwrap_or_default(),
```

2. Note: The `PathInfo` struct fields remain the same (this is internal-to-output conversion, not changing the output format)

3. Verify that all enum data is preserved in the conversion:
   - `enum_instructions` → PathInfo field
   - `variant_chain` → `enum_variant_path` field in PathInfo
   - `applicable_variants` → currently not in output format (may be added by dependent plan)
   - `variant_chain_root_example` → currently not in output format (will be added by dependent plan)

### Phase 6: Remove Old Field

**File**: `mutation_path.rs`

1. Remove `enum_variant_path` field completely
2. Remove any helper methods that only existed for old field
3. Ensure all tests pass

### Phase 7: Testing and Validation

1. **Unit Tests**:
   - Test `EnumPathData` construction
   - Test helper methods
   - Test serialization/deserialization if applicable

2. **Integration Tests**:
   - Verify enum paths still generate correctly
   - Check output format matches expectations
   - Validate with `TestVariantChainEnum` example

3. **Regression Tests**:
   - Run full test suite
   - Verify no functionality changes
   - Check performance remains acceptable

## Migration Checklist

- [ ] Create `EnumPathData` struct with all fields
- [ ] Add `enum_data: Option<EnumPathData>` to `MutationPathInternal`
- [ ] Update all builders to populate new field
- [ ] Update all code reading old field
- [ ] Update serialization/output code
- [ ] Remove `enum_variant_path` field
- [ ] Run all tests
- [ ] Update documentation

## Field Descriptions

### variant_chain
The complete chain of `VariantPath` entries from the root type down to the current path. Each entry contains the variant name, full mutation path, instructions, and example value. For example, for path `.middle_struct.nested_enum.name` traversing `TestVariantChainEnum::WithMiddleStruct` → `BottomEnum::VariantB`, this would contain two `VariantPath` entries preserving all metadata needed for mutation guidance and parent enum processing.

### applicable_variants
All enum variants that share the exact same signature (field names and types) and therefore support this mutation path. Used to inform AI agents which variants work with a given path.

### variant_chain_root_example
The complete root-level example that correctly demonstrates this specific variant chain. This will be populated by the root example fix plan (plan-mutation-path-root-example.md).

### enum_instructions
Human-readable text explaining how to use this enum path, which variants to select, etc. Always populated when `EnumPathData` exists, generated based on the variant chain by `generate_enum_instructions()`. Note: This is a required `String` field (not `Option<String>`) because the optionality is handled at the `Option<EnumPathData>` level - if you have enum data, you always have instructions.

## Success Criteria

1. All enum-related data consolidated into `EnumPathData`
2. No references to old `enum_variant_path` field remain
3. All tests pass
4. Output format preserves enum information correctly
5. Code is cleaner and more maintainable
6. Ready for variant chain root example enhancement

## Dependencies

None - this is a prerequisite refactoring.

## Dependents

- `plan-mutation-path-root-example.md` requires this refactoring to be complete before implementation

## Notes

- This is a pure refactoring with no functionality changes
- Focus on maintaining exact same behavior during migration
- The new structure will be enhanced by future plans
- Consider adding more fields to `EnumPathData` as needs arise
