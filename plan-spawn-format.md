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
// Path builders generate examples during traversal (no change to signature)
// Spawn format = root path's example (PathKind::RootValue)
fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>>
```

### Files That Will Change

#### `mcp/src/brp_tools/brp_type_schema/mutation_path_builder/mod.rs`
The `MutationPathBuilder` trait signature remains unchanged:
```rust
pub trait MutationPathBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>>;
}
```

**Key change**: Builders no longer call `TypeInfo::build_type_example()`. Instead, they build examples internally during the single path traversal.

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
        let example = TypeInfo::build_type_example(ctx.type_name(), &ctx.registry, depth);
        // ...
    }
}
```

To:
```rust
impl MutationPathBuilder for SomeBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        // Use the logic migrated from TypeInfo::build_type_example for this type
        // Each builder contains the example-building logic for its specific type
        // (no more calls to TypeInfo::build_type_example)
        
        // Build this level's example using migrated type-specific logic
        // Recurse for child paths - each child builds its own example bottom-up
        // Assemble complete paths with examples in single traversal
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
        
        // NEW: Single call gets both - extract spawn format from paths
        let mutation_paths_vec = Self::build_mutation_paths(...);  // Returns Vec<MutationPathInternal>
        
        // Extract spawn format from root path (PathKind::RootValue)
        let spawn_format = mutation_paths_vec.iter()
            .find(|path| matches!(path.path_kind, PathKind::RootValue(_)))
            .map(|path| path.example.clone());
            
        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec, &registry);
        
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

### Code Extraction Details: What Gets Moved Where

The core insight is that `TypeInfo::build_type_example` contains a match statement (lines 311-410) where each branch handles a specific `TypeKind`. Each branch's logic needs to be extracted and moved to the corresponding builder:

#### From `TypeInfo::build_type_example` match statement:

**Array Logic → `ArrayMutationBuilder`** (lines 318-344):
```rust
// EXTRACT THIS BLOCK FROM build_type_example:
TypeKind::Array => {
    let item_type = field_schema
        .get_field(SchemaField::Items)
        .and_then(|items| items.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);

    item_type.map_or(json!(null), |item_type_name| {
        let item_example = Self::build_type_example(&item_type_name, registry, depth.increment());
        let size = type_name.as_str()
            .rsplit_once("; ")
            .and_then(|(_, rest)| rest.strip_suffix(']'))
            .and_then(|s| s.parse::<usize>().ok())
            .map_or(DEFAULT_EXAMPLE_ARRAY_SIZE, |s| s.min(MAX_EXAMPLE_ARRAY_SIZE));
        let array = vec![item_example; size];
        json!(array)
    })
}
```

**Tuple Logic → `TupleMutationBuilder`** (lines 346-376):
```rust
// EXTRACT THIS BLOCK FROM build_type_example:
TypeKind::Tuple | TypeKind::TupleStruct => {
    field_schema
        .get_field(SchemaField::PrefixItems)
        .and_then(Value::as_array)
        .map_or(json!(null), |prefix_items| {
            let tuple_examples: Vec<Value> = prefix_items
                .iter()
                .map(|item| {
                    item.get_field(SchemaField::Type)
                        .and_then(Self::extract_type_ref_with_schema_field)
                        .map_or_else(
                            || json!(null),
                            |ft| Self::build_type_example(&ft, registry, depth.increment()),
                        )
                })
                .collect();

            if tuple_examples.is_empty() {
                json!(null)
            } else {
                json!(tuple_examples)
            }
        })
}
```

**Struct Logic → `StructMutationBuilder`** (lines 377-388):
```rust
// EXTRACT THIS BLOCK FROM build_type_example:
TypeKind::Struct => {
    field_schema
        .get_field(SchemaField::Properties)
        .map_or(json!(null), |properties| {
            StructMutationBuilder::build_struct_example_from_properties(
                properties,
                registry,
                depth.increment(),
            )
        })
}
```

**List/Set Logic → `ListMutationBuilder` and `SetMutationBuilder`** (lines 389-408):
```rust
// EXTRACT THIS BLOCK FROM build_type_example:
TypeKind::List | TypeKind::Set => {
    let item_type = field_schema
        .get_field(SchemaField::Items)
        .and_then(|items| items.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);

    item_type.map_or(json!(null), |item_type_name| {
        let item_example = Self::build_type_example(&item_type_name, registry, depth.increment());
        let array = vec![item_example; 2];
        json!(array)
    })
}
```

**Enum Logic → `EnumMutationBuilder`** (lines 312-317):
```rust
// EXTRACT THIS BLOCK FROM build_type_example:
TypeKind::Enum => EnumMutationBuilder::build_enum_example(
    field_schema,
    registry,
    Some(type_name),
    depth.increment(),
),
```

**Default/Value Logic → `DefaultMutationBuilder`** (line 409):
```rust
// EXTRACT THIS BLOCK FROM build_type_example:
_ => json!(null),
```

#### Key Integration Points:

Each builder will integrate these code blocks at the point where they currently call `TypeInfo::build_type_example`. The recursive calls to `Self::build_type_example(&field_type, registry, depth.increment())` within these blocks will be replaced with direct builder dispatch through the unified system.

#### BRP_MUTATION_KNOWLEDGE Integration

**CRITICAL**: The knowledge lookup step (lines 300-303) from `TypeInfo::build_type_example` must be preserved:
```rust
// Use enum dispatch for format knowledge lookup
if let Some(example) = KnowledgeKey::find_example_for_type(type_name) {
    return example;
}
```

This will be handled through a new trait method with default implementation:

```rust
pub trait MutationPathBuilder {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>>;

    /// Check for hardcoded knowledge example, falling back to schema-based generation
    /// Default implementation handles the knowledge lookup pattern used by all builders
    fn build_example_with_knowledge(
        &self,
        type_name: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // First check BRP_MUTATION_KNOWLEDGE for hardcoded examples
        if let Some(example) = KnowledgeKey::find_example_for_type(type_name) {
            return example;
        }
        
        // Fall back to builder-specific schema-based example generation
        self.build_schema_example(type_name, registry, depth)
    }

    /// Build example from schema - implemented by each builder for their specific type
    fn build_schema_example(
        &self,
        type_name: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value;
}
```

**Usage in builders**: Replace `TypeInfo::build_type_example(type_name, registry, depth)` calls with `self.build_example_with_knowledge(type_name, registry, depth)`. This ensures:
- Consistent BRP_MUTATION_KNOWLEDGE integration across all builders
- No duplication of knowledge lookup logic
- Proper fallback to schema-based generation when no hardcoded knowledge exists

#### Builder-Specific Schema Example Implementations

Each builder implements `build_schema_example()` using the extracted TypeInfo logic:

**ArrayMutationBuilder**:
```rust
fn build_schema_example(&self, type_name: &BrpTypeName, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    let Some(field_schema) = registry.get(type_name) else { return json!(null); };
    
    // EXTRACTED from TypeInfo::build_type_example TypeKind::Array branch:
    let item_type = field_schema
        .get_field(SchemaField::Items)
        .and_then(|items| items.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);

    item_type.map_or(json!(null), |item_type_name| {
        let item_example = self.build_example_with_knowledge(&item_type_name, registry, depth.increment());
        let size = type_name.as_str()
            .rsplit_once("; ")
            .and_then(|(_, rest)| rest.strip_suffix(']'))
            .and_then(|s| s.parse::<usize>().ok())
            .map_or(DEFAULT_EXAMPLE_ARRAY_SIZE, |s| s.min(MAX_EXAMPLE_ARRAY_SIZE));
        let array = vec![item_example; size];
        json!(array)
    })
}
```

**StructMutationBuilder**:
```rust
fn build_schema_example(&self, type_name: &BrpTypeName, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    let Some(field_schema) = registry.get(type_name) else { return json!(null); };
    
    // EXTRACTED from TypeInfo::build_type_example TypeKind::Struct branch:
    field_schema
        .get_field(SchemaField::Properties)
        .map_or(json!(null), |properties| {
            self.build_struct_example_from_properties_with_knowledge(properties, registry, depth.increment())
        })
}

// Updated utility method to use trait method:
fn build_struct_example_from_properties_with_knowledge(&self, properties: &Value, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    // ... existing logic but replace TypeInfo::build_type_example calls with:
    self.build_example_with_knowledge(&field_type, registry, depth)
}
```

**TupleMutationBuilder**:
```rust
fn build_schema_example(&self, type_name: &BrpTypeName, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    let Some(field_schema) = registry.get(type_name) else { return json!(null); };
    
    // EXTRACTED from TypeInfo::build_type_example TypeKind::Tuple branch:
    field_schema
        .get_field(SchemaField::PrefixItems)
        .and_then(Value::as_array)
        .map_or(json!(null), |prefix_items| {
            let tuple_examples: Vec<Value> = prefix_items
                .iter()
                .map(|item| {
                    item.get_field(SchemaField::Type)
                        .and_then(Self::extract_type_ref_with_schema_field)
                        .map_or_else(
                            || json!(null),
                            |ft| self.build_example_with_knowledge(&ft, registry, depth.increment()),
                        )
                })
                .collect();

            if tuple_examples.is_empty() { json!(null) } else { json!(tuple_examples) }
        })
}
```

**EnumMutationBuilder**:
```rust
fn build_schema_example(&self, type_name: &BrpTypeName, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    let Some(field_schema) = registry.get(type_name) else { return json!(null); };
    
    // EXTRACTED from TypeInfo::build_type_example TypeKind::Enum branch:
    EnumMutationBuilder::build_enum_example_with_knowledge(field_schema, registry, Some(type_name), depth.increment(), self)
}

// Updated enum example builder to use trait method for recursive calls
```

**ListMutationBuilder & SetMutationBuilder**:
```rust
fn build_schema_example(&self, type_name: &BrpTypeName, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    let Some(field_schema) = registry.get(type_name) else { return json!(null); };
    
    // EXTRACTED from TypeInfo::build_type_example TypeKind::List/Set branch:
    let item_type = field_schema
        .get_field(SchemaField::Items)
        .and_then(|items| items.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);

    item_type.map_or(json!(null), |item_type_name| {
        let item_example = self.build_example_with_knowledge(&item_type_name, registry, depth.increment());
        let array = vec![item_example; 2];
        json!(array)
    })
}
```

**DefaultMutationBuilder**:
```rust
fn build_schema_example(&self, type_name: &BrpTypeName, registry: &HashMap<BrpTypeName, Value>, depth: RecursionDepth) -> Value {
    // EXTRACTED from TypeInfo::build_type_example default branch:
    json!(null)
}
```

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

1. Migrate example-building logic from `TypeInfo::build_type_example` into each builder
2. Update all 8 builder implementations simultaneously to use migrated logic:
   - `array_builder.rs`
   - `default_builder.rs`
   - `enum_builder.rs`
   - `list_builder.rs`
   - `map_builder.rs`
   - `set_builder.rs`
   - `struct_builder.rs`
   - `tuple_builder.rs`
3. Update the caller in `TypeInfo::from_schema()` to extract spawn format from root path
4. Remove `TypeInfo::build_type_example` and related functions that are no longer needed
5. Clean up any remaining references

## Testing Strategy

1. Ensure existing tests pass with new architecture
2. Verify spawn formats match current output
3. Verify mutation paths remain unchanged
4. Add tests for consistency between spawn format and mutation examples

## Design Review Skip Notes

### DESIGN-1: Inconsistent handling of recursion depth in unified approach - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Category**: DESIGN
- **Location**: Section: Core Concept
- **Issue**: Plan doesn't specify how recursion depth tracking will work when building both spawn format and mutation paths in single traversal
- **Existing Implementation**: The plan already specifies removing `TypeInfo::build_type_example` which contains the inconsistent depth tracking logic. The migrated logic will use consistent depth.increment() calls in each builder's single traversal
- **Plan Section**: Section: Logic Migration - Moving Example Building to Builders
- **Verdict**: CONFIRMED
- **Reasoning**: This finding correctly identified inconsistent recursion handling, but the solution already exists in the plan - removing the problematic `build_type_example` function and migrating its logic to builders eliminates the depth tracking inconsistencies
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### DESIGN-2: Error propagation strategy missing from unified architecture
- **Status**: SKIPPED
- **Category**: DESIGN
- **Location**: Section: Core Concept
- **Issue**: Plan doesn't specify how example generation errors will be handled in unified approach
- **Proposed Change**: Add error handling for separate spawn format and mutation path generation processes
- **Verdict**: CONFIRMED
- **Reasoning**: This is a real design issue about error handling, but it applies to the OLD architecture with separate build_spawn_format and build_mutation_paths functions. The new unified approach eliminates these separate functions - spawn format becomes the root path's example from the single build_paths traversal. Error handling will be part of the unified build_paths method, not separate processes
- **Decision**: User elected to skip this recommendation

### DESIGN-3: Public API breaking change not addressed in plan - **Verdict**: REJECTED
- **Status**: SKIPPED  
- **Location**: Section: Complete Function Removals
- **Issue**: Plan removes build_type_example but doesn't address public API build_example_value_for_type that depends on it
- **Reasoning**: This finding is based on a misunderstanding of the plan. The plan explicitly states that BOTH functions (build_type_example and build_example_value_for_type) will be removed entirely, not just build_type_example. Additionally, my codebase search confirms that build_example_value_for_type is only used internally within the same file - it's not actually used as an external public API anywhere else in the codebase. Therefore, this is not a breaking public API change but rather an internal refactoring where both functions are intentionally removed and their functionality migrated to path builders.
- **Decision**: User elected to skip this recommendation - finding addresses removed functionality

### IMPLEMENTATION-1: Utility function migration scope incomplete - **Verdict**: MODIFIED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Utility Function Migrations
- **Issue**: Function has circular dependency with TypeInfo::build_type_example that will break when TypeInfo functions are removed
- **Reasoning**: The finding correctly identified a circular dependency issue, but the solution already exists in the plan. The new "Code Extraction Details: What Gets Moved Where" section explicitly specifies moving struct example building logic (including build_struct_example_from_properties) into StructMutationBuilder while completely removing TypeInfo::build_type_example. This eliminates the circular dependency by design.
- **Existing Implementation**: Section "Code Extraction Details: What Gets Moved Where" shows that struct logic gets extracted FROM TypeInfo::build_type_example and moved INTO StructMutationBuilder, while build_struct_example_from_properties stays as a private utility within the builder. The removal of TypeInfo::build_type_example eliminates the circular dependency.
- **Plan Section**: Section: Code Extraction Details: What Gets Moved Where - Struct Logic → StructMutationBuilder
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### DESIGN-4: Inconsistent recursion depth handling in plan examples - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Logic Migration - Moving Example Building to Builders
- **Issue**: Inconsistent depth handling between builders - enum_builder increments depth before calling TypeInfo, while struct_builder passes depth unchanged
- **Reasoning**: The finding correctly identified inconsistent depth handling between builders when calling TypeInfo::build_type_example, but the entire call pattern gets eliminated by the plan's architecture. The plan removes TypeInfo::build_type_example entirely and moves example-building logic directly into each builder, eliminating all cross-calling between builders and TypeInfo.
- **Existing Implementation**: The "Code Extraction Details: What Gets Moved Where" section shows logic being extracted FROM TypeInfo::build_type_example and moved directly INTO each builder. The "Complete Function Removals" section removes TypeInfo::build_type_example entirely. This eliminates the inconsistent calling pattern the finding was concerned about.
- **Plan Section**: Section: Code Extraction Details: What Gets Moved Where and Section: Complete Function Removals
- **Critical Note**: This functionality/design already exists in the plan - the architectural change makes the depth handling inconsistency moot since there's no longer a central function for builders to call inconsistently

### DESIGN-5: Missing hardcoded knowledge integration specification - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Core Concept
- **Issue**: Plan doesn't specify how BRP_MUTATION_KNOWLEDGE hardcoded examples will integrate with unified builder approach
- **Reasoning**: The finding correctly identified that BRP_MUTATION_KNOWLEDGE integration was missing from the plan specification, but the solution has been added. The plan now includes a "BRP_MUTATION_KNOWLEDGE Integration" section that specifies using a trait method with default implementation to handle knowledge lookup consistently across all builders.
- **Existing Implementation**: The "BRP_MUTATION_KNOWLEDGE Integration" section shows adding `build_example_with_knowledge()` trait method with default implementation that checks hardcoded knowledge first, then falls back to builder-specific schema generation. This eliminates duplication while preserving the essential knowledge lookup step.
- **Plan Section**: Section: BRP_MUTATION_KNOWLEDGE Integration
- **Critical Note**: This functionality/design already exists in the plan - the trait method approach ensures consistent knowledge integration across all builders