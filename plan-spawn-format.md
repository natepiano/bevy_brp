# Plan: Unify Example Generation Through Path Builders

## Goal
**Eliminate redundant example generation systems by making path builders the single source of truth for all JSON example generation, including spawn formats.**

## Current Problem
We have three separate systems generating the same JSON examples:
1. Path builders generate examples for each mutation path
2. `TypeInfo::build_type_example()` independently generates examples
3. `TypeInfo::build_spawn_format()` has yet another example generation system

This causes:
- Code duplication and maintenance burden
- Potential inconsistencies between examples
- Confusion about which system to use when
- Double recursion depth tracking issues

## Proposed Solution
Make path builders generate everything in a single traversal:
- The root path builds the complete spawn format
- Nested paths build their specific mutation examples
- Eliminate all other example generation code

## Architecture Changes

### Core Concept
```rust
// Path builders will generate ALL examples during traversal
// Returns: (spawn_format, mutation_paths)
fn build_paths(&self, ctx) -> Result<(Value, Vec<MutationPathInternal>)>
```

### Files That Will Change

#### `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/mod.rs`
Change the `MutationPathBuilder` trait:
```rust
// OLD
pub trait MutationPathBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>>;
}

// NEW
pub trait MutationPathBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<MutationPathOutput>;
}

pub struct MutationPathOutput {
    pub spawn_format: Value,  // The complete root example
    pub mutation_paths: Vec<MutationPathInternal>,  // Field-level paths only
}
```

#### All Builder Files
Each builder in `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/`:
- `array_builder.rs`
- `default_builder.rs`
- `enum_builder.rs`
- `list_builder.rs`
- `map_builder.rs`
- `set_builder.rs`
- `struct_builder.rs`
- `tuple_builder.rs`

Will change from:
```rust
impl MutationPathBuilder for SomeBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        // Build paths, calling TypeInfo::build_type_example for examples
    }
}
```

To:
```rust
impl MutationPathBuilder for SomeBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<MutationPathOutput> {
        // Build spawn format first (complete example)
        let spawn_format = self.build_complete_example(ctx, depth);
        
        // Build mutation paths (without root)
        let mutation_paths = self.build_mutation_paths(ctx, depth, &spawn_format);
        
        Ok(MutationPathOutput { spawn_format, mutation_paths })
    }
}
```

#### `mcp/src/brp_tools/brp_type_schema/type_info.rs`
Update to use the new path builder output:
```rust
impl TypeInfo {
    pub fn from_schema(brp_type_name: BrpTypeName, type_schema: &Value, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        // ...
        
        // OLD: Separate calls for paths and spawn format
        let mutation_paths_vec = Self::build_mutation_paths(...);
        let spawn_format = Self::build_spawn_format(...);
        
        // NEW: Single call gets both
        let output = Self::build_mutation_paths(...);  // Returns MutationPathOutput
        let spawn_format = Some(output.spawn_format);
        let mutation_paths = Self::convert_mutation_paths(&output.mutation_paths, &registry);
        
        // ...
    }
}
```

## Logic Migration - Moving Example Building to Builders

### Migrate TypeInfo Logic to Individual Builders

The key insight is that `TypeInfo::build_type_example` contains the core example-building logic that must be preserved but moved into individual builders to eliminate double recursion.

#### From `mcp/src/brp_tools/brp_type_schema/type_info.rs`:
The logic currently in `build_type_example`'s match statement should be moved:
- **Enum logic** → `EnumMutationBuilder` 
- **Struct logic** → `StructMutationBuilder`
- **Array logic** → `ArrayMutationBuilder` 
- **Tuple logic** → `TupleMutationBuilder`
- **Value logic** → `DefaultMutationBuilder`
- etc.

Each builder's `build_paths` method will:
1. **Build its own level's example** using migrated logic (no recursion)
2. **Recurse for child paths** (which build their own examples)  
3. **Assemble complete example** from child results bottom-up
4. **Return paths with examples** in single traversal

### Complete Function Removals

#### From `mcp/src/brp_tools/brp_type_schema/type_info.rs`:
```rust
// REMOVE AFTER LOGIC MIGRATION - No longer needed
pub fn build_type_example(
    type_name: &BrpTypeName,
    registry: &HashMap<BrpTypeName, Value>,
    depth: RecursionDepth,
) -> Value { ... }

// REMOVE ENTIRELY - No longer needed  
pub fn build_example_value_for_type(
    type_name: &BrpTypeName,
    registry: &HashMap<BrpTypeName, Value>,
) -> Value { ... }

// REMOVE ENTIRELY - Path builders handle this now
fn build_spawn_format(
    type_schema: &Value,
    registry: Arc<HashMap<BrpTypeName, Value>>,
    type_kind: &TypeKind,
    type_name: &BrpTypeName,
) -> Option<Value> { ... }

// REMOVE ENTIRELY - Path builders handle this
fn build_struct_spawn_format(...) -> Option<Value> { ... }

// REMOVE ENTIRELY - Path builders handle this
fn build_tuple_spawn_format(...) -> Option<Value> { ... }
```

### Function Call Removals

#### From all builder files:
Remove ALL calls to `TypeInfo::build_type_example()`:
```rust
// Examples of lines to remove/replace:
TypeInfo::build_type_example(ctx.type_name(), &ctx.registry, depth)
TypeInfo::build_type_example(&element_type, registry, RecursionDepth::ZERO)
TypeInfo::build_type_example(&field_type, &ctx.registry, RecursionDepth::ZERO)
```

These will be replaced with internal example building within each builder.

### Utility Function Migrations

#### From `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/struct_builder.rs`:
```rust
// This utility function stays but moves to be private within StructMutationBuilder
pub fn build_struct_example_from_properties(...) -> Value { ... }
```

#### From `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/builders/enum_builder.rs`:
```rust
// This stays but becomes the core of enum spawn format generation
pub fn build_enum_example(...) -> Value { ... }
```

## Benefits

1. **Single source of truth**: Path builders own all example generation
2. **Consistent examples**: One traversal generates everything
3. **Clear separation**: Spawn format in `spawn_format` field, mutations in `mutation_paths`
4. **No double depth tracking**: One recursion system, one depth counter
5. **Simpler mental model**: "Path builders generate all examples"

## Migration Strategy

**Atomic change required**: All components must be updated simultaneously in a single commit:

1. Update `MutationPathBuilder` trait definition to return `MutationPathOutput`
2. Update all 8 builder implementations simultaneously:
   - `array_builder.rs`
   - `default_builder.rs`
   - `enum_builder.rs`
   - `list_builder.rs`
   - `map_builder.rs`
   - `set_builder.rs`
   - `struct_builder.rs`
   - `tuple_builder.rs`
3. Update the single caller in `TypeInfo::from_schema()` to handle the new output
4. Remove all `TypeInfo` example generation functions that are no longer needed
5. Clean up any remaining references

## Testing Strategy

1. Ensure existing tests pass with new architecture
2. Verify spawn formats match current output
3. Verify mutation paths remain unchanged
4. Add tests for consistency between spawn format and mutation examples