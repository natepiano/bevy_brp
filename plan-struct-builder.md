# StructMutationBuilder Migration Plan

## Overview
This document provides the complete implementation plan for migrating StructMutationBuilder to the ProtocolEnforcer pattern. This migration is part of Phase 5b of removing ExampleBuilder and implementing enforced recursion protocol.

## Key Concept: How ProtocolEnforcer Handles Hardcoded Knowledge
**IMPORTANT**: ProtocolEnforcer naturally handles hardcoded knowledge (like Transform's Vec3 fields) through its early return pattern:
1. ProtocolEnforcer checks for hardcoded knowledge FIRST (line 35-37)
2. If knowledge exists, it returns immediately with the hardcoded example
3. No recursion to children happens - field paths are not generated
4. This is the CORRECT behavior - types with BRP-specific formats should use those formats

Therefore, StructMutationBuilder does NOT need special knowledge handling after migration.

## Reference Implementations to Study
Before starting, study these successfully migrated builders:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/array_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/list_builder.rs`

## Implementation Steps

### Step 1: Add Required Imports
```rust
use std::collections::HashMap;
use crate::error::Error;
use super::{PathKind, PathAction, MutationPathDescriptor};
```

### Step 2: Remove All These Sections Completely
1. **ALL depth checks** - Lines containing `depth.exceeds_limit()`
   - Line 34 in `build_paths()`
   - Line 83 in `build_schema_example()`
   - Line 445 in `build_struct_example_from_properties_with_context()`

2. **ALL NotMutable path creation methods**:
   - `build_not_mutable_path_from_support()` (lines ~287-299)
   - `build_not_mutatable_field_from_support()` (lines ~302-313) - note typo

3. **ALL mutation status handling**:
   - Remove `propagate_struct_immutability()` function (lines ~413-436)
   - Remove call to `Self::propagate_struct_immutability(&mut paths)` (line ~76)
   - Remove ALL `mutation_status` and `mutation_status_reason` field assignments

4. **ALL direct knowledge lookups**:
   - Remove ALL direct `BRP_MUTATION_KNOWLEDGE` accesses
   - Remove `NotMutableReason` imports

5. **Delete entire methods**:
   - `build_schema_example()` method
   - `build_struct_example_from_properties()` static method
   - `build_struct_example_from_properties_with_context()` helper

### Step 3: Implement New Protocol Methods

#### 3.1: Mark as Migrated
```rust
fn is_migrated(&self) -> bool {
    true
}
```

#### 3.2: Implement collect_children()
```rust
fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
    // Following the error pattern from migrated builders (array, list, map, set)
    // Always return errors for missing schemas, never empty vectors
    let Some(schema) = ctx.require_registry_schema() else {
        return Err(Error::SchemaProcessing {
            message:   format!("No schema found for struct type: {}", ctx.type_name()),
            type_name: Some(ctx.type_name().to_string()),
            operation: Some("collect_children".to_string()),
            details:   None,
        }.into());
    };

    // Extract properties from schema - use proper schema methods
    let Some(properties) = schema.get_properties() else {
        // Empty struct (no properties) is valid - return empty vector
        // This is different from missing schema which is an error
        return Ok(vec![]);
    };

    // Convert each field into a PathKind
    let mut children = Vec::new();
    for (field_name, field_schema) in properties {
        // Extract the field type name using proper schema field extraction
        // Note: SchemaField::extract_field_type handles complex schemas with $ref
        let field_type = SchemaField::extract_field_type(field_schema)
            .unwrap_or_else(|| BrpTypeName::from(field_name));

        // Create PathKind for this field
        let path_kind = PathKind::StructField {
            field_name: field_name.clone(),
            type_name: field_type.to_string(),
            parent_type: ctx.type_name().to_string(),
        };

        children.push(path_kind);
    }

    Ok(children)
}
```

#### 3.3: Implement assemble_from_children()
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value> {
    if children.is_empty() {
        // Valid case: empty struct with no fields (e.g., marker structs)
        return Ok(json!({}));
    }

    // Build the struct example from child examples
    let mut struct_obj = serde_json::Map::new();

    // MutationPathDescriptor for StructField is just the field name as a string
    // This follows the same pattern as other migrated builders (array, list, map, set)
    for (descriptor, example) in children {
        // descriptor derefs to the field name string
        // e.g., MutationPathDescriptor("position") -> "position"
        let field_name = (*descriptor).to_string();
        struct_obj.insert(field_name, example);
    }

    Ok(Value::Object(struct_obj))
}
```

#### 3.4: Update build_paths() to Return Error
```rust
fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth)
    -> Result<Vec<MutationPathInternal>> {
    Err(Error::InvalidState(format!(
        "StructMutationBuilder.build_paths() called directly for type '{}' - should be wrapped by ProtocolEnforcer",
        ctx.type_name()
    )))
}
```

### Step 4: Update TypeKind Dispatch
In `type_kind.rs`, update the Struct match arm:
```rust
// CHANGE FROM:
Self::Struct => StructMutationBuilder.build_paths(ctx, builder_depth),

// CHANGE TO:
Self::Struct => self.builder().build_paths(ctx, builder_depth),
```

### Step 5: Error Handling Updates
Follow the error handling patterns from migrated builders:
- Missing schema → Return `Err(Error::SchemaProcessing)` with descriptive message
- Empty struct (no properties) → Return `Ok(vec![])` - this is valid
- Failed type extraction → Use `SchemaField::extract_field_type` with fallback to field name
- Never return empty vectors for error cases - always return proper errors

## Testing Plan

### 1. Build Check
```bash
cargo build
```

### 2. Test Key Struct Types
- Simple struct with primitive fields
- Nested struct with complex fields
- Transform (has hardcoded knowledge - should use BRP format)
- Struct with optional fields
- Empty struct

### 3. Verify Behaviors
- ✅ Field paths are created (e.g., `.position.x`)
- ✅ Root path has complete struct example
- ✅ Transform uses hardcoded [x,y,z] format (from ProtocolEnforcer)
- ✅ No NotMutable paths created directly by builder
- ✅ No depth checking in builder
- ✅ No mutation status computation in builder

## Key Differences from Other Builders
1. **Field-based recursion**: Unlike arrays/lists that use indices, structs use named fields
2. **No child_path_action override**: Structs expose field paths (unlike Map/Set)
3. **Property extraction**: Must handle schema's "properties" field carefully

## Common Pitfalls to Avoid
1. **DON'T** try to handle hardcoded knowledge in the builder - ProtocolEnforcer does this
2. **DON'T** create NotMutable paths - return errors instead
3. **DON'T** check depth limits - ProtocolEnforcer handles this
4. **DON'T** compute mutation status - ProtocolEnforcer computes from children
5. **DON'T** forget to update TypeKind dispatch to use `self.builder()`

## Success Criteria
- [ ] All ExampleBuilder references removed
- [ ] build_paths() returns Error::InvalidState
- [ ] collect_children() returns PathKinds for all fields
- [ ] assemble_from_children() builds proper struct JSON
- [ ] TypeKind::Struct uses trait dispatch
- [ ] cargo build succeeds
- [ ] Test suite passes
- [ ] Transform type uses hardcoded format (verified through testing)

## Final Cleanup Checklist
- [ ] Remove ExampleBuilder import
- [ ] Delete build_schema_example() method
- [ ] Delete static helper methods
- [ ] Remove unused imports
- [ ] Remove all NotMutableReason references
- [ ] Remove all depth checking code
- [ ] Remove mutation status propagation logic