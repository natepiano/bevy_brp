# Plan: Remove Redundant Mutation Precheck

## Problem Statement

The current implementation performs redundant recursion:
1. **First pass**: `type_supports_mutation_precheck()` recursively traverses the type tree to check mutability
2. **Second pass**: Path building traverses the same tree again to build mutation paths

This is inefficient and unnecessarily complex. We're essentially asking "can we mutate this?" then immediately asking "how do we mutate this?" - when we could just try to build mutation paths and handle failures as they occur.

## Current Flow (Redundant)

```
Container Type → Precheck (recurse to inner) → Check Value type traits → Return bool
     ↓
Path Building → Recurse to inner again → Build paths or NotMutatable
```

## Proposed Flow (Efficient)

```
Container Type → Try building paths → Recurse once → Value type checks traits inline → Return paths or NotMutatable
```

## Implementation Strategy

### Phase 1: Remove Precheck Infrastructure

**File: `mutation_path_builders.rs`**

1. **Delete these methods entirely**:
   - `type_supports_mutation()` (lines 239-241)
   - `type_supports_mutation_with_depth()` (lines 244-339)
   
   These are the redundant precheck methods that duplicate work.

2. **Keep these methods** (they're still needed for detailed error messages):
   - `type_supports_mutation_detailed()` (lines 342-345)
   - `type_supports_mutation_with_depth_detailed()` (lines 347-447)

### Phase 2: Remove Precheck Calls from Path Builders

**File: `mutation_path_builders.rs` - `TypeKind::build_paths`** (lines 471-489)

Remove the entire precheck block:
```rust
// DELETE THIS ENTIRE BLOCK
match self {
    Self::List | Self::Array | Self::Map | Self::Option => {
        if !ctx.type_supports_mutation(ctx.type_name()) {
            return Ok(vec![Self::build_not_mutatable_path(ctx)]);
        }
    }
    Self::Value => {
        if !ctx.value_type_has_serialization(ctx.type_name()) {
            return Ok(vec![Self::build_not_mutatable_path(ctx)]);
        }
    }
    _ => {}
}
```

## DESIGN REVIEW AGREEMENT: SIMPLIFICATION-001 - Simplify Value type builder approach

**Plan Status**: ✅ APPROVED - Ready for future implementation

### Problem Addressed
The plan creates a separate `ValueMutationBuilder` when the logic could be integrated into the existing dispatch mechanism. The `DefaultMutationBuilder` is generic and used for multiple type kinds, but Value type checking can be handled directly in the dispatch.

### Solution Overview
Instead of creating a separate `ValueMutationBuilder`, handle Value types with inline checking directly in the `TypeKind::build_paths` dispatch. This eliminates unnecessary builder separation while maintaining the same inline checking behavior.

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Update TypeKind::build_paths dispatch** (lines 471-489):
```rust
impl MutationPathBuilder for TypeKind {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        match self {
            Self::Array => ArrayMutationBuilder.build_paths(ctx),
            Self::Enum => EnumMutationBuilder.build_paths(ctx),
            Self::Struct => StructMutationBuilder.build_paths(ctx),
            Self::Tuple | Self::TupleStruct => TupleMutationBuilder.build_paths(ctx),
            
            // Handle Value types with inline checking
            Self::Value => {
                if !ctx.value_type_has_serialization(ctx.type_name()) {
                    return Ok(vec![Self::build_not_mutatable_path(ctx)]);
                }
                DefaultMutationBuilder.build_paths(ctx)
            }
            
            // Handle container types with inline checking  
            Self::List | Self::Map | Self::Option => {
                if !ctx.type_supports_mutation(ctx.type_name()) {
                    return Ok(vec![Self::build_not_mutatable_path(ctx)]);
                }
                DefaultMutationBuilder.build_paths(ctx)
            }
        }
    }
}
```

### Integration with Existing Plan
- **Dependencies**: Simplifies Phase 3 implementation
- **Impact on existing sections**: Removes the need for Phase 3.1 Value Type Builder section
- **Related components**: Maintains consistency with other inline checking approaches

### Implementation Priority: Medium

### Verification Steps
1. Compile successfully after changes
2. Verify Value types with missing traits return NotMutatable
3. Verify Value types with traits build normal paths
4. Check no additional builder type is needed

---
**Design Review Decision**: Approved for inclusion in plan
**Next Steps**: Code changes ready for implementation when needed

### Phase 3: Update Individual Builders to Check Inline

#### 3.1 Value Type Handling (Simplified)
Value types are handled directly in the `TypeKind::build_paths` dispatch with inline serialization checking. No separate builder needed.

## DESIGN REVIEW AGREEMENT: IMPLEMENTATION-001 - Add Map container type inline checking example

**Plan Status**: ✅ APPROVED - Ready for future implementation (Map only, Option excluded)

### Problem Addressed
The plan provides example for List/Array inline checking but doesn't specify how Map types should do inline checking. This creates inconsistent guidance and could lead to suboptimal implementations when Map container types are eventually given dedicated builders instead of relying on the current DefaultMutationBuilder.

### Solution Overview
Add concrete implementation example for Map inline checking strategy, following the same pattern as List/Array. Note: Option type is excluded from this recommendation as it will be treated as a regular enum in a separate plan.

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Add Map Type Inline Checking**:
```rust
impl MutationPathBuilder for MapMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        // Extract value type inline instead of precheck
        let Some(value_type) = Self::extract_map_value_type(schema) else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        // Check value type mutability inline
        let value_ctx = MutationPathContext::new(
            RootOrField::root(&value_type),
            ctx.registry,
            None,
        );
        
        let Some(value_schema) = ctx.get_type_schema(&value_type) else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        let value_kind = TypeKind::from_schema(value_schema, &value_type);
        
        // For Value types, check serialization inline
        if matches!(value_kind, TypeKind::Value) 
            && !ctx.value_type_has_serialization(&value_type) {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        }
        
        // Value type is mutable, proceed with normal map path building
        // Create dynamic key examples like {"example_key": value_example}
        // ... rest of map-specific path building logic
    }
}
```

### Integration with Existing Plan
- **Dependencies**: Uses the same inline checking pattern as List/Array
- **Impact on existing sections**: Completes the container type examples in Phase 3.2
- **Related components**: Aligns Map handling with established List/Array pattern

### Implementation Priority: Medium

### Verification Steps
1. Test Map types with various value types
2. Verify inline checking prevents redundant recursion
3. Ensure error messages are clear for non-mutatable map values
4. Check performance improvement for nested maps

---
**Design Review Decision**: Approved for inclusion in plan (Map only)
**Next Steps**: Code changes ready for implementation when needed

#### 3.2 Container Builders (List, Array, Map)
These builders should:
1. Try to determine their inner type
2. Create a context for the inner type
3. Recursively call path building on inner type
4. If inner type returns NotMutatable paths, propagate that up

Note: Option type will be handled as a regular enum in a separate plan, not as a special container type.

Example for List/Array:
```rust
impl MutationPathBuilder for ListMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        // Extract element type
        let Some(element_type) = Self::extract_list_element_type(schema) else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        // Create context for element and try building its paths
        let elem_ctx = MutationPathContext::new(
            RootOrField::root(&element_type),
            ctx.registry,
            None,
        );
        
        // Get element type's schema and kind
        let Some(elem_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        };
        
        let elem_kind = TypeKind::from_schema(elem_schema, &element_type);
        let elem_paths = elem_kind.build_paths(&elem_ctx)?;
        
        // Check if element is mutatable based on returned paths
        let elem_mutatable = elem_paths.iter().any(|p| 
            !matches!(p.path_kind, MutationPathKind::NotMutatable)
        );
        
        if !elem_mutatable {
            // Element can't be mutated, so neither can the list
            return Ok(vec![TypeKind::build_not_mutatable_path(ctx)]);
        }
        
        // Element is mutatable, build normal list paths
        // ... existing list path building logic
    }
}
```

## DESIGN REVIEW AGREEMENT: DESIGN-001 - Specify partial tuple mutability handling

**Plan Status**: ✅ APPROVED - Ready for future implementation

### Problem Addressed
The plan removes precheck but doesn't address how individual tuple elements with mixed mutability states should be handled during inline checking. The current binary approach (all-or-nothing) doesn't handle realistic scenarios where tuples contain mixed mutability types like `(String, Entity)` where `String` is mutable but `Entity` might not be through BRP.

### Solution Overview
For tuple types, inline checking must handle mixed mutability states:

1. **Per-Element Validation**: Each tuple element gets validated independently during path building
2. **Partial Success Handling**: If some elements are mutable and others aren't, return paths for mutable elements and NotMutatable paths for immutable ones
3. **Root Propagation Logic**: Only mark the root tuple as NotMutatable if ALL elements are non-mutable
4. **Preserve Element Paths**: Individual element paths should still be generated even if marked NotMutatable for debugging

This approach allows partial tuple mutation where possible while maintaining clear error reporting.

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Add new MutationPathKind variant**:
```rust
pub enum MutationPathKind {
    // ... existing variants
    PartiallyMutable, // New variant for mixed mutability tuples
}
```

- **Update tuple element building** (lines 1212-1225):
```rust
fn build_tuple_element_path(
    ctx: &MutationPathContext<'_>,
    index: usize,
    element_info: &Value,
    path_prefix: &str,
    parent_type: &BrpTypeName,
) -> Option<MutationPathInternal> {
    let element_type = SchemaField::extract_field_type(element_info)?;
    
    // Create element context for recursive validation
    let elem_ctx = MutationPathContext::new(
        RootOrField::root(&element_type),
        ctx.registry,
        None,
    );
    
    // Inline checking: try building element paths and check result
    match element_type.validate_inline_mutation(&elem_ctx) {
        MutationSupport::Supported => {
            // Build normal mutable path with proper metadata
            Some(MutationPathInternal {
                path_kind: MutationPathKind::TupleElement { 
                    index, 
                    parent_type: parent_type.clone(),
                    mutability_state: ElementMutability::Mutable,
                },
                // ...
            })
        },
        not_supported => {
            // Build NotMutatable path with detailed context preservation
            Some(MutationPathInternal {
                path_kind: MutationPathKind::NotMutatable,
                example: json!({
                    "NotMutatable": format!("{not_supported}"),
                    "agent_directive": "Element type cannot be mutated through BRP",
                    "element_index": index,
                    "parent_type": parent_type,
                }),
                // ...
            })
        }
    }
}
```

- **Enhanced propagation logic**:
```rust
fn propagate_tuple_mixed_mutability(paths: &mut [MutationPathInternal]) {
    let has_root = paths.iter().any(|p| p.path.is_empty());
    
    if has_root {
        let (mutable_count, immutable_count) = paths
            .iter()
            .filter(|p| !p.path.is_empty())
            .fold((0, 0), |(mut_count, immut_count), path| {
                match path.path_kind {
                    MutationPathKind::NotMutatable => (mut_count, immut_count + 1),
                    _ => (mut_count + 1, immut_count),
                }
            });
            
        // Root mutation strategy based on element composition
        if let Some(root) = paths.iter_mut().find(|p| p.path.is_empty()) {
            match (mutable_count, immutable_count) {
                (0, _) => {
                    // All elements immutable - root cannot be mutated
                    root.path_kind = MutationPathKind::NotMutatable;
                },
                (_, 0) => {
                    // All elements mutable - keep existing mutable root path
                },
                (_, _) => {
                    // Mixed mutability - root cannot be replaced, but individual elements can be mutated
                    root.path_kind = MutationPathKind::PartiallyMutable;
                    root.example = json!({
                        "PartialMutation": format!("Some elements of {} are immutable", root.type_name),
                        "agent_directive": "Use individual element paths - root replacement not supported",
                        "mutable_elements": mutable_count,
                        "immutable_elements": immutable_count
                    });
                }
            }
        }
    }
}
```

### Integration with Existing Plan
- **Dependencies**: Builds on Phase 3 inline checking infrastructure
- **Impact on existing sections**: Enhances Phase 4 tuple builder update with proper mixed mutability handling
- **Related components**: Affects tuple and tuple struct builders

### Implementation Priority: High

### Verification Steps
1. Test with mixed mutability tuples like `(Transform, Handle<Mesh>)`
2. Verify individual element paths are preserved
3. Check root path correctly reflects partial mutability
4. Ensure backward compatibility for existing tuple handling

---
**Design Review Decision**: Approved for inclusion in plan
**Next Steps**: Code changes ready for implementation when needed

### Phase 4: Update Tuple Builder

The tuple builder (lines 1212-1225) currently calls `ctx.type_supports_mutation()`. Update it to check mutability inline:

```rust
fn build_tuple_element_path(
    ctx: &MutationPathContext<'_>,
    index: usize,
    element_info: &Value,
    path_prefix: &str,
    parent_type: &BrpTypeName,
) -> Option<MutationPathInternal> {
    let element_type = SchemaField::extract_field_type(element_info)?;
    
    // Instead of calling type_supports_mutation, build path and check result
    let elem_ctx = MutationPathContext::new(
        RootOrField::root(&element_type),
        ctx.registry,
        None,
    );
    
    let Some(elem_schema) = ctx.get_type_schema(&element_type) else {
        // Element type not in registry - not mutatable
        return Some(MutationPathInternal {
            path: /* ... */,
            example: json!({"NotMutatable": /* ... */}),
            path_kind: MutationPathKind::NotMutatable,
            /* ... */
        });
    };
    
    // Check element type kind
    let elem_kind = TypeKind::from_schema(elem_schema, &element_type);
    
    // For Value types, check serialization inline
    if matches!(elem_kind, TypeKind::Value) && !ctx.value_type_has_serialization(&element_type) {
        let detailed_support = ctx.type_supports_mutation_detailed(&element_type);
        return Some(MutationPathInternal {
            /* ... NotMutatable path ... */
        });
    }
    
    // Element seems mutatable, build normal path
    /* ... existing path building ... */
}
```

## Benefits of This Approach

1. **Eliminates Redundant Recursion**: We traverse the type tree only once
2. **Simpler Logic**: Mutability checking happens inline where it's needed
3. **Better Performance**: Roughly 50% reduction in recursive calls for nested types
4. **Clearer Code Flow**: Each builder is responsible for its own mutability checking
5. **Same Results**: The output remains identical - we just get there more efficiently

## Migration Strategy

1. **Keep `type_supports_mutation_detailed`**: Still needed for error messages
2. **Atomic Migration**: All changes applied together in single implementation
3. **Test Coverage**: Existing tests should pass without modification

## DESIGN REVIEW AGREEMENT: TYPE-SYSTEM-001 - Error handling enum instead of runtime string checks

**Plan Status**: ✅ APPROVED - Ready for future implementation

### Problem Addressed
The codebase uses a mix of boolean checks and structured `MutationSupport` enum error handling. Boolean checks lose valuable error context, and the `from_paths` method falls back to string-based inference which creates generic error messages.

### Solution Overview  
Replace boolean mutation checks with structured error handling using MutationSupport enum variants for all mutation failure cases. This eliminates runtime string-based error handling in favor of compile-time type safety.

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Lines to change**: TypeKind::build_paths method (lines 471-489), add new MutationResult enum  
- **Current code pattern**: 
```rust
match self {
    Self::List | Self::Array | Self::Map | Self::Option => {
        if !ctx.type_supports_mutation(ctx.type_name()) {
            return Ok(vec![Self::build_not_mutatable_path(ctx)]);
        }
    }
    Self::Value => {
        if !ctx.value_type_has_serialization(ctx.type_name()) {
            return Ok(vec![Self::build_not_mutatable_path(ctx)]);
        }
    }
    _ => {}
}
```
- **New code implementation**:
```rust  
pub enum MutationResult {
    Success(Vec<MutationPathInternal>),
    NotMutatable(MutationSupport),
}

impl TypeKind {
    fn try_build_paths(&self, ctx: &MutationPathContext<'_>) -> MutationResult {
        match self.validate_mutation_capability(ctx) {
            MutationSupport::Supported => MutationResult::Success(self.build_paths(ctx)?),
            not_supported => MutationResult::NotMutatable(not_supported)
        }
    }
}
```

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`
- **Lines to change**: TypeInfo::from_schema method, remove MutationSupport::from_paths fallback
- **Current code pattern**: Uses from_paths to infer mutation support from generated paths
- **New code implementation**: Use MutationResult directly to preserve precise error context

### Integration with Existing Plan
- **Dependencies**: Should be done after removing precheck infrastructure (Phase 1)
- **Impact on existing sections**: Enhances Phase 2 by replacing boolean checks with structured types
- **Related components**: All mutation builders benefit from clearer error handling

### Implementation Priority: High

### Verification Steps
1. Compile successfully after changes
2. Run existing tests
3. Verify error messages maintain quality and specificity
4. Check that MutationSupport enum variants are used consistently

---
**Design Review Decision**: Approved for inclusion in plan
**Next Steps**: Code changes ready for implementation when needed

## Success Criteria

1. All existing tests pass
2. No change in JSON output for any type
3. Reduced code complexity (fewer lines)
4. Performance improvement measurable for deeply nested types
5. Clearer separation of concerns in builders

## Design Review Skip Notes

### TYPE-SYSTEM-002: Encode mutation state machine in types
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Description**: Proposed using phantom types to encode mutation validation states
- **Reason**: Investigated and Rejected - Current builder pattern with inline validation already achieves the same goals without added complexity. Goes against the plan's direction of eliminating redundant validation.

### TYPE-SYSTEM-003: Missing mutation capability validation API design
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Description**: Proposed creating a public API for external mutation validation
- **Reason**: User decision - not needed. The current `type_supports_mutation` is private/internal only, not a public API. No external validation API is being removed or needed.

### TYPE-SYSTEM-004: Missing error context propagation in inline validation
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Description**: Proposed enhancing error context preservation during inline validation
- **Reason**: Investigated and found redundant - current implementation already preserves comprehensive error context through `build_not_mutatable_path` which calls `type_supports_mutation_detailed`. Both current and proposed approaches use identical error handling mechanisms.

### IMPLEMENTATION-002: Missing rollback strategy for incremental migration
- **Status**: SKIPPED
- **Category**: IMPLEMENTATION
- **Description**: Proposed adding rollback strategy for gradual migration approach
- **Reason**: User decision - this is an atomic change, not gradual. Remove references to incremental approach.

### DESIGN-002: Inconsistent container type handling strategy
- **Status**: SKIPPED
- **Category**: DESIGN
- **Description**: Proposed adding architectural decision section to explain container type categorization
- **Reason**: User decision - add note about Option type future plans. Option is currently handled inconsistently (as container but with enum-style extraction) because we plan to remove wrapper type special case handling for Option so it will always be treated as a regular enum. This change hasn't been implemented yet, so Option should be kept as-is except for changes necessary for this precheck removal plan.

## Risk Mitigation

- **Preserve detailed methods**: Keep error message generation intact
- **Extensive testing**: Run against all Bevy component types
- **Atomic implementation**: All changes applied together to avoid inconsistent intermediate states

## Example: Transform Component

**Current flow**:
1. Precheck Transform → recurse to Vec3 fields → check each Vec3 has Serialize → return true
2. Build paths → recurse to Vec3 fields again → build paths

**New flow**:
1. Build Transform paths → encounter Vec3 field → Vec3 builder checks Serialize inline → returns paths

This eliminates the duplicate traversal of Transform's three Vec3 fields.