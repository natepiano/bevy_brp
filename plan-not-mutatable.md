# Interactive Plan: Centralized NotMutatable Path Generation and MutationStatus Handling

## Overview
Implement consistent `NotMutatable` path generation with a private helper function in `ProtocolEnforcer` and proper `MutationStatus` propagation for migrated builders (Default, Map, Set). Builders return `Error::NotMutatable` when they detect non-mutatable conditions, while ProtocolEnforcer converts these errors to paths and manages status propagation from children to parents.

## Current Problem
- Each builder has duplicate `build_not_mutatable_path` helper functions with identical logic
- `MutationStatus` assignment is inconsistent across builders
- Complex HashMap/HashSet key/element detection needs shared utility functions
- No mutation status propagation from children to parents
- Need a single source of truth for NotMutatable path construction

## COMPREHENSIVE CHANGES REQUIRED
- **1 helper function** - Private `build_not_mutatable_path()` in ProtocolEnforcer for consistent path generation
- **1 enum rename** - `ComplexMapKey` → `ComplexCollectionKey` to cover both HashMap and HashSet
- **1 error variant** - Add `Error::NotMutatable(NotMutatableReason)` to distinguish NotMutatable from real errors
- **1 trait extension** - Add `is_complex_type()` to `JsonFieldAccess` trait
- **1 status propagation handler** - `ProtocolEnforcer` mutation status propagation from children
- **0 new test components** - Existing infrastructure already covers all scenarios
- **2 builder updates** - Map and Set builders to detect complex keys/elements and return NotMutatable errors
- **1 mutation status algorithm** - Propagation rules: All/Mixed/Partial → Parent status

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for "go ahead" from user

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & INSTALL**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt && cargo install --path mcp
   ```
   Then inform user to run: `/mcp reconnect brp`

5. **VALIDATE**: Wait for user to confirm the build succeeded

6. **TEST** (if applicable): Run validation tests specific to that step

7. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

8. **PROCEED**: Move to next step only after confirmation
</Instructions>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### STEP 1: Rename MutationSupport Variant
**Status:** ✅ COMPLETED

**Objective:** Rename `ComplexMapKey` to `ComplexCollectionKey` to cover both HashMap and HashSet cases

**Note:** Already completed - the enum has been renamed to `NotMutatableReason` and the variant has been renamed to `ComplexCollectionKey`.

### STEP 1a: Add NotMutatable Error Variant
**Status:** ✅ COMPLETED

**Objective:** Add a NotMutatable variant to the Error enum to distinguish between actual errors and NotMutatable conditions

**Changes to make:**
1. Add `NotMutatable(NotMutatableReason)` variant to Error enum
2. Add helper method to check if an error is NotMutatable

**Files to modify:**
- `mcp/src/error.rs`

**Code changes:**
```rust
// In error.rs
pub enum Error {
    // ... existing variants ...
    
    /// Type cannot be mutated for a specific reason
    NotMutatable(NotMutatableReason),
    
    // ... other variants ...
}

impl Error {
    /// Check if this error represents a NotMutatable condition
    pub fn as_not_mutatable(&self) -> Option<&NotMutatableReason> {
        match self {
            Error::NotMutatable(reason) => Some(reason),
            _ => None,
        }
    }
}
```

**Expected outcome:**
- Clear distinction between actual errors and NotMutatable conditions
- Type-safe way to return NotMutatable from builders
- ProtocolEnforcer can identify and handle NotMutatable specially

### STEP 2: Add NotMutatable Helper Function  
**Status:** ✅ COMPLETED

**Objective:** Add private helper function in ProtocolEnforcer for consistent NotMutatable path generation

**Changes to make:**
1. Add `build_not_mutatable_path()` as a private method in `ProtocolEnforcer`
2. Function automatically determines context description from `PathKind`
3. Provides consistent formatting for all NotMutatable paths
4. Only ProtocolEnforcer can create NotMutatable paths - builders return errors

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Code changes:**
```rust
// In protocol_enforcer.rs
impl ProtocolEnforcer {
    /// Build a NotMutatable path with consistent formatting (private to ProtocolEnforcer)
    /// 
    /// This centralizes NotMutatable path creation, ensuring only ProtocolEnforcer
    /// can create these paths while builders simply return Error::NotMutatable.
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        reason: NotMutatableReason,
    ) -> MutationPathInternal {
        // Automatically determine context description from path_kind
        let context_description = match &ctx.path_kind {
            PathKind::StructField { .. } => "field",
            PathKind::TupleElement { .. } => "tuple element",
            PathKind::ArrayElement { .. } => "array element",
            PathKind::EnumVariant { .. } => "enum variant",
            PathKind::MapKey => "map key",
            PathKind::SetElement => "set element",
            PathKind::RootValue(_) => "type",
            _ => "element",
        };
        
        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example: json!({
                "NotMutatable": format!("{reason}"),
                "agent_directive": format!("This {context_description} cannot be mutated - {reason}")
            }),
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason: Option::<String>::from(&reason),
        }
    }
}
```

**Expected outcome:**
- Single helper function eliminates code duplication
- Consistent NotMutatable path formatting
- Automatic context description based on PathKind
- Builders cannot accidentally create NotMutatable paths directly

### STEP 3: Add ProtocolEnforcer Helper Method
**Status:** ✅ COMPLETED

**Objective:** Add helper method to ProtocolEnforcer for handling NotMutatable errors cleanly

**Changes to make:**
1. Add `handle_assemble_error()` method to process errors from `assemble_from_children()`
2. Method checks for NotMutatable errors and creates appropriate paths
3. Keeps main `build_paths()` flow clean and readable

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Code changes:**
```rust
impl ProtocolEnforcer {
    /// Handle errors from assemble_from_children, creating NotMutatable paths when appropriate
    fn handle_assemble_error(
        &self,
        ctx: &RecursionContext,
        error: crate::error::Error,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check if it's a NotMutatable condition
        if let Some(reason) = error.as_not_mutatable() {
            // Return a single NotMutatable path for this type
            Ok(vec![Self::build_not_mutatable_path(ctx, reason.clone())])
        } else {
            // Real error - propagate it
            Err(error)
        }
    }
}
```

**Expected outcome:**
- Clean separation of error handling logic
- Main build_paths flow remains readable
- Consistent NotMutatable path creation

### STEP 4: Verify Test Components  
**Status:** ✅ COMPLETED

**Objective:** Verify existing test components adequately cover mutation status scenarios

**Existing test infrastructure:**
1. `TestMapComponent.enum_keyed: HashMap<SimpleTestEnum, String>` - Complex enum keys (NotMutatable)
2. `TestMapComponent.strings: HashMap<String, String>` - Simple string keys (Mutatable)
3. `SimpleSetComponent.string_set: HashSet<String>` - Simple string elements (Mutatable)
4. `TestCollectionComponent.struct_set: HashSet<TestStructWithSerDe>` - Complex struct elements (NotMutatable)

**No new components needed** - Existing infrastructure already covers:
- HashMap with complex keys → NotMutatable
- HashMap with simple keys → Mutatable
- HashSet with complex elements → NotMutatable  
- HashSet with simple elements → Mutatable

**Expected outcome:**
- Leverage existing test components without duplication
- All mutation status scenarios already covered by current infrastructure

### STEP 4a: Add NotInRegistry Handling to ProtocolEnforcer
**Status:** ✅ COMPLETED

**Objective:** Add NotInRegistry check and handling in ProtocolEnforcer right after recursion limit check

**Changes to make:**
1. Add registry check immediately after recursion limit check in `build_paths()`
2. Use existing `build_not_mutatable_path()` helper to create consistent NotMutatable paths
3. This centralizes all infrastructure-level NotMutatable conditions in ProtocolEnforcer

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Code changes:**
```rust
// In ProtocolEnforcer::build_paths()
// Right after recursion limit check (line 77), add:

// 1a. Check if type is in registry
if ctx.require_registry_schema().is_none() {
    return Ok(vec![Self::build_not_mutatable_path(
        ctx,
        NotMutatableReason::NotInRegistry(ctx.type_name().clone()),
    )]);
}
```

**Expected outcome:**
- Types not in registry are caught early by ProtocolEnforcer
- Consistent NotMutatable path formatting for registry issues
- Individual builders no longer need registry checks

### STEP 5: Add MutationStatus Propagation Handler
**Status:** ✅ COMPLETED

**Objective:** Implement mutation status propagation in `ProtocolEnforcer` to compute parent status from children

**Changes to make:**
1. Add `determine_parent_mutation_status()` function
2. Use the helper method to handle NotMutatable errors from `assemble_from_children()`
3. Update parent path's mutation status based on children

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Code changes:**
```rust
// In ProtocolEnforcer::build_paths()
// After collecting children, try to assemble parent
let parent_example = match self.inner.assemble_from_children(ctx, child_examples) {
    Ok(example) => example,
    Err(e) => {
        // Use helper method to handle NotMutatable errors cleanly
        return self.handle_assemble_error(ctx, e);
    }
};

// After collecting all child paths, determine parent mutation status
let parent_status = Self::determine_parent_mutation_status(&all_paths);

// Update the parent path with computed status
all_paths.insert(0, MutationPathInternal {
    path: ctx.mutation_path.clone(),
    example: parent_example,
    type_name: ctx.type_name().clone(),
    path_kind: ctx.path_kind.clone(),
    mutation_status: parent_status, // Computed from children
    error_reason: if matches!(parent_status, MutationStatus::NotMutatable) {
        Some("all_children_not_mutatable".to_string())
    } else if matches!(parent_status, MutationStatus::PartiallyMutatable) {
        Some("mixed_mutability_children".to_string())
    } else {
        None
    },
});

impl ProtocolEnforcer {
    fn determine_parent_mutation_status(child_paths: &[MutationPathInternal]) -> MutationStatus {
        // Fast path: if ANY child is PartiallyMutatable, parent must be too
        if child_paths.iter().any(|p| matches!(p.mutation_status, MutationStatus::PartiallyMutatable)) {
            return MutationStatus::PartiallyMutatable;
        }
        
        let has_mutatable = child_paths.iter().any(|p| matches!(p.mutation_status, MutationStatus::Mutatable));
        let has_not_mutatable = child_paths.iter().any(|p| matches!(p.mutation_status, MutationStatus::NotMutatable));
        
        match (has_mutatable, has_not_mutatable) {
            (true, true) => MutationStatus::PartiallyMutatable,   // Mixed
            (true, false) => MutationStatus::Mutatable,          // All mutatable  
            (false, true) => MutationStatus::NotMutatable,       // All not mutatable
            (false, false) => MutationStatus::Mutatable,         // No children (leaf)
        }
    }
}
```

**Expected outcome:**
- Parent mutation status correctly computed from children
- Clean propagation rules: All same → Parent same, Mixed → PartiallyMutatable
- Simple error handling preserved with `?` operator

### STEP 6: Update MapMutationBuilder to Return NotMutatable Error
**Status:** ✅ COMPLETED

**Objective:** Update MapMutationBuilder to detect complex keys and return NotMutatable error

**Changes to make:**
1. Use `is_complex_type()` from `JsonFieldAccess` trait
2. Return `Error::NotMutatable` instead of generic error
3. ProtocolEnforcer will handle path creation

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`

**Code changes:**
```rust
// In MapMutationBuilder::is_complex_key()
fn is_complex_key(value: &Value) -> bool {
    value.is_complex_type()  // Use the trait method
}

// In MapMutationBuilder::assemble_from_children()
if Self::is_complex_key(key_example) {
    // Return NotMutatable error instead of generic error
    return Err(Error::NotMutatable(
        NotMutatableReason::ComplexCollectionKey(ctx.type_name().clone())
    ).into());
}

// Continue with normal map assembly if keys are primitive...
```

**Expected outcome:**
- MapMutationBuilder signals complex keys via NotMutatable error
- ProtocolEnforcer creates the NotMutatable path
- Type system enforces proper reason usage

### STEP 8: Implement SetMutationBuilder Complex Element Detection
**Status:** ✅ COMPLETED

**Objective:** Add complex element detection for HashSet types and return NotMutatable error

**Changes to make:**
1. Add `is_complex_type()` method to `JsonFieldAccess` trait in `string_traits.rs`
2. Add `SetMutationBuilder::is_complex_element()` using the trait method
3. Return `Error::NotMutatable` instead of throwing error
4. ProtocolEnforcer will handle path creation

**Files to modify:**
- `mcp/src/string_traits.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`

**Code changes:**
```rust
// In string_traits.rs, add to JsonFieldAccess trait:
pub trait JsonFieldAccess {
    // ... existing methods ...
    
    /// Check if this JSON value represents a complex (non-primitive) type
    /// Complex types (Array, Object) cannot be used as HashMap keys or HashSet elements in BRP
    fn is_complex_type(&self) -> bool;
}

impl JsonFieldAccess for Value {
    // ... existing methods ...
    
    fn is_complex_type(&self) -> bool {
        matches!(self, Value::Array(_) | Value::Object(_))
    }
}

// In SetMutationBuilder::is_complex_element()
fn is_complex_element(value: &Value) -> bool {
    value.is_complex_type()  // Use the trait method
}

// In SetMutationBuilder::assemble_from_children()
if Self::is_complex_element(item_example) {
    // Return NotMutatable error instead of generic error
    return Err(Error::NotMutatable(
        NotMutatableReason::ComplexCollectionKey(ctx.type_name().clone())
    ).into());
}

// Continue with normal set assembly if elements are primitive...
```

**Expected outcome:**
- SetMutationBuilder signals complex elements via NotMutatable error
- ProtocolEnforcer creates the NotMutatable path
- Type system enforces proper reason usage

### STEP 9: Test and Validate Complete System
**Status:** ✅ COMPLETED

**Objective:** Verify all mutation status scenarios work correctly

**Changes to make:**
1. Test `TestMapComponent.enum_keyed` → Should be NotMutatable
2. Test `TestMapComponent.strings` → Should remain Mutatable  
3. Test `TestSetComponent.complex_set` → Should be NotMutatable
4. Test `TestSetComponent.simple_set` → Should remain Mutatable
5. Test parent components containing mixed children → Should be PartiallyMutatable

**Validation checklist:**
- [ ] Complex HashMap keys marked NotMutatable
- [ ] Simple HashMap keys remain Mutatable
- [ ] Complex HashSet elements marked NotMutatable  
- [ ] Simple HashSet elements remain Mutatable
- [ ] Parent components with mixed children marked PartiallyMutatable
- [ ] Error messages are clear and informative
- [ ] No panics or fallback values

**Expected outcome:**
- All mutation status scenarios working correctly
- Proper error propagation and status assignment
- System ready for production use

## Mutation Status Propagation Rules

### Core Algorithm
```rust
fn determine_parent_mutation_status(child_paths: &[MutationPathInternal]) -> MutationStatus {
    // Rule 1: ANY PartiallyMutatable child → Parent is PartiallyMutatable
    if child_paths.iter().any(|p| matches!(p.mutation_status, MutationStatus::PartiallyMutatable)) {
        return MutationStatus::PartiallyMutatable;
    }
    
    // Rule 2: Count Mutatable vs NotMutatable children
    let has_mutatable = child_paths.iter().any(|p| matches!(p.mutation_status, MutationStatus::Mutatable));
    let has_not_mutatable = child_paths.iter().any(|p| matches!(p.mutation_status, MutationStatus::NotMutatable));
    
    // Rule 3: Apply propagation logic
    match (has_mutatable, has_not_mutatable) {
        (true, true) => MutationStatus::PartiallyMutatable,   // Mixed children
        (true, false) => MutationStatus::Mutatable,          // All children mutatable
        (false, true) => MutationStatus::NotMutatable,       // All children not mutatable
        (false, false) => MutationStatus::Mutatable,         // No children (leaf node)
    }
}
```

### Propagation Examples
1. **Struct with all primitive fields** → `Mutatable`
2. **Struct with complex HashMap field** → `PartiallyMutatable` (field is NotMutatable, others Mutatable)
3. **HashMap with enum keys** → `NotMutatable` (terminal - cannot mutate)
4. **HashSet with enum elements** → `NotMutatable` (terminal - cannot mutate)
5. **Nested struct containing PartiallyMutatable child** → `PartiallyMutatable` (cascades up)

## Complex Collection Detection

### HashMap Complex Keys
```rust
fn is_complex_key(value: &Value) -> bool {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => false, // Primitive
        Value::Array(_) | Value::Object(_) => true, // Complex (enum/struct)
    }
}
```

### HashSet Complex Elements
```rust
fn is_complex_element(value: &Value) -> bool {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => false, // Primitive
        Value::Array(_) | Value::Object(_) => true, // Complex (enum/struct)
    }
}
```

## Test Scenarios

### TestMapComponent
- `strings: HashMap<String, String>` → `Mutatable` (primitive keys)
- `enum_keyed: HashMap<SimpleTestEnum, String>` → `NotMutatable` (complex keys)

### TestSetComponent  
- `simple_set: HashSet<String>` → `Mutatable` (primitive elements)
- `complex_set: HashSet<SimpleTestEnum>` → `NotMutatable` (complex elements)

### Parent Components
- Component containing only `TestMapComponent.strings` → `Mutatable`
- Component containing only `TestMapComponent.enum_keyed` → `NotMutatable`
- Component containing both → `PartiallyMutatable`

## Benefits

1. **Centralized Logic**: All mutation status assignment in `ProtocolEnforcer`
2. **Consistent Propagation**: Uniform rules applied at every level
3. **Rich Error Context**: Detailed information for debugging
4. **Future-Proof**: Works for all current and future migrated builders
5. **Clean Separation**: True errors vs NotMutatable conditions properly separated

## Expected Outcome
- All migrated builders (Default, Map, Set) use centralized mutation status handling
- Complex collections properly marked NotMutatable with clear error messages
- Mutation status propagates correctly through nested structures
- Foundation established for future builder migrations
- No panics, fallbacks, or inconsistent behavior

## Design Review Notes

### TYPE-SYSTEM-1: Identical Complex Detection Logic - **Verdict**: CONFIRMED ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: Mutation Status Propagation Rules
- **Issue**: The plan originally proposed duplicate `is_complex_element` and `is_complex_key` functions
- **Reasoning**: Both functions perform identical Value pattern matching to detect complex types

### Approved Change:
Extend the existing `JsonFieldAccess` trait in `string_traits.rs` with an `is_complex_type()` method that both MapMutationBuilder and SetMutationBuilder can use, eliminating code duplication and creating a single source of truth for complex type detection.

### Implementation Notes:
- Leverages existing `JsonFieldAccess` trait infrastructure
- Both builders use `value.is_complex_type()` for consistency
- Avoids creating unnecessary new modules

### TYPE-SYSTEM-2: MutationSupport Enum Semantic Duplication - **Verdict**: MODIFIED ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: Add Error Infrastructure
- **Issue**: Plan proposed adding `ComplexSetElement` variant alongside existing `ComplexMapKey`
- **Reasoning**: Both variants represent the same concept - collections with complex keys/elements that cannot be mutated

### Approved Change:
Rename the existing `ComplexMapKey` variant to `ComplexCollectionKey`. This single variant name covers both HashMap keys and HashSet elements, eliminating semantic duplication while requiring minimal code changes.

### Implementation Notes:
- Single variant handles both HashMap and HashSet cases
- Updates required to Display and From implementations
- Cleaner, more maintainable enum design

### DESIGN-1: Overlapping Error Handling Pathways - **Verdict**: REJECTED (After Investigation)
- **Status**: REJECTED - Investigation showed selective error catching is idiomatic Rust
- **Location**: Section: Add Centralized MutationStatus Handler
- **Issue**: Originally thought error catching created confusion
- **Investigation Result**: Selective error catching is common in Rust (146+ instances in codebase)

### New Approach:
Builders return `Error::NotMutatable` when they detect non-mutatable conditions, and only ProtocolEnforcer creates the actual NotMutatable paths using its private `build_not_mutatable_path()` helper. This enforces proper separation: builders detect conditions and return errors, ProtocolEnforcer handles path creation.

### Implementation Notes:
- Builders detect NotMutatable conditions and return errors
- Only ProtocolEnforcer creates NotMutatable paths
- Clean separation of concerns with proper encapsulation

### DESIGN-3: Missing Test Component Conflicts - **Verdict**: CONFIRMED ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: Create Test Components
- **Issue**: Plan proposed new TestSetComponent when existing components already cover all scenarios
- **Reasoning**: TestMapComponent.enum_keyed, SimpleSetComponent.string_set, and TestCollectionComponent.struct_set already provide complete test coverage

### Approved Change:
Use existing test infrastructure without creating new components. The current components already cover all HashMap and HashSet mutation scenarios with both simple and complex keys/elements.

### Implementation Notes:
- No new test components needed
- Reduces code duplication and maintenance burden
- Leverages well-established test infrastructure