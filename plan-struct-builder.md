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
    // The new require_registry_schema() returns Result with standard error
    let schema = ctx.require_registry_schema()?;

    // Extract properties from schema - use proper schema methods
    let Some(properties) = schema.get_properties() else {
        // Missing properties field indicates schema error (not empty struct)
        return Err(Error::SchemaProcessing {
            message: format!("Struct schema missing 'properties' field for type: {}", ctx.type_name()),
            type_name: Some(ctx.type_name().to_string()),
            operation: Some("extract_struct_properties".to_string()),
            details: None,
        }.into());
    };

    // Empty properties map is valid (empty struct/marker struct)
    if properties.is_empty() {
        return Ok(vec![]); // Valid marker struct
    }

    // Convert each field into a PathKind
    let mut children = Vec::new();
    for (field_name, field_schema) in properties {
        // Extract field type or return error immediately - no fallback
        // Note: SchemaField::extract_field_type handles complex schemas with $ref
        let Some(field_type) = SchemaField::extract_field_type(field_schema) else {
            return Err(Error::SchemaProcessing {
                message: format!("Failed to extract type for field '{}' in struct '{}'", field_name, ctx.type_name()),
                type_name: Some(ctx.type_name().to_string()),
                operation: Some("extract_field_type".to_string()),
                details: Some(format!("Field: {}", field_name)),
            }.into());
        };

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
- Missing schema → Handled automatically by `ctx.require_registry_schema()?`
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

## Design Review Skip Notes

### DC-1: TypeKind dispatch update may already be implemented differently - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Step 4: Update TypeKind Dispatch
- **Issue**: Step 4 of the plan proposes changing TypeKind::Struct dispatch from `StructMutationBuilder.build_paths(ctx, builder_depth)` to `self.builder().build_paths(ctx, builder_depth)`, but the current code in type_kind.rs line 172 already shows this exact pattern is in use. This suggests the plan may be based on outdated code state.
- **Reasoning**: The finding misrepresents the current code state. The plan correctly identifies a needed change, and this finding would incorrectly prevent that improvement from being implemented. The proposed change is necessary for architectural coherence by ensuring all variants use the consistent `self.builder()` pattern, which includes proper `ProtocolEnforcer` wrapping for migrated builders.
- **Decision**: User elected to accept the rejection and continue

### DP-1: New collect_children() logic duplicates existing extract_properties() pattern - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Step 3.2: Implement collect_children()
- **Issue**: The proposed `collect_children()` method (lines 78-95) implements very similar field iteration logic to the existing `extract_properties()` method (lines 452-466) in struct_builder.rs, but with different error handling and data structures. This creates conceptual duplication even though the methods will be used in different contexts.
- **Reasoning**: This is not conceptual duplication requiring consolidation - it's the natural evolution from old to new architecture during a planned migration. The temporary similarity will resolve automatically when the migration completes and the old code is removed. The `extract_properties()` method is part of the OLD builder implementation that will be removed during migration, while `collect_children()` is part of the NEW protocol implementation that replaces the old system.
- **Decision**: User elected to accept the rejection and continue

### AC-1: Migration plan correctly represents complete atomic change - **Verdict**: CONFIRMED ✅
- **Status**: APPROVED - Plan structure is appropriate
- **Location**: Success Criteria
- **Issue**: The plan appropriately structures the migration as a complete, indivisible change from the old ExampleBuilder pattern to the new ProtocolEnforcer pattern. The success criteria ensure all parts of the migration are complete before considering it successful.
- **Reasoning**: This finding correctly identifies a positive aspect of the codebase planning. The migration plan demonstrates thorough analysis, proven strategy, atomic design, and quality assurance. The plan is well-designed and follows best practices for atomic changes in this codebase.
- **Decision**: User acknowledged the positive assessment

## Final Cleanup Checklist
- [ ] Remove ExampleBuilder import
- [ ] Delete build_schema_example() method
- [ ] Delete static helper methods
- [ ] Remove unused imports
- [ ] Remove all NotMutableReason references
- [ ] Remove all depth checking code
- [ ] Remove mutation status propagation logic