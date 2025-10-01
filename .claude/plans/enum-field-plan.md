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

1. Identify all locations that create or modify `enum_variant_path`
2. Update to use `enum_data` instead:

```rust
// OLD:
path.enum_variant_path = variant_path_vec;
path.enum_instructions = Some(instructions);

// NEW:
path.enum_data = Some(EnumPathData::new(variant_chain, instructions));
```

3. Map existing `VariantPath` data to new `EnumPathData` fields:
   - Move the entire `Vec<VariantPath>` directly into `variant_chain` (preserves all metadata)
   - Initialize `variant_chain_root_example` as None (will be populated by dependent plan)
   - For `applicable_variants`: Note that this field is currently populated in `ExampleGroup` structure (see enum_path_builder.rs:429-432). During this refactoring, preserve the existing population logic by migrating it to work with the new `EnumPathData` structure. This is NOT future work - the functionality exists and must be preserved during migration

### Phase 4: Update All Access Patterns

**Files**: All code that reads `enum_variant_path`

1. Search for all usages: `rg "enum_variant_path"`
2. Update each access pattern:

```rust
// OLD:
if !path.enum_variant_path.is_empty() {
    // ...
}

// NEW:
if let Some(enum_data) = &path.enum_data {
    if !enum_data.is_empty() {
        // ...
    }
}
```

3. Common patterns to update:
   - Checking if path has enum variants
   - Iterating over variant chain
   - Building output structures

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
