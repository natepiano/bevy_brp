# Plan: Unified Type System and Duplication Removal

## Overview
The current type schema system has significant code duplication across spawn format generation, mutation path generation, and multiple mutation builder implementations. This plan outlines a comprehensive refactoring to eliminate duplication and create a single, unified type processing system.

## Current State

### Problems
1. **Spawn/Mutation Duplication**: `build_spawn_format()` and `build_mutation_paths()` have separate implementations for building type examples
2. **Multiple Mutation Builders**: Each type has its own builder with similar patterns but different implementations:
   - `StructMutationBuilder` 
   - `EnumMutationBuilder`
   - `TupleMutationBuilder`
   - `ArrayMutationBuilder` 
   - `ListMutationBuilder`
   - `ValueMutationBuilder`
3. **Repeated Recursion Logic**: Every builder implements the same recursion pattern (depth checks, nested type extraction, recursive calls)
4. **Missing Enum Support**: Enums don't get spawn formats even when they're spawnable components
5. **Inconsistent Examples**: Spawn format examples may differ from root mutation path examples for the same type

### Current Code Structure
- `TypeInfo::build_spawn_format()` - Orchestrator that dispatches to specific builders
- `TypeInfo::build_struct_spawn_format()` - Builds spawn format for structs
- `TypeInfo::build_tuple_spawn_format()` - Builds spawn format for tuples/tuple structs
- No enum spawn format builder exists
- Mutation path builders generate field/element paths but not root replacement paths for structs

## Proposed Unified Solution

### Key Insights
1. **Spawn format = Root mutation path**: The empty path `""` example should be identical to spawn format
2. **All builders follow same pattern**: Every type builder does the same thing - build own paths, extract nested types, recurse
3. **Single recursion logic**: The depth checks, nested extraction, and recursive calls can be abstracted once

### Unified Implementation Strategy

#### Phase 1: Abstract Common Recursion Pattern
Create a shared recursion framework that all builders can use:

```rust
pub trait TypeHandler {
    /// Build this type's immediate paths (no recursion)
    fn build_own_paths(&self, ctx: &MutationPathContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>>;
    
    /// Extract nested types for recursion (fields, variants, elements)
    fn extract_nested_types(&self, ctx: &MutationPathContext) 
        -> Vec<NestedTypeInfo>;
}

#[derive(Debug, Clone)]
pub struct NestedTypeInfo {
    pub path_prefix: String,    // ".field_name", ".0", etc.
    pub type_name: BrpTypeName,
    pub context_data: NestedContext, // Field info, variant info, etc.
}

// Default implementation of the universal recursion pattern
impl<T: TypeHandler> MutationPathBuilder for T {
    fn build_paths(&self, ctx: &MutationPathContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        
        // Universal safety check
        if depth.exceeds_limit() {
            return Ok(vec![Self::build_depth_exceeded_path(ctx)]);
        }
        
        // Step 1: Build own immediate paths
        let mut paths = self.build_own_paths(ctx, depth)?;
        
        // Step 2: Extract and recurse into nested types
        let nested_types = self.extract_nested_types(ctx);
        for nested in nested_types {
            let nested_schema = ctx.get_type_schema(&nested.type_name)?;
            let nested_kind = TypeKind::from_schema(nested_schema, &nested.type_name);
            let nested_ctx = nested.create_context(ctx);
            
            // Recurse with incremented depth
            let nested_paths = nested_kind.build_paths(&nested_ctx, depth.increment())?;
            paths.extend(nested_paths);
        }
        
        Ok(paths)
    }
}
```

#### Phase 2: Simplify Individual Builders
Convert all builders to the unified pattern:

```rust
// New simplified StructMutationBuilder
pub struct StructHandler;

impl TypeHandler for StructHandler {
    fn build_own_paths(&self, ctx: &MutationPathContext, _depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        // Generate root struct path + individual field paths
        // No recursion logic here - just immediate struct concerns
        Ok(vec![
            self.build_root_struct_path(ctx)?,
            // Individual field paths are built by recursion framework
        ])
    }
    
    fn extract_nested_types(&self, ctx: &MutationPathContext) -> Vec<NestedTypeInfo> {
        let properties = Self::extract_properties(ctx);
        properties.into_iter().map(|(field_name, field_type)| {
            NestedTypeInfo {
                path_prefix: format!(".{field_name}"),
                type_name: field_type,
                context_data: NestedContext::StructField { field_name, parent_type: ctx.type_name() }
            }
        }).collect()
    }
}

// New simplified EnumHandler  
pub struct EnumHandler;

impl TypeHandler for EnumHandler {
    fn build_own_paths(&self, ctx: &MutationPathContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        // Generate root enum path only
        // Variant nested paths handled by recursion framework
        Ok(vec![self.build_root_enum_path(ctx, depth)?])
    }
    
    fn extract_nested_types(&self, ctx: &MutationPathContext) -> Vec<NestedTypeInfo> {
        let variants = Self::extract_enum_variants(ctx.schema);
        let mut nested = Vec::new();
        
        for variant in variants {
            match variant {
                EnumVariantInfo::Unit(_) => {}, // No nested types
                EnumVariantInfo::Tuple(name, types) => {
                    for (index, type_name) in types.iter().enumerate() {
                        nested.push(NestedTypeInfo {
                            path_prefix: format!(".{index}"),
                            type_name: type_name.clone(),
                            context_data: NestedContext::EnumTuple { variant: name, index }
                        });
                    }
                },
                EnumVariantInfo::Struct(name, fields) => {
                    for field in fields {
                        nested.push(NestedTypeInfo {
                            path_prefix: format!(".{}", field.field_name),
                            type_name: field.type_name,
                            context_data: NestedContext::EnumStruct { variant: name, field_name: field.field_name }
                        });
                    }
                }
            }
        }
        
        nested
    }
}
```

#### Phase 3: Unify Spawn Format Generation
Replace spawn format builders with mutation path extraction:

```rust
impl TypeInfo {
    pub fn build_spawn_format(&self) -> Option<Value> {
        // Build complete mutation paths
        let mutation_paths = self.build_mutation_paths()?;
        
        // Extract root path (empty string) as spawn format
        mutation_paths
            .iter()
            .find(|path| path.path.is_empty())
            .map(|path| path.example.clone())
    }
}

// Delete all the old spawn format builders:
// - build_struct_spawn_format() 
// - build_tuple_spawn_format()
// - build_enum_spawn_format() (doesn't exist anyway)
```

#### Phase 4: Consolidate Type Dispatch
Simplify the TypeKind enum and dispatch logic:

```rust
pub enum TypeKind {
    Struct(StructHandler),
    Enum(EnumHandler), 
    Tuple(TupleHandler),
    Array(ArrayHandler),
    List(ListHandler),
    Value(ValueHandler),
}

// All implement TypeHandler, so dispatch is uniform
impl TypeKind {
    pub fn build_paths(&self, ctx: &MutationPathContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        match self {
            TypeKind::Struct(handler) => handler.build_paths(ctx, depth),
            TypeKind::Enum(handler) => handler.build_paths(ctx, depth),
            // ... all use the same interface
        }
    }
}
```

## Benefits

1. **Massive Code Reduction**: Eliminate 6+ separate mutation builders, replacing with unified TypeHandler pattern
2. **Single Source of Truth**: One code path for all type processing (spawn, mutation, examples)  
3. **Automatic Enum Support**: Enums get spawn formats "for free" from their root mutation paths
4. **Perfect Consistency**: Spawn format always matches root mutation example across all types
5. **Easier Maintenance**: Changes to type processing affect everything uniformly
6. **Simpler Architecture**: One recursion pattern, one dispatch mechanism, one set of type handlers
7. **Better Testing**: Test the TypeHandler trait once, all types inherit the behavior

## Implementation Phases

### Immediate Phase: Minimal Enum Fix (Current Work)
**Goal**: Fix enum recursion bug without major refactoring
- Add `extract_variant_inner_types` to `EnumMutationBuilder`
- Update `EnumMutationBuilder::build_paths` with recursion like `StructMutationBuilder`  
- Keep existing architecture intact

### Future Phase: Complete Unification (Major Refactoring)
**Goal**: Eliminate all duplication through TypeHandler abstraction
- Abstract common recursion pattern into `TypeHandler` trait
- Convert all builders to simplified handlers
- Unify spawn format generation  
- Delete duplicate code

**Dependencies**: Enum recursion fix must be completed and stable first

## Testing Strategy

1. **Compatibility Tests**: Ensure spawn formats remain backward compatible
2. **Enum Tests**: Verify enum components get proper spawn formats
3. **Example Consistency**: Test that root mutation path examples match spawn formats
4. **Coverage**: Test all type kinds (struct, tuple, enum, array, etc.)

### A/B Comparison Testing

Since this change affects the MCP tool's behavior, we need to verify compatibility through real-world testing:

1. **Two-Session Comparison**:
   - Session A: Install and run current MCP tool version (before changes)
   - Session B: Install and run new MCP tool version (after changes)
   - Run identical agentic tests in both sessions
   - Compare outputs to ensure:
     - All existing spawn formats remain identical
     - New enum spawn formats appear only in Session B
     - No regressions in other type kinds

2. **Test Coverage Requirements**:
   - **Struct spawn formats**: Verify complex structs like `Transform`, `Sprite`
   - **Tuple struct spawn formats**: Test single-field (unwrapped) and multi-field variants
   - **Tuple spawn formats**: Ensure proper array generation
   - **Enum spawn formats**: NEW - verify all variant types (unit, tuple, struct)
   - **Nested types**: Test spawn formats with nested structs, Options, arrays

3. **Agentic Test Execution**:
   ```bash
   # Session A (baseline)
   cargo install --path mcp  # Install current version
   # Exit and restart Claude to pick up current tool
   # Run: .claude/commands/test.md with spawn format tests
   
   # Session B (new implementation)  
   cargo install --path mcp  # Install new version
   # Exit and restart Claude to pick up new tool
   # Run: .claude/commands/test.md with spawn format tests
   
   # Compare outputs to verify backward compatibility
   ```

4. **Specific Test Cases**:
   - `extras_plugin::TestEnumWithSerDe` - Should gain spawn format in new version
   - `bevy_transform::components::transform::Transform` - Should remain identical
   - `extras_plugin::TestTupleStruct` - Should remain unwrapped for single field
   - `extras_plugin::TestComplexComponent` - Should handle nested types correctly

## Migration Path

1. Implement alongside existing code initially
2. Add feature flag or configuration to switch between old/new implementations
3. Migrate tests incrementally
4. Remove old implementation once new one is proven stable

## Open Questions

1. Should we preserve any spawn-format-specific logic for special cases?
2. How do we handle types that support spawn but not mutation (if any exist)?
3. Should the root mutation path be generated for types that can't be spawned as components?

## Array Mutation Path Investigation

**TODO**: Determine the purpose and usage of `MutationPathKind::ArrayElement` by creating test scenarios:

1. **Create test component** with array field (e.g., `positions: [Vec3; 4]`)
2. **Register and spawn** the test component in a Bevy app  
3. **Test mutation paths** to verify if arrays generate `[index]` style paths or recurse to element types
4. **Document behavior** - does `[Vec3; 4]` produce `[0].x`, `[1].y` paths or just Vec3 component paths?

**Current Status**: `ArrayMutationBuilder` recurses directly to element types, bypassing array indexing. The `ArrayElement` variant may be unused legacy code or intended for future functionality. Testing needed to clarify expected behavior and determine if array indexing mutations are supported.

## Success Criteria

- [ ] All existing spawn formats continue to work
- [ ] Enum components get spawn formats
- [ ] Code duplication is eliminated
- [ ] Root mutation path examples match spawn formats exactly
- [ ] Test coverage for all type kinds