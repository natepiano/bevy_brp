# Type Schema Module Structure Plan

## Current State Analysis

### File Size Issues

The `mutation_path_builders.rs` file is currently **1,510 lines**, making it difficult to navigate and maintain. Key issues:

- **StructMutationBuilder alone is 724 lines** (nearly half the file!)
- Mixed concerns across core traits, context structures, and multiple builder implementations
- Helper functions scattered throughout rather than organized

### Current Logical Groupings

1. **Helper Functions & Types** (lines 1-91)
   - `HardcodedPathsResult` enum
   - Type extraction helpers
   - `RootOrField` context enum

2. **MutationPathContext** (lines 92-347)
   - Registry access
   - Type extraction methods (list, map, option, tuple)
   - Mutation support validation
   - Wrapper handling

3. **Core Trait & Dispatch** (lines 349-462)
   - `MutationPathBuilder` trait
   - `TypeKind` implementation with type-directed dispatch

4. **Individual Type Builders** (lines 464-1509)
   - `ArrayMutationBuilder` (~106 lines)
   - `EnumMutationBuilder` (~210 lines)
   - `StructMutationBuilder` (~724 lines!)
   - `TupleMutationBuilder` (~215 lines)
   - `MapMutationBuilder` (~68 lines)
   - `DefaultMutationBuilder` (~41 lines)

## Discovered Issues

### 1. Circular Dependencies

There's a **mutual recursion** between `mutation_path_builders.rs` and `type_info.rs`:

```
type_info::build_example_value_for_type()
    → EnumMutationBuilder::build_enum_example()
    → EnumMutationBuilder::build_struct_example_from_properties()
        → TypeInfo::build_example_value_for_type() [recursion]
```

### 2. Misnamed/Misplaced Methods

- `EnumMutationBuilder::build_struct_example_from_properties()` builds struct examples but lives in the enum builder
- Originally created for enum struct variants but now used for all structs
- Creates confusion about responsibilities

### 3. Public Method Dependencies

External usage from `type_info.rs`:
- `build_enum_example()` - Used for building enum examples
- `build_struct_example_from_properties()` - Used for building struct examples
- `extract_enum_variants()` - Only used internally (could be private)

## Recommended Reorganization

### Option 1: Submodule Structure (Recommended)

```
mcp/src/brp_tools/brp_type_schema/
├── mutation_path_builders/
│   ├── mod.rs              # Core trait, TypeKind impl, re-exports
│   ├── context.rs          # MutationPathContext, RootOrField
│   ├── helpers.rs          # Extract functions, HardcodedPathsResult
│   ├── examples.rs         # Shared example building (resolves circular dep)
│   └── builders/
│       ├── mod.rs          # Re-exports all builders
│       ├── array.rs        # ArrayMutationBuilder
│       ├── enum_type.rs    # EnumMutationBuilder
│       ├── struct_type/    # StructMutationBuilder (split further)
│       │   ├── mod.rs      # Main struct builder
│       │   ├── properties.rs # Property extraction & iteration
│       │   ├── nested.rs   # Nested field expansion
│       │   └── hardcoded.rs # Hardcoded knowledge handling
│       ├── tuple.rs        # TupleMutationBuilder
│       ├── map.rs          # MapMutationBuilder
│       └── default.rs      # DefaultMutationBuilder
```

### Option 2: Peer Module Structure

```
mcp/src/brp_tools/brp_type_schema/
├── mutation_path_builders.rs     # Core trait & dispatch only
├── mutation_context.rs            # MutationPathContext, RootOrField
├── mutation_examples.rs           # Shared example building
├── mutation_builder_array.rs     
├── mutation_builder_enum.rs      
├── mutation_builder_struct.rs    # Still large, but isolated
├── mutation_builder_tuple.rs     
├── mutation_builder_map.rs       
└── mutation_builder_default.rs   
```

## Benefits of Reorganization

1. **Better Navigation**: Each builder in its own file (100-200 lines typically)
2. **Clear Boundaries**: Separation of concerns between context, trait, and implementations
3. **Easier Maintenance**: Changes isolated to specific builders
4. **Resolved Circular Deps**: Example building extracted to shared module
5. **Scalability**: StructMutationBuilder can be further split into submodules

## Migration Strategy

### Phase 1: Current Refactor (In Progress)
- Simplifying example creation for mutation paths
- Removing redundant validation

### Phase 2: Remove WrapperType Special Handling
- Treat wrappers as regular enums
- Simplify the type detection logic
- This will reduce complexity before reorganization

### Phase 3: Module Reorganization
1. Create new module structure
2. Move example building methods to shared `examples.rs`
3. Split builders into separate files
4. Break up StructMutationBuilder into logical submodules
5. Update imports and visibility

### Phase 4: Cleanup
- Make `extract_enum_variants()` private if only used internally
- Rename methods for clarity (e.g., generic `build_struct_example` instead of `build_struct_example_from_properties`)
- Add module-level documentation

## Special Considerations

1. **Preserving Public API**: The public methods used by `type_info.rs` need to remain accessible
2. **Testing**: Ensure comprehensive tests during reorganization
3. **Documentation**: Add clear module docs explaining the builder pattern and type-directed dispatch
4. **Performance**: The reorganization shouldn't impact performance (just code organization)

## Future Improvements

After reorganization:
1. Consider extracting immutability propagation logic to shared utilities
2. Potentially create a `validation` submodule for mutation support checking
3. Look into further simplifying the hardcoded knowledge system
4. Consider if some builders (like `DefaultMutationBuilder`) could be merged

## Timeline

- **Phase 1**: Current (simplification refactor)
- **Phase 2**: Next sprint (remove wrapper types)
- **Phase 3**: Following sprint (module reorganization)
- **Phase 4**: Cleanup and documentation

This reorganization will make the codebase significantly more maintainable while preserving all current functionality.