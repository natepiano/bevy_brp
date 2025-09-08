# Plan: Unify Example Generation Through Path Builders

## Goal
**Eliminate code complexity and duplication by making path builders generate examples during their single type traversal, instead of having separate example generation systems that duplicate the same logic.**

## Current Problem: Code Duplication and Complexity
We have **three separate systems** that all contain logic for traversing and understanding the same type structures:

1. **Path building logic**: Path builders traverse types to build mutation paths
2. **Example building logic**: `TypeInfo::build_type_example()` contains separate logic to traverse the same types for examples  
3. **Spawn format logic**: `TypeInfo::build_spawn_format()` has yet another set of logic for the same traversals

**The core issue**: We're maintaining three separate codebases that all need to understand how to navigate structs, enums, arrays, tuples, etc. When we need to add support for a new type or change how a type works, we have to update logic in multiple places.

This causes:
- **Code duplication** - same type-handling logic scattered across multiple systems
- **Maintenance burden** - changes require updates in 2-3 different places
- **Inconsistency risk** - the separate systems can drift apart and generate different results
- **Complexity** - developers need to understand multiple systems instead of one
- **Double recursion depth tracking** - each system manages its own recursion limits

## Proposed Solution
Make path builders generate everything in a **single depth-first traversal**:
- **Depth-first, post-order traversal**: Recurse to children first, then construct parent examples from child results
- Each builder constructs examples **only after** all child recursions complete
- Spawn format assembled **bottom-up** from collected mutation path examples  
- Eliminate all other example generation code

**Critical traversal pattern**: Children must be processed completely before parent assembly can begin. This ensures that when building a struct example, all field examples are available; when building an array example, all element examples are ready.

## Concrete Example: Building Examples Bottom-Up

Consider this nested struct:
```rust
Person {
    name: String,           
    address: Address {      
        street: String,     
        city: String,       
    }
}
```

**Depth-first traversal builds examples at each level representing that level and everything below:**

1. **Build `.address.street`** → example: `"123 Main St"` (just the string)
2. **Build `.address.city`** → example: `"Portland"` (just the string)  
3. **Build `.address`** → example: `{"street": "123 Main St", "city": "Portland"}` (assembled from street/city)
4. **Build `.name`** → example: `"John"` (just the string)
5. **Build root/spawn format** → example: `{"name": "John", "address": {"street": "123 Main St", "city": "Portland"}}` (assembled from name/address)

**Key insight**: Each mutation path example represents the **complete subtree from that point down**. The spawn format is simply the **root level's complete example** - built using the exact same bottom-up assembly process as all other mutation path examples.

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
        
        // NEW: Single call gets both - construct spawn format from collected paths
        let mutation_paths_vec = Self::build_mutation_paths(...);  // Returns Vec<MutationPathInternal>
        
        // CONSTRUCT spawn format from mutation paths using depth-first results
        let spawn_format = Self::construct_spawn_format_from_paths(&mutation_paths_vec, type_kind);
            
        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec, &registry);

// NEW METHOD: Construct spawn format from mutation paths bottom-up
fn construct_spawn_format_from_paths(
    paths: &[MutationPathInternal], 
    type_kind: &TypeKind
) -> Option<Value> {
    match type_kind {
        TypeKind::Struct => {
            // Construct struct by collecting field examples from field paths
            let mut struct_obj = Map::new();
            for path in paths {
                if let PathKind::StructField { field_name, .. } = &path.path_kind {
                    struct_obj.insert(field_name.clone(), path.example.clone());
                }
            }
            if struct_obj.is_empty() { None } else { Some(Value::Object(struct_obj)) }
        }
        TypeKind::Array => {
            // Construct array by collecting element examples from element paths
            let mut elements = Vec::new();
            for path in paths {
                if let PathKind::ArrayElement { .. } = &path.path_kind {
                    elements.push(path.example.clone());
                }
            }
            if elements.is_empty() { None } else { Some(json!(elements)) }
        }
        TypeKind::Tuple => {
            // Construct tuple by collecting element examples in index order
            let mut tuple_elements = Vec::new();
            for path in paths {
                if let PathKind::IndexedElement { .. } = &path.path_kind {
                    tuple_elements.push(path.example.clone());
                }
            }
            if tuple_elements.is_empty() { None } else { Some(json!(tuple_elements)) }
        }
        _ => None, // Other types don't support spawn
    }
}
        
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

Each builder's `build_paths` method will follow **depth-first, post-order traversal**:
1. **Recurse to all children first** - collect child paths with their examples
2. **Wait for all child recursions to complete** - ensure child examples are ready
3. **Assemble parent example** using child results (bottom-up construction)
4. **Return complete paths with examples** from single depth-first traversal

**Traversal ordering is critical**: Parent examples can only be constructed after ALL child examples are available.

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

    /// Build example using depth-first traversal - ensures children complete before parent
    /// Default implementation handles knowledge lookup and enforces traversal ordering
    fn build_example_with_knowledge(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Value {
        // First check BRP_MUTATION_KNOWLEDGE for hardcoded examples
        if let Some(example) = KnowledgeKey::find_example_for_type(ctx.type_name()) {
            return example;
        }
        
        // DEPTH-FIRST DISPATCH - ensures proper traversal ordering
        let Some(schema) = ctx.require_schema() else { return json!(null); };
        let type_kind = TypeKind::from_schema(schema, ctx.type_name());
        
        // Dispatch to appropriate builder - each builder MUST complete child recursion first
        match type_kind {
            TypeKind::Array => ArrayMutationBuilder.build_schema_example(ctx, depth),
            TypeKind::Struct => StructMutationBuilder.build_schema_example(ctx, depth),
            TypeKind::Enum => EnumMutationBuilder.build_schema_example(ctx, depth),
            TypeKind::Tuple | TypeKind::TupleStruct => TupleMutationBuilder.build_schema_example(ctx, depth),
            TypeKind::List => ListMutationBuilder.build_schema_example(ctx, depth),
            TypeKind::Set => SetMutationBuilder.build_schema_example(ctx, depth),
            TypeKind::Map => MapMutationBuilder.build_schema_example(ctx, depth),
            _ => DefaultMutationBuilder.build_schema_example(ctx, depth),
        }
    }

    /// Build example from schema - implemented by each builder for their specific type
    /// Uses RecursionContext to access schema and helper methods
    fn build_schema_example(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Value;
}
```

**Usage in builders**: Replace `TypeInfo::build_type_example(type_name, registry, depth)` calls with `self.build_example_with_knowledge(ctx, depth)` where `ctx` is a RecursionContext for the target type. This ensures:
- Consistent BRP_MUTATION_KNOWLEDGE integration across all builders
- **Central type dispatch** - the trait method automatically routes to the correct builder
- **Recursive example building** - child types get routed to their appropriate builders
- No duplication of knowledge lookup or dispatch logic
- Proper fallback to schema-based generation when no hardcoded knowledge exists

**Key Architecture**: The `build_example_with_knowledge` default implementation becomes the new central dispatcher, replacing `TypeInfo::build_type_example`'s routing logic while preserving knowledge lookup.

#### Builder-Specific Schema Example Implementations

Each builder implements `build_schema_example()` using the extracted TypeInfo logic:

**ArrayMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let Some(field_schema) = ctx.require_schema() else { return json!(null); };
    
    // DEPTH-FIRST PATTERN: Extract child type info first
    let item_type = ctx.extract_list_element_type(field_schema);

    item_type.map_or(json!(null), |item_type_name| {
        // STEP 1: RECURSE TO CHILD FIRST - complete child traversal before parent assembly
        let item_path_kind = PathKind::RootValue { type_name: item_type_name.clone() };
        let item_ctx = RecursionContext::new(item_path_kind, Arc::clone(&ctx.registry));
        
        // CRITICAL: Child recursion MUST complete before parent construction
        let item_example = self.build_example_with_knowledge(&item_ctx, depth.increment());
        
        // STEP 2: CONSTRUCT PARENT AFTER CHILD COMPLETION - bottom-up assembly
        let size = ctx.type_name().as_str()
            .rsplit_once("; ")
            .and_then(|(_, rest)| rest.strip_suffix(']'))
            .and_then(|s| s.parse::<usize>().ok())
            .map_or(DEFAULT_EXAMPLE_ARRAY_SIZE, |s| s.min(MAX_EXAMPLE_ARRAY_SIZE));
        
        // Parent assembly using completed child example - builds complete array for THIS level
        let array = vec![item_example; size];
        json!(array)
        
        // CRITICAL: This array example represents the complete array from this level down
        // Example: [10.5, 10.5, 10.5] - this becomes the example for this array mutation path
        // If a struct contains this array field, this complete array becomes that field's value
    })
}
```

**StructMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let Some(field_schema) = ctx.require_schema() else { return json!(null); };
    
    // DEPTH-FIRST PATTERN: Process all fields before struct assembly
    field_schema
        .get_field(SchemaField::Properties)
        .map_or(json!(null), |properties| {
            self.build_struct_example_from_properties_with_knowledge(properties, ctx, depth.increment())
        })
}

// DEPTH-FIRST utility method - ensures all field recursions complete before struct construction:
fn build_struct_example_from_properties_with_knowledge(&self, properties: &Value, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let mut struct_example = Map::new();
    
    // STEP 1: RECURSE TO ALL FIELDS FIRST - collect all field examples
    for (field_name, field_schema) in properties.as_object().unwrap_or(&Map::new()) {
        if let Some(field_type) = SchemaField::extract_field_type(field_schema) {
            // CRITICAL: Each field recursion MUST complete before moving to next field
            let field_path_kind = PathKind::RootValue { type_name: field_type.clone() };
            let field_ctx = RecursionContext::new(field_path_kind, Arc::clone(&ctx.registry));
            
            // Child recursion completes before parent assembly continues
            let field_example = self.build_example_with_knowledge(&field_ctx, depth);
            struct_example.insert(field_name.clone(), field_example);
        }
    }
    
    // STEP 2: CONSTRUCT STRUCT AFTER ALL FIELD COMPLETIONS - bottom-up assembly
    // This creates the example for THIS level containing all child examples
    // Example: {"name": "John", "address": {"street": "123 Main", "city": "Portland"}}
    Value::Object(struct_example)
    
    // CRITICAL: This struct example becomes the example for any parent path that contains this struct
    // If this is `.address`, this complete struct becomes the value for the `.address` mutation path
    // If this is root level, this complete struct becomes the spawn format
}
```

**TupleMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let Some(field_schema) = ctx.require_schema() else { return json!(null); };
    
    // DEPTH-FIRST PATTERN: Process all tuple elements before tuple assembly
    field_schema
        .get_field(SchemaField::PrefixItems)
        .and_then(Value::as_array)
        .map_or(json!(null), |prefix_items| {
            let mut tuple_examples = Vec::new();
            
            // STEP 1: RECURSE TO ALL ELEMENTS FIRST - complete each element before next
            for item in prefix_items {
                if let Some(element_type) = item.get_field(SchemaField::Type)
                    .and_then(Self::extract_type_ref_with_schema_field) 
                {
                    // CRITICAL: Each element recursion MUST complete before moving to next
                    let element_path_kind = PathKind::RootValue { type_name: element_type.clone() };
                    let element_ctx = RecursionContext::new(element_path_kind, Arc::clone(&ctx.registry));
                    
                    // Child recursion completes before parent assembly continues
                    let element_example = self.build_example_with_knowledge(&element_ctx, depth.increment());
                    tuple_examples.push(element_example);
                } else {
                    tuple_examples.push(json!(null));
                }
            }

            // STEP 2: CONSTRUCT TUPLE AFTER ALL ELEMENT COMPLETIONS - bottom-up assembly
            if tuple_examples.is_empty() { 
                json!(null) 
            } else { 
                json!(tuple_examples)
                // CRITICAL: This tuple example represents the complete tuple from this level down
                // Example: [10.5, "hello", true] - this becomes the example for this tuple mutation path
                // If a struct contains this tuple field, this complete tuple becomes that field's value
            }
        })
}
```

**EnumMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let Some(field_schema) = ctx.require_schema() else { return json!(null); };
    
    // DEPTH-FIRST PATTERN: Process enum variants (may contain structs/tuples with nested fields)
    let enum_example = EnumMutationBuilder::build_enum_example(
        field_schema,
        &ctx.registry,
        Some(ctx.type_name()),
        depth.increment(),
    );
    
    // CRITICAL: This enum example represents the complete enum value from this level down
    // Examples: 
    // - Unit variant: "Active" 
    // - Struct variant: {"Config": {"timeout": 30, "retries": 3}}
    // - Tuple variant: {"Point": [10.5, 20.0]}
    // If a struct contains this enum field, this complete enum becomes that field's value
    enum_example
}
```

**ListMutationBuilder & SetMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let Some(field_schema) = ctx.require_schema() else { return json!(null); };
    
    // DEPTH-FIRST PATTERN: Extract item type and recurse to build item examples first
    let item_type = field_schema
        .get_field(SchemaField::Items)
        .and_then(|items| items.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);

    item_type.map_or(json!(null), |item_type_name| {
        // STEP 1: RECURSE TO ITEM TYPE FIRST - complete item example before collection assembly
        let item_path_kind = PathKind::RootValue { type_name: item_type_name.clone() };
        let item_ctx = RecursionContext::new(item_path_kind, Arc::clone(&ctx.registry));
        
        // CRITICAL: Item recursion MUST complete before collection construction
        let item_example = self.build_example_with_knowledge(&item_ctx, depth.increment());
        
        // STEP 2: CONSTRUCT COLLECTION AFTER ITEM COMPLETION - bottom-up assembly
        let collection = vec![item_example; 2];
        json!(collection)
        
        // CRITICAL: This collection example represents the complete Vec/HashSet from this level down
        // Examples: [{"name": "John"}, {"name": "Jane"}] or [10, 20] 
        // If a struct contains this collection field, this complete collection becomes that field's value
    })
}
```

**MapMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    let Some(field_schema) = ctx.require_schema() else { return json!(null); };
    
    // DEPTH-FIRST PATTERN: Extract key/value types and recurse to build examples first
    let key_type = field_schema.get_field(SchemaField::PatternProperties)
        .and_then(|props| props.as_object()?.values().next())
        .and_then(|v| v.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);
        
    let value_type = field_schema.get_field(SchemaField::AdditionalProperties)
        .and_then(|props| props.get_field(SchemaField::Type))
        .and_then(Self::extract_type_ref_with_schema_field);

    match (key_type, value_type) {
        (Some(key_type_name), Some(value_type_name)) => {
            // STEP 1: RECURSE TO KEY AND VALUE TYPES FIRST
            let key_path_kind = PathKind::RootValue { type_name: key_type_name.clone() };
            let key_ctx = RecursionContext::new(key_path_kind, Arc::clone(&ctx.registry));
            let key_example = self.build_example_with_knowledge(&key_ctx, depth.increment());
            
            let value_path_kind = PathKind::RootValue { type_name: value_type_name.clone() };
            let value_ctx = RecursionContext::new(value_path_kind, Arc::clone(&ctx.registry));
            let value_example = self.build_example_with_knowledge(&value_ctx, depth.increment());
            
            // STEP 2: CONSTRUCT MAP AFTER KEY/VALUE COMPLETION - bottom-up assembly
            let mut map = Map::new();
            map.insert("example_key".to_string(), value_example);
            json!(map)
            
            // CRITICAL: This map example represents the complete HashMap from this level down
            // Example: {"player_1": {"score": 100, "level": 5}}
            // If a struct contains this map field, this complete map becomes that field's value
        }
        _ => json!(null)
    }
}
```

**DefaultMutationBuilder**:
```rust
fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
    // EXTRACTED from TypeInfo::build_type_example default branch:
    // Primitive/default types don't need recursion - return null
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

## Migration Strategy: Incremental Commit Sequence

**Safe incremental migration**: Each commit can be tested independently without breaking existing functionality:

### Commit 1: Add trait infrastructure
- Add new trait methods `build_example_with_knowledge()` and `build_schema_example()` to `MutationPathBuilder`
- Provide default implementations that temporarily delegate to `TypeInfo::build_type_example`
- **Verification**: Code compiles, all tests pass with no behavior changes

### Commit 2-9: Migrate builders individually  
Each builder gets its own commit to implement `build_schema_example()`:

**Commit 2**: `DefaultMutationBuilder` - simplest case (returns `json!(null)`)
**Commit 3**: `ArrayMutationBuilder` - implement depth-first array example building  
**Commit 4**: `ListMutationBuilder` & `SetMutationBuilder` - similar collection pattern
**Commit 5**: `MapMutationBuilder` - key/value example building
**Commit 6**: `TupleMutationBuilder` - tuple element assembly
**Commit 7**: `StructMutationBuilder` - field example assembly  
**Commit 8**: `EnumMutationBuilder` - variant example building
**Commit 9**: Update trait's `build_example_with_knowledge()` to use proper type dispatch instead of `TypeInfo::build_type_example`

**Verification after each**: Run tests for that specific builder type

### Commit 10: Update path building to populate examples
- Modify each builder's `build_paths()` to populate the `example` field using the new methods
- Examples now built during path traversal, but `TypeInfo` still uses old methods
- **Verification**: Path examples are correct, spawn format still works via old path

### Commit 11: Add spawn format construction method
- Add `construct_spawn_format_from_paths()` to `TypeInfo`  
- Update `TypeInfo::from_schema()` to use new construction method
- Keep old functions for now as safety net
- **Verification**: Both old and new spawn format generation produce identical results

### Commit 12: Remove old functions
- Remove `TypeInfo::build_type_example()`
- Remove `TypeInfo::build_example_value_for_type()`  
- Remove `TypeInfo::build_spawn_format()` and helpers
- Remove any test scaffolding
- **Verification**: Full test suite passes with only new system

### Key Benefits of This Sequence:
1. **Each commit is independently testable** - can verify correctness at each step
2. **Bisectable** - if issues arise, can identify exact commit that introduced them
3. **Rollback-friendly** - can revert individual commits if needed
4. **No parallel systems** - avoids complexity of maintaining two systems
5. **Clear progress tracking** - can see migration progress builder by builder

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

### ⚠️ PREJUDICE WARNING - DUPLICATION-6: Plan relocates duplication rather than eliminating it
- **Status**: PERMANENTLY REJECTED
- **Category**: DUPLICATION
- **Location**: Section: Code Extraction Details: What Gets Moved Where
- **Issue**: Plan moves logic from TypeInfo::build_type_example into individual builders, but this just relocates the same example-generation code into 8 different files. The real duplication issue is that THREE separate systems generate examples: builders, TypeInfo, and spawn format builders. Moving code between them doesn't eliminate the fundamental duplication.
- **Reasoning**: This finding is based on a fundamental misunderstanding of the plan's architecture. The plan DOES eliminate the three separate systems by consolidating ALL example generation into path builders during their single traversal, then completely removing TypeInfo::build_type_example and spawn format builders entirely. The logic migration consolidates example generation into ONE system (path builders) instead of three separate systems.
- **Critical Note**: DO NOT SUGGEST THIS AGAIN - Permanently rejected by user

### IMPLEMENTATION-2: Heavy RecursionContext creation for every recursive call is performance-expensive - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Category**: IMPLEMENTATION
- **Location**: Section: Builder-Specific Schema Example Implementations
- **Issue**: The plan suggests creating new RecursionContext instances for every recursive call. RecursionContext contains Arc<HashMap<BrpTypeName, Value>> which is expensive to clone, even with Arc.
- **Reasoning**: This finding is based on a fundamental misunderstanding of Arc performance characteristics. Arc::clone() is extremely cheap - it only increments an atomic reference counter (1-2 CPU cycles) and does NOT clone the underlying HashMap data. Arc is specifically designed for exactly this use case. The current RecursionContext design is well-architected and efficient.
- **Decision**: User elected to skip this recommendation

