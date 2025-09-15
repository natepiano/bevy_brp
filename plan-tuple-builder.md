# Plan: TupleMutationBuilder Migration to ProtocolEnforcer

## Migration Context

We are migrating the 7th of 8 builders from the legacy ExampleBuilder pattern to the new ProtocolEnforcer-based protocol. TupleMutationBuilder is part of Phase 5b: Incremental Builder Migration.

**Status**: 6 of 8 builders completed
- ✅ ValueMutationBuilder, MapMutationBuilder, SetMutationBuilder, ListMutationBuilder, ArrayMutationBuilder, StructMutationBuilder
- 🔄 **TupleMutationBuilder** (current)
- ⏸️ EnumMutationBuilder
- ⏸️ mod.rs default trait implementation

## Universal Migration Pattern

### Core Requirements for ALL Builders
1. **Implement ONLY**: `collect_children()` and `assemble_from_children()`
2. **Set**: `is_migrated()` to return `true`
3. **Keep**: `build_paths()` but make it return `Error::InvalidState`
4. **Optional**: Override `child_path_action()` if this type shouldn't expose child paths
5. **Remove**: All ExampleBuilder usage
6. **Update**: TypeKind to use trait dispatch: `self.builder().build_paths(ctx, builder_depth)`

### What ProtocolEnforcer Handles (DO NOT IMPLEMENT IN BUILDERS)
- ❌ ALL lines with `depth.exceeds_limit()`
- ❌ ALL `ctx.require_registry_schema_legacy() else` blocks creating NotMutable paths
- ❌ ENTIRE `build_not_mutable_path` method
- ❌ ALL `mutation_status` and `mutation_status_reason` field assignments
- ❌ ALL `NotMutableReason` imports and usage
- ❌ ALL direct `BRP_MUTATION_KNOWLEDGE` lookups
- ❌ **CRITICAL**: Do NOT add knowledge checks in individual builders!

### Error Handling Pattern
- Use `Error::InvalidState` for protocol violations (missing required children)
- Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
- Use `Error::NotMutable(reason)` when detecting non-mutable conditions - ProtocolEnforcer will handle path creation
- Update `assemble_from_children` to return `Result<Value>` not `Value`

### Method Signatures (Migrated Builders)
```rust
fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>>
fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<MutationPathDescriptor, Value>) -> Result<Value>
fn is_migrated(&self) -> bool { true }
```

## TupleMutationBuilder Specific Details

### Current Issues to Fix
- **Line 390**: ExampleBuilder usage in `build_schema_example()`
- **Line 285**: ExampleBuilder usage in static method `build_tuple_example_static()`
- **Line 317**: ExampleBuilder usage in static method `build_tuple_struct_example_static()`

### Special Handle Detection Logic
- **KEEP** the `is_handle_only_wrapper()` helper function
- **MOVE** the Handle detection check from `build_paths()` to `assemble_from_children()`
- When Handle wrapper detected, return `Err(Error::NotMutable(NotMutableReason::NonMutableHandle { container_type, element_type }))`
- ProtocolEnforcer will catch this error and create the NotMutable path

### Error Handling Pattern for Migrated Builders
Migrated builders follow a clean separation of concerns:
- **ProtocolEnforcer** handles all protocol validation (registry, depth, knowledge checks)
- **Individual builders** handle only data structure processing using `Result<T>` types
- **Schema errors** use `Error::SchemaProcessing` with structured error details
- **No direct error path creation** - builders return errors, ProtocolEnforcer creates paths

### Mutation Status Propagation Removal (CRITICAL)
- **Remove ENTIRE** `propagate_tuple_mixed_mutability()` function (lines ~341-383)
- **Remove call** to `Self::propagate_tuple_mixed_mutability(&mut paths)` in build_paths (line ~81)
- This complex logic for computing PartiallyMutable status is now handled by ProtocolEnforcer

### Code to Remove
- ❌ ALL recursion limit checks (`depth.exceeds_limit()`)
- ❌ ALL registry validation creating NotMutable paths
- ❌ ALL mutation status assignment logic
- ❌ The entire `propagate_tuple_mixed_mutability()` function and its call
- ❌ Static methods: `build_tuple_example_static()`, `build_tuple_struct_example_static()`
- ❌ Override of `build_schema_example()`

### Path Exposure
- **Note**: No need to override `child_path_action()` - Tuples expose indexed element paths like `[0].field`
- This is different from containers (Map, Set) which skip child paths due to BRP limitations

### TypeKind Update Required
```rust
// In type_kind.rs, change from:
Self::Tuple | Self::TupleStruct => TupleMutationBuilder.build_paths(ctx, builder_depth),
// To:
Self::Tuple | Self::TupleStruct => self.builder().build_paths(ctx, builder_depth),
```

## Implementation Plan

### Step 1: Add Required Import
```rust
use crate::error::Error;
```

### Step 2: Implement Protocol Methods
```rust
impl MutationPathBuilder for TupleMutationBuilder {
    fn is_migrated(&self) -> bool {
        true
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        // Use Result-returning API - ProtocolEnforcer handles missing schema
        let schema = ctx.require_registry_schema()?;

        let Some(prefix_items) = schema.get_field(SchemaField::PrefixItems) else {
            return Err(Error::SchemaProcessing {
                message: format!("Missing prefixItems in tuple schema for: {}", ctx.type_name()),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("extract_prefix_items".to_string()),
                details: None,
            }.into());
        };

        // Extract element types and create PathKind objects
        // Handle wrapper detection by returning Error::NotMutable for ProtocolEnforcer to process
    }

    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<MutationPathDescriptor, Value>) -> Result<Value> {
        // Check for Handle wrapper first - return Error::NotMutable if detected
        // Otherwise assemble tuple from child examples
    }
}
```

### Step 3: Update build_paths() to Error
```rust
fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
    Err(Error::InvalidState(
        "TupleMutationBuilder::build_paths() called directly - should use ProtocolEnforcer wrapper".to_string()
    ))
}
```

### Step 4: Remove Obsolete Code
- ❌ Delete `propagate_tuple_mixed_mutability()` function
- ❌ Delete static methods `build_tuple_example_static()` and `build_tuple_struct_example_static()`
- ❌ **Delete entire `build_schema_example()` method override** - use default trait implementation
- ❌ Delete all direct `build_not_mutable_path()` calls - ProtocolEnforcer handles these
- ❌ Delete all `NotMutableReason` handling in individual methods
- ❌ Delete registry validation creating NotMutable paths
- ❌ Remove ExampleBuilder import
- ❌ Remove all recursion/registry/mutation status logic

### Step 5: Update TypeKind Dispatch
In `type_kind.rs`:
```rust
Self::Tuple | Self::TupleStruct => self.builder().build_paths(ctx, builder_depth),
```

### Step 6: Validation
- Run `cargo build` to check for compilation issues
- **STOP** and ask user to validate and discuss
- **CODE REVIEW**: After validation, stop and ask user to review the TupleMutationBuilder implementation

## Reference Implementations
Study these for exact patterns:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`

## Critical Success Criteria
1. ✅ Only implements `collect_children()` and `assemble_from_children()`
2. ✅ No direct knowledge, depth, or registry checks
3. ✅ Returns errors instead of creating NotMutable paths
4. ✅ Preserves Handle wrapper detection logic in `assemble_from_children()`
5. ✅ Compiles successfully after TypeKind dispatch update
6. ✅ All static methods and complex mutation status logic removed