# Guidelines for assemble_from_children Implementation

## Overview
Each container type has unique assembly logic in its `assemble_from_children` method. This document provides implementation patterns for each builder type.

## Core Principle
- Builders receive child values in the SAME ORDER they returned PathKinds from `collect_children()`
- Builders ONLY assemble examples - no mutation status determination
- ProtocolEnforcer determines all mutation status and reasons

## Implementation Patterns by Type

### MapMutationBuilder
Creates a JSON object with a single key-value pair.

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // Map expects exactly 2 children in positional order: [key, value]
    if children.len() != 2 {
        return Err(anyhow::anyhow!("Map requires exactly 2 children, got {}", children.len()));
    }
    
    let key_value = &children[0];
    let value_value = &children[1];
    
    // Create the map with the example key-value pair
    let mut map = serde_json::Map::new();
    
    // Convert key to string for JSON map
    let key_str = match key_value {
        Value::String(s) => s.clone(),
        _ => serde_json::to_string(key_value)
            .unwrap_or_else(|_| "example_key".to_string()),
    };
    
    map.insert(key_str, value_value.clone());
    Ok(Value::Object(map))
}
```

**Output example:** `{"example_key": {...value...}}`

### SetMutationBuilder
Creates a JSON array with a single example element.

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // Set expects exactly 1 child: [items]
    if children.len() != 1 {
        return Err(anyhow::anyhow!("Set requires exactly 1 child, got {}", children.len()));
    }
    
    let items_value = &children[0];
    
    // Create array with single example item
    Ok(Value::Array(vec![items_value.clone()]))
}
```

**Output example:** `[{...item...}]`

### ListMutationBuilder
Creates a JSON array with a single example element (similar to Set).

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // List expects exactly 1 child: [element]
    if children.len() != 1 {
        return Err(anyhow::anyhow!("List requires exactly 1 child, got {}", children.len()));
    }
    
    let element_value = &children[0];
    
    // Create array with single example element
    Ok(Value::Array(vec![element_value.clone()]))
}
```

**Output example:** `[{...element...}]`

### ArrayMutationBuilder
Creates a JSON array with all elements (fixed size).

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // Array includes all children as elements
    // The number of children should match the array's fixed size
    
    // For arrays with many elements, might want to limit examples
    const MAX_ARRAY_EXAMPLES: usize = 3;
    
    let examples = if children.len() > MAX_ARRAY_EXAMPLES {
        children.into_iter().take(MAX_ARRAY_EXAMPLES).collect()
    } else {
        children
    };
    
    Ok(Value::Array(examples))
}
```

**Output example:** `[elem1, elem2, elem3, ...]`

### TupleMutationBuilder
Creates a JSON array with elements in exact positional order.

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // Tuple includes all children in their exact positions
    // The number of children should match the tuple's arity
    
    // Could validate expected arity here
    // let expected_arity = self.get_arity_from_schema(ctx);
    // if children.len() != expected_arity { ... }
    
    Ok(Value::Array(children))
}
```

**Output example:** `[field0_value, field1_value, field2_value]`

### StructMutationBuilder
Creates a JSON object with named fields.

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // Struct needs to recover field names from PathKinds
    let path_kinds = self.collect_children(ctx);
    
    if path_kinds.len() != children.len() {
        return Err(anyhow::anyhow!(
            "Mismatch between PathKinds ({}) and children ({})", 
            path_kinds.len(), 
            children.len()
        ));
    }
    
    let mut obj = serde_json::Map::new();
    
    // Zip PathKinds with values to reconstruct field mapping
    for (path_kind, value) in path_kinds.iter().zip(children.iter()) {
        match path_kind {
            PathKind::StructField { field_name, .. } => {
                obj.insert(field_name.clone(), value.clone());
            }
            _ => {
                // Unexpected PathKind for struct field
                return Err(anyhow::anyhow!("Expected StructField PathKind"));
            }
        }
    }
    
    Ok(Value::Object(obj))
}
```

**Output example:** `{"field1": value1, "field2": value2, "field3": value3}`

### EnumMutationBuilder
Creates variant-specific JSON structures. Most complex due to multiple variant types.

```rust
fn assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value> {
    // Enum is the most complex - needs to handle different variant types
    
    // Get the variant information from context or schema
    let variant_info = self.get_variant_info(ctx)?;
    
    match variant_info {
        VariantType::Unit(name) => {
            // Unit variant: just the name as string
            Ok(Value::String(name))
        }
        VariantType::Newtype(name) => {
            // Newtype variant: {"VariantName": inner_value}
            if children.len() != 1 {
                return Err(anyhow::anyhow!("Newtype variant expects 1 child"));
            }
            let mut obj = serde_json::Map::new();
            obj.insert(name, children[0].clone());
            Ok(Value::Object(obj))
        }
        VariantType::Tuple(name, arity) => {
            // Tuple variant: {"VariantName": [field0, field1, ...]}
            if children.len() != arity {
                return Err(anyhow::anyhow!(
                    "Tuple variant expects {} children, got {}", 
                    arity, 
                    children.len()
                ));
            }
            let mut obj = serde_json::Map::new();
            obj.insert(name, Value::Array(children));
            Ok(Value::Object(obj))
        }
        VariantType::Struct(name) => {
            // Struct variant: {"VariantName": {"field1": value1, ...}}
            // Similar to StructMutationBuilder but wrapped in variant name
            let path_kinds = self.collect_children(ctx);
            let mut fields = serde_json::Map::new();
            
            for (path_kind, value) in path_kinds.iter().zip(children.iter()) {
                if let PathKind::StructField { field_name, .. } = path_kind {
                    fields.insert(field_name.clone(), value.clone());
                }
            }
            
            let mut obj = serde_json::Map::new();
            obj.insert(name, Value::Object(fields));
            Ok(Value::Object(obj))
        }
    }
}
```

**Output examples:**
- Unit: `"VariantName"`
- Newtype: `{"VariantName": inner_value}`
- Tuple: `{"VariantName": [field0, field1]}`
- Struct: `{"VariantName": {"field1": value1, "field2": value2}}`

## Key Design Decisions

### Why Positional Ordering?
- Eliminates arbitrary string labels for child identification
- Reduces HashMap overhead
- Each builder defines its own positional convention
- ProtocolEnforcer preserves order when recursing

### How Named Fields Work
- Struct and Enum builders call `collect_children(ctx)` again in `assemble_from_children`
- This returns the same PathKinds with embedded field names
- Builders zip PathKinds with values to recover the field mapping
- No information is lost despite using positional Vec<Value>

### Mutation Status Handling
- Builders NEVER determine mutation status
- Builders ONLY assemble examples from children
- ProtocolEnforcer is the sole authority for:
  - Determining mutation status (Mutatable/NotMutatable/PartiallyMutatable)
  - Generating mutation_status_reason strings
  - Aggregating child statuses

## Migration Checklist
When migrating a builder to the new pattern:

1. ✅ Update `collect_children()` to return `Vec<PathKind>` instead of `Vec<(String, RecursionContext)>`
2. ✅ Implement `assemble_from_children(&self, ctx: &RecursionContext, children: Vec<Value>) -> Result<Value>`
3. ✅ Remove any mutation status determination logic
4. ✅ Remove any path creation logic
5. ✅ Ensure `include_child_paths()` returns the correct boolean
6. ✅ Test that examples assemble correctly

## Common Pitfalls to Avoid

1. **Don't determine mutation status** - That's ProtocolEnforcer's job
2. **Don't create MutationPathInternal** - Builders only return examples
3. **Don't forget to validate child count** - Each type expects specific numbers
4. **Don't lose field names** - Struct/Enum must recover them from PathKinds
5. **Don't assume child order** - Always use the order YOU defined in collect_children()