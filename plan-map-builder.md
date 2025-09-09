# MapMutationBuilder Migration Plan

## The Big Picture

**Current State**: MapMutationBuilder calls ExampleBuilder 5 times to generate examples, duplicating the traversal logic that already exists in the path building system.

**Goal**: Convert MapMutationBuilder to use the ProtocolEnforcer pattern where it ONLY needs to:
1. Identify its children (key and value types)
2. Assemble the map from child examples
3. Let ProtocolEnforcer handle ALL the complexity (recursion, depth, knowledge checks)

## Current Architecture (BEFORE)

### What MapMutationBuilder Does Now:
```rust
build_paths() {
    // 1. Calls build_schema_example() to get map example
    let example = self.build_schema_example(ctx, depth);
    
    // 2. Returns single path (maps have no nested paths)
    Ok(vec![MutationPathInternal { example, ... }])
}

build_schema_example() {
    // 1. Extracts key/value types from schema
    // 2. Calls ExampleBuilder for key type (LINE 79)
    // 3. Calls ExampleBuilder for value type (LINE 80)
    // 4. Converts key to string and builds map
}

build_map_example_static() {
    // DUPLICATE of build_schema_example logic
    // Calls ExampleBuilder for key (LINE 132)
    // Calls ExampleBuilder for value (LINE 133)
}

build_map_mutation_path() {
    // Calls ExampleBuilder for entire map (LINE 161)
}
```

### The Problem:
- **5 ExampleBuilder calls** scattered across 3 methods
- **Logic duplication** between instance and static methods
- **No recursion** into key/value types for nested paths
- **Maps treated as leaf nodes** when they actually have children

## New Architecture (AFTER)

### How It Works With ProtocolEnforcer:

When `TypeKind::builder()` is called for a Map type and `is_migrated() = true`:
1. Returns `ProtocolEnforcer::new(MapMutationBuilder)`
2. ProtocolEnforcer calls `MapMutationBuilder::collect_children()`
3. **ProtocolEnforcer recursively builds FULL paths for key and value** (depth-first traversal)
4. **ProtocolEnforcer extracts the ROOT PATH example from each child's path tree**
5. ProtocolEnforcer calls `MapMutationBuilder::assemble_from_children()` with **complete examples**
6. ProtocolEnforcer constructs the final path list

### CRITICAL: What assemble_from_children() Receives

For a `HashMap<String, Transform>`, the `assemble_from_children()` method receives:
- **"key"**: A complete String example (e.g., `"example_key"`)
- **"value"**: A **COMPLETE Transform example** from Transform's root mutation path

The Transform example will be the full spawn format:
```json
{
  "translation": [10.0, 0.0, 5.0],
  "rotation": [0.0, 0.0, 0.0, 1.0],
  "scale": [1.0, 1.0, 1.0]
}
```

**This happens automatically in Phase 5b!** As soon as MapMutationBuilder is migrated and wrapped with ProtocolEnforcer, the depth-first recursion provides complete examples to assemble_from_children().

### What MapMutationBuilder Needs to Implement:

```rust
impl MutationPathBuilder for MapMutationBuilder {
    // 1. PANIC GUARD - Never called when wrapped by ProtocolEnforcer
    fn build_paths(&self, ctx: &RecursionContext, _depth: RecursionDepth) 
        -> Result<Vec<MutationPathInternal>> {
        tracing::error!("MapMutationBuilder::build_paths() called directly! Type: {}", ctx.type_name());
        panic!("MapMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}", ctx.type_name());
    }
    
    // 2. MIGRATION FLAG - Tells system to use ProtocolEnforcer
    fn is_migrated(&self) -> bool { 
        true 
    }
    
    // 3. IDENTIFY CHILDREN - Extract key and value types
    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        let Some(schema) = ctx.require_schema() else {
            return vec![];
        };
        
        // Extract key type from schema
        let key_type = schema
            .get_field(SchemaField::KeyType)
            .and_then(|key_field| key_field.get_field(SchemaField::Type))
            .and_then(/* extract type reference */);
            
        // Extract value type from schema  
        let value_type = schema
            .get_field(SchemaField::ValueType)
            .and_then(|value_field| value_field.get_field(SchemaField::Type))
            .and_then(/* extract type reference */);
            
        let mut children = vec![];
        
        if let Some(key_t) = key_type {
            // Create context for key recursion
            let key_ctx = /* create RecursionContext for key type */;
            children.push(("key".to_string(), key_ctx));
        }
        
        if let Some(val_t) = value_type {
            // Create context for value recursion
            let val_ctx = /* create RecursionContext for value type */;
            children.push(("value".to_string(), val_ctx));
        }
        
        children
    }
    
    // 4. ASSEMBLE FROM CHILDREN - Build map from COMPLETE examples
    fn assemble_from_children(
        &self, 
        _ctx: &RecursionContext, 
        children: HashMap<String, Value>
    ) -> Value {
        // At this point, children contains COMPLETE examples:
        // - "key": Full example for the key type (e.g., "example_key" for String)
        // - "value": Full example for the value type (e.g., complete Transform JSON)
        
        let key_example = children.get("key").unwrap_or(&json!("example_key"));
        let value_example = children.get("value").unwrap_or(&json!("example_value"));
        
        // Convert key to string (JSON maps need string keys)
        let key_str = match key_example {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => serde_json::to_string(other).unwrap_or_else(|_| "example_key".to_string())
        };
        
        // Build final map with the COMPLETE value example
        // For HashMap<String, Transform>, value_example is the full Transform
        let mut map = serde_json::Map::new();
        map.insert(key_str, value_example.clone());
        json!(map)
    }
}
```

### AND DON'T FORGET: Update TypeKind::build_paths()

In `type_kind.rs` line 177, change the Map dispatch:
```rust
// BEFORE (will cause panic!):
Self::Map => MapMutationBuilder.build_paths(ctx, builder_depth),

// AFTER (uses ProtocolEnforcer wrapper):
Self::Map => self.builder().build_paths(ctx, builder_depth),
```

## What Gets Deleted

### Methods to DELETE entirely:
1. `build_schema_example()` - Lines 53-100 (48 lines)
2. `build_map_example_static()` - Lines 106-153 (48 lines)
3. `build_map_mutation_path()` - Lines 156-171 (16 lines)

### Imports to DELETE:
- Line 19: `use crate::brp_tools::brp_type_schema::example_builder::ExampleBuilder;`

### Total code reduction: ~112 lines deleted, ~40 lines added = **72 lines saved**

## The Flow Change

### OLD Flow (Complex):
```
TypeKind::Map -> build_paths() 
    -> build_schema_example() 
        -> ExampleBuilder::build_example(key)
        -> ExampleBuilder::build_example(value)
    -> return single path
```

### NEW Flow (Simple):
```
TypeKind::Map -> ProtocolEnforcer::build_paths()
    -> MapMutationBuilder::collect_children() // Just identify key/value
    -> [ProtocolEnforcer handles recursion to key/value]
    -> MapMutationBuilder::assemble_from_children() // Just build map
    -> return complete path tree
```

## Key Insights

1. **Maps ARE NOT leaf nodes** - They have key and value types that need recursion
2. **The builder only needs to know WHAT its children are** - Not HOW to build them
3. **ProtocolEnforcer handles ALL complexity** - Depth checking, knowledge lookup, recursion
4. **Bottom-up assembly** - Children complete first, then parent assembles from results

## Migration Steps

1. Add the 4 protocol methods (build_paths panic, is_migrated, collect_children, assemble_from_children)
2. Delete the 3 old methods (build_schema_example, build_map_example_static, build_map_mutation_path)
3. Remove ExampleBuilder import
4. **CRITICAL**: Update TypeKind::build_paths() to use trait dispatch for Map type
   - In `type_kind.rs` line 177, change:
   - FROM: `Self::Map => MapMutationBuilder.build_paths(ctx, builder_depth),`
   - TO: `Self::Map => self.builder().build_paths(ctx, builder_depth),`
   - This ensures the ProtocolEnforcer wrapper is used when is_migrated() = true
   - Without this change, the panic guard in MapMutationBuilder::build_paths() would trigger!
5. Test that maps still generate correct examples

## Why This Works

The ProtocolEnforcer pattern means MapMutationBuilder becomes MUCH simpler:
- **No more calling ExampleBuilder** - ProtocolEnforcer handles recursion
- **No more depth tracking** - ProtocolEnforcer handles it
- **No more knowledge checks** - ProtocolEnforcer handles it
- **Receives complete examples** - Children are fully recursed before assembly
- **Just two simple responsibilities**: identify children, assemble from complete results

### Example Flow for HashMap<String, Transform>

1. MapMutationBuilder identifies children: key (String) and value (Transform)
2. ProtocolEnforcer recursively builds Transform's FULL path tree
3. Transform builder returns multiple paths (root + fields)
4. ProtocolEnforcer extracts Transform's root path example (complete spawn format)
5. MapMutationBuilder receives this complete Transform example in assemble_from_children()
6. Final map contains real Transform data, not placeholders!

This is the same pattern that worked for DefaultMutationBuilder, just with the added step of identifying and assembling from key/value children.