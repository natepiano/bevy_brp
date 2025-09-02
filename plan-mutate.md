# Plan: Fix Mutation Support by Building from Paths

## Problem

The current BRP type schema system has inconsistent logic for determining mutation support:

1. **get_supported_operations()** assumes Components/Resources can mutate and adds `Mutate` upfront
2. **type_supports_mutation()** tries to validate this assumption but uses different logic
3. **Value types** (like `String`) with serialization support are incorrectly excluded from mutation
4. **Tuple structs** (like `Text`) with serializable fields get marked as non-mutatable

This causes types like `bevy_ui::widget::text::Text` to be incorrectly auto-passed when they should be testable.

## Solution Architecture

Instead of predicting mutation support, **build mutation paths first** and derive supported operations from actual results:

```
1. Build mutation paths (recursive, handles all nested types)
2. Analyze built paths → count mutatable vs NotMutatable  
3. If any mutatable paths exist → earn "Mutate" operation
4. Update supported_operations based on proof-of-work
```

## Implementation Plan

## Implementation Order (CRITICAL)

**Phase 1**: Prepare path-building foundation
1. Fix compilation errors in Value type logic 
2. Update `TypeKind::Value` mutation logic in `MutationPathBuilder::build_paths`

**Phase 2**: Modify operation assignment logic  
3. Update `get_supported_operations()` to remove upfront `Mutate` assignment (Step 1)
4. Add post-build mutation analysis to `from_schema()` (Step 2)

**Phase 3**: Clean up obsolete code
5. Remove `type_supports_mutation()` circular dependency (Step 3)

### Step 1: Remove Upfront Mutate Assignment

**File**: `mcp/src/brp_tools/brp_type_schema/type_info.rs`  
**Method**: `get_supported_operations()`

```diff
- if has_component {
-     operations.push(BrpSupportedOperation::Mutate);  // Remove assumption
-     // ...
- }
- if has_resource {
-     operations.push(BrpSupportedOperation::Mutate);  // Remove assumption  
-     // ...
- }
```

Start with minimal operations only:
- Always: `Query`
- Components: `Get` 
- Components with serialization: `Spawn`, `Insert`
- Resources with serialization: `Insert`

### Step 2: Add Post-Build Mutation Analysis

**File**: `mcp/src/brp_tools/brp_type_schema/type_info.rs`  
**Method**: `from_registry_schema()`

After mutation paths are built, add:

```rust
// After building mutation_paths, check if any are actually mutatable
let has_mutatable_paths = mutation_paths.values().any(|path| {
    !matches!(path.path_kind, MutationPathKind::NotMutatable)
});

// Earn mutation support based on actual capability
if has_mutatable_paths {
    supported_operations.push(BrpSupportedOperation::Mutate);
}
```

### Step 3: Remove type_supports_mutation Logic

**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`  
**Method**: `type_supports_mutation()`

Current hardcoded logic:
```rust
TypeKind::Value => {
    // Complex serialization checking logic
}
```

Replace with path-building approach:
- Remove `type_supports_mutation()` method entirely
- Let mutation path builders naturally determine mutability through schema inspection
- Remove circular dependency between path building and mutation support

### Step 4: Update Value Type Handling

**File**: `mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`

For Value types, determine mutability through **actual reflection support**:

```rust
TypeKind::Value => {
    // Build mutation paths - they will be NotMutatable if type lacks reflection
    // String with Serialize/Deserialize will get mutatable paths
    // RenderTarget without serialization will get NotMutatable paths
    // Let the schema determine the outcome
}
```

## Expected Results

### Before Fix:
- **String**: `supported_operations: ["query"]` (wrong)
- **Text**: Auto-passed as non-testable (wrong)  
- **RenderTarget**: `supported_operations: ["query"]` (correct)

### After Fix:
- **String**: `supported_operations: ["query", "mutate"]` (correct - has serialization)
- **Text**: Testable with mutation paths `["", ".0"]` (correct - String field is mutatable)
- **RenderTarget**: `supported_operations: ["query"]` (correct - no serialization)

## Testing Plan

1. **String Value Type**:
   - Should get `mutate` in supported_operations (has Serialize/Deserialize)
   - Should have mutatable paths

2. **Text Tuple Struct**:
   - Should NOT be auto-passed  
   - Should have mutation paths: `["", ".0"]`
   - `.0` path should be mutatable (points to String)

3. **RenderTarget Enum**:
   - Should NOT get `mutate` in supported_operations (no serialization)
   - Should have NotMutatable paths only

4. **Regression Testing**:
   - Components with mutation should still work
   - Resources with mutation should still work
   - Complex nested structures should still work

## Benefits

1. **Single Source of Truth**: Mutation paths determine supported operations
2. **Eliminates Duplication**: No parallel logic in multiple places
3. **Self-Consistent**: What we advertise matches what actually works
4. **Extensible**: New type patterns automatically work if paths can be built
5. **Debuggable**: Clear path from schema → paths → operations

## Design Review Skip Notes

### ⚠️ PREJUDICE WARNING - TYPE-SYSTEM-003: Stringly-typed phase ordering violates type safety principles
- **Status**: PERMANENTLY REJECTED
- **Category**: TYPE-SYSTEM
- **Description**: Recommendation to replace string-based phase descriptions with enum-based implementation phases and dependency enforcement
- **Reason**: Reviewer repeatedly suggested this despite prior rejections
- **Critical Note**: DO NOT SUGGEST THIS AGAIN - This recommendation has been permanently rejected due to reviewer repetition

### TYPE-SYSTEM-001: Replace missing method with proper trait-based dispatch
- **Status**: INVESTIGATED AND REJECTED
- **Category**: TYPE-SYSTEM
- **Description**: Recommendation to implement missing `value_type_has_serialization()` method in `MutationPathContext`
- **Reason**: Investigation found this contradicts the plan's core architectural goal of removing `type_supports_mutation` logic entirely. The missing method call should be removed, not implemented, to align with the new architecture.
- **Investigation Findings**: The recommendation would implement deprecated architecture that the plan explicitly aims to eliminate. The correct solution is to remove the problematic method call and let mutation path building determine mutability naturally.

### TYPE-SYSTEM-002: Replace mutation support boolean with enum state
- **Status**: INVESTIGATED AND REJECTED
- **Category**: TYPE-SYSTEM
- **Description**: Recommendation to replace `type_supports_mutation` boolean return with `MutationSupportResult` enum
- **Reason**: Investigation found this directly contradicts the plan's Step 3 goal to remove the `type_supports_mutation()` method entirely. Adding complexity to a method that should be eliminated represents over-engineering in the wrong direction.
- **Investigation Findings**: The plan's new post-build analysis architecture already provides better error context through `NotMutatable` paths without maintaining circular dependencies. The enum would add theoretical elegance to deprecated code while the practical solution already exists in the new architecture.

### TYPE-SYSTEM-004: Missing mutation operation result enum violates error handling principles
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Description**: Recommendation to replace string-based mutation result examples with proper enum that models all possible mutation analysis outcomes
- **Reason**: User decision - vastly over engineering

### DESIGN-003: Missing atomic rollback strategy for partial implementation failures
- **Status**: SKIPPED
- **Category**: DESIGN
- **Description**: Recommendation to add rollback strategy section that documents how to safely revert each phase if issues are discovered during testing
- **Reason**: User decision - making an atomic change here

### IMPLEMENTATION-003: Missing error context propagation in mutation path analysis
- **Status**: SKIPPED
- **Category**: IMPLEMENTATION
- **Description**: Recommendation to enhance mutation analysis logic to include error context and logging when types don't support mutation
- **Reason**: User decision - over engineering

### DESIGN-002: Missing handling of intermediate mutation state during transition
- **Status**: SKIPPED
- **Category**: DESIGN
- **Description**: Recommendation to add transition safety measures to prevent breaking existing functionality during implementation
- **Reason**: User decision - implementing this as an atomic change, no intermediate transition state needed

## DESIGN REVIEW AGREEMENT: IMPLEMENTATION-001 - Value type mutation logic contradicts plan goals

**Plan Status**: ✅ APPROVED - Ready for future implementation

### Problem Addressed
The plan shows `TypeKind::Value` as non-mutatable, but the goal is to make serializable value types (like `String`) mutatable

### Solution Overview  
Implement proper value type mutation support with serialization checking at the individual mutation evaluation level

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Lines to change**: 276-301 (TypeKind::Value match arm)
- **Additional**: Add missing `value_type_has_serialization` method to `MutationPathContext`

**Current code pattern**: 
```rust
Self::Value => {
    // Value types (opaque types in Bevy's reflection system) cannot be mutated
    let reason = "Opaque type - cannot be mutated...";
    // Always returns NotMutatable paths
}
```

**New code implementation**:
```rust  
Self::Value => {
    // Check if this value type has serialization support
    if ctx.value_type_has_serialization(ctx.type_name()) {
        // Serializable value types like String can be mutated
        DefaultMutationBuilder.build_paths(ctx)
    } else {
        // Non-serializable value types remain non-mutatable
        let reason = "Opaque type without serialization support - cannot be mutated...";
        // Build NotMutatable path as before
    }
}

// Add to MutationPathContext impl:
fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
    self.get_type_schema(type_name)
        .map(|schema| {
            let reflect_types = Self::extract_reflect_types(schema);
            reflect_types.contains(&ReflectTrait::Serialize) && 
            reflect_types.contains(&ReflectTrait::Deserialize)
        })
        .unwrap_or(false)
}
```

### Integration with Existing Plan
- **Dependencies**: Must be implemented before Phase 2 changes to avoid compilation errors
- **Impact on existing sections**: Enables proper aggregation from individual mutation paths to top-level type operations
- **Related components**: Works with MutationState::from_paths() aggregation and final supported_operations determination

### Implementation Priority: High

### Verification Steps
1. Compile successfully after changes
2. Test String type gets mutatable paths (not NotMutatable)
3. Test Text(String) type properly aggregates String field mutability to type-level mutation support
4. Verify non-serializable Value types still get NotMutatable paths

---
**Design Review Decision**: Approved for inclusion in plan on current date  
**Next Steps**: Code changes ready for implementation when needed

## DESIGN REVIEW AGREEMENT: IMPLEMENTATION-002 - Missing import and method implementation

**Plan Status**: ✅ APPROVED - Ready for future implementation

### Problem Addressed
The code references `ReflectTrait` and `SchemaField` but these may not be in scope where the missing method needs to be implemented

### Solution Overview  
Add required imports and implement the missing `value_type_has_serialization` method to match the structure already defined in IMPLEMENTATION-001

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Lines to change**: Add to `MutationPathContext` impl block
- **Additional**: Ensure proper imports are available

**Current code pattern**: `// Missing implementation`

**New code implementation**:
```rust  
// Add to MutationPathContext impl block to match IMPLEMENTATION-001:
fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
    use super::response_types::ReflectTrait;
    
    self.get_type_schema(type_name)
        .map(|schema| {
            let reflect_types = schema
                .get_field(SchemaField::ReflectTypes)
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .filter_map(|s| s.parse::<ReflectTrait>().ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            
            reflect_types.contains(&ReflectTrait::Serialize) && 
            reflect_types.contains(&ReflectTrait::Deserialize)
        })
        .unwrap_or(false)
}
```

### Integration with Existing Plan
- **Dependencies**: Must be implemented alongside IMPLEMENTATION-001 changes
- **Impact on existing sections**: Provides the missing method required by the Value type mutation logic
- **Related components**: Used by TypeKind::Value handling in IMPLEMENTATION-001

### Implementation Priority: High

### Verification Steps
1. Compile successfully after adding the method
2. Verify method correctly identifies String as having serialization traits
3. Verify method correctly identifies RenderTarget as lacking serialization traits

---
**Design Review Decision**: Approved for inclusion in plan on current date  
**Next Steps**: Code changes ready for implementation when needed

## DESIGN REVIEW AGREEMENT: SIMPLIFICATION-001 - Over-engineered mutation state enum with unused variants

**Plan Status**: ✅ APPROVED - Ready for future implementation

### Problem Addressed
The `MutationState` enum defines three variants (`All`, `Some`, `None`) but the logic only cares about "has any mutatable paths" vs "has no mutatable paths"

### Solution Overview  
Simplify the enum to only distinguish between "supported" and "not supported" since the distinction between "all mutatable" and "some mutatable" doesn't affect behavior

### Required Code Changes

#### Files to Modify:
**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`
- **Lines to change**: MutationState enum definition and from_paths method
- **Additional**: Update usage in from_schema method

**Current code pattern**: 
```rust
#[derive(Debug, Clone, PartialEq)]
enum MutationState {
    /// All mutation paths are mutatable
    All,
    /// Some mutation paths are mutatable, others are not
    Some,
    /// No mutation paths are mutatable
    None,
}

impl MutationState {
    fn from_paths(paths: &HashMap<String, MutationPath>) -> Self {
        let mutatable_count = paths.values().filter(/*...*/).count();
        match (mutatable_count, paths.len()) {
            (0, _) => Self::None,
            (n, total) if n == total => Self::All,
            _ => Self::Some,
        }
    }
}
```

**New code implementation**:
```rust  
#[derive(Debug, Clone, PartialEq)]
enum MutationSupport {
    /// At least one mutation path is mutatable
    Supported,
    /// No mutation paths are mutatable  
    NotSupported,
}

impl MutationSupport {
    fn from_paths(paths: &HashMap<String, MutationPath>) -> Self {
        let has_mutatable = paths.values().any(|path| {
            !matches!(path.path_kind, MutationPathKind::NotMutatable)
        });
        if has_mutatable { Self::Supported } else { Self::NotSupported }
    }
}

// Update usage in from_schema method:
let mutation_support = MutationSupport::from_paths(&mutation_paths);
if matches!(mutation_support, MutationSupport::NotSupported) {
    supported_operations.retain(|op| *op != BrpSupportedOperation::Mutate);
}
```

### Integration with Existing Plan
- **Dependencies**: Should be implemented alongside other Step 2 changes in Phase 2
- **Impact on existing sections**: Simplifies the logic for determining mutation support without changing behavior
- **Related components**: Works with the post-build mutation analysis added in Step 2

### Implementation Priority: Low

### Verification Steps
1. Compile successfully after changes
2. Run existing tests to verify behavior unchanged
3. Verify both "all mutatable" and "some mutatable" cases still result in mutation support
4. Verify "no mutatable paths" case still removes mutation support

---
**Design Review Decision**: Approved for inclusion in plan on 2025-09-02  
**Next Steps**: Code changes ready for implementation when needed