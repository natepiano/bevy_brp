# Plan: Encapsulate Path Building Logic

## Goal
Simplify mutation path building by encapsulating path construction logic in `RecursionContext`, eliminating redundant work across all builders. Currently every builder manually constructs paths with `format!(".{field_name}")` and duplicates `PathKind` logic.

## Current Problems
1. **Redundant path construction**: Every builder does `format!(".{field_name}")` 
2. **Duplicated `PathKind` logic**: Each builder manually creates `PathKind::StructField` variants
3. **Confusing naming**: `mutation_path` field stores field name, not actual mutation path
4. **Scattered logic**: Path building concerns spread across 8+ builder files

## Proposed Solution
Centralize path building in `RecursionContext` with a helper method that handles all path construction and `PathKind` selection. Builders focus only on determining examples and mutation status.

## Changes Required

### 1. Update `PathLocation::Element` structure
**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`
- Change `mutation_path` field to store actual full paths (e.g., ".translation.x")
- Update `create_field_context()` to pass `new_path_prefix` instead of extracted field name
- This makes the naming accurate and eliminates path prefix tracking

### 2. Add `build_mutation_path()` helper method
**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`
```rust
impl RecursionContext {
    /// Build a complete MutationPathInternal for the current context
    pub fn build_mutation_path(
        &self,
        example: Value,
        mutation_status: MutationStatus,
        error_reason: Option<String>,
    ) -> MutationPathInternal {
        // Handle path construction based on location
        // Select appropriate PathKind variant
        // Use parent knowledge if available for examples
    }
}
```

### 3. Update all builders to use helper method
**Files to update** (8 builders):
- `builders/array_builder.rs`
- `builders/default_builder.rs` 
- `builders/enum_builder.rs`
- `builders/list_builder.rs`
- `builders/map_builder.rs`
- `builders/set_builder.rs`
- `builders/struct_builder.rs`
- `builders/tuple_builder.rs`

**Changes per builder**:
- Replace manual `MutationPathInternal` construction with `ctx.build_mutation_path()`
- Remove `format!(".{field_name}")` path construction
- Remove `PathKind::StructField` creation
- Focus only on example value generation and mutation status determination

### 4. Update `type_kind.rs` error path construction
**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/type_kind.rs`
- Update `build_treat_as_value_path()` and `build_not_mutatable_path_from_support()` 
- Use new helper or adapt to new `mutation_path` structure

## Implementation Strategy

### Phase 1: Foundation
1. Add `build_mutation_path()` helper to `RecursionContext` (without changing existing structure)
2. Test helper works with current `mutation_path` field (field name only)

### Phase 2: Transition builders
1. Update one builder at a time to use the helper
2. Test each builder change individually
3. Ensure all `PathKind` variants are handled correctly

### Phase 3: Complete transition
1. Update `create_field_context()` to store full paths in `mutation_path`
2. Update helper method to use full paths directly
3. Remove any remaining manual path construction

## Expected Benefits
- **Simplified builders**: Focus only on business logic (examples, mutation status)
- **Centralized path logic**: Single place to maintain path construction rules
- **Accurate naming**: `mutation_path` actually contains mutation paths
- **Reduced duplication**: Eliminate repeated `format!()` and `PathKind` code
- **Easier maintenance**: Path construction changes only need updates in one place

## Risk Mitigation
- Implement in phases to test each change
- Keep existing tests passing at each step
- Verify all `PathKind` variants (`RootValue`, `StructField`, `IndexedElement`) work correctly
- Ensure knowledge system integration remains functional