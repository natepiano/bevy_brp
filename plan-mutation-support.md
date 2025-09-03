# Plan: Implement Comprehensive MutationSupport Enum

## Problem Statement

The current implementation uses a simplified binary `MutationSupport` enum that only distinguishes between `Supported` and `NotSupported`. This provides no actionable information about WHY a type cannot be mutated, making it difficult for users and AI agents to understand mutation failures.

**Current Implementation (Oversimplified):**
```rust
#[derive(Debug, Clone, PartialEq)]
enum MutationSupport {
    Supported,
    NotSupported,
}
```

**What We Need:**
A comprehensive enum that provides detailed error context explaining exactly why mutation is not supported, enabling better error messages in the JSON response.

## Desired Outcome

Users and AI agents should receive clear, actionable error messages like:
- "Type `VisibilityClass` contains non-mutatable element type: `smallvec::SmallVec<[core::any::TypeId; 1]>`"
- "Type `TypeId` lacks Serialize/Deserialize traits required for mutation"
- "Type `CustomContainer<NonSerializable>` analysis exceeded maximum recursion depth"

## Technical Implementation

### Error Propagation Strategy for Nested Types

When container types fail mutation validation due to inner types, the enhanced system must propagate detailed error information to provide actionable feedback about the root cause of validation failures.

#### Nested Type Error Propagation Requirements:

1. **Preserve Root Cause**: For `Vec<NonSerializable>` → report `NonMutatableElements { container_type: Vec<NonSerializable>, element_type: NonSerializable }`
2. **Deep Tracing**: For complex nested types like `Vec<HashMap<String, TypeId>>` → trace to the deepest failing type (`core::any::TypeId`)
3. **Error Context Preservation**: Maintain full type path information through recursive validation
4. **Actionable Information**: Include both container and failing element types in error messages

#### Enhanced Recursive Validation Pattern:

```rust
fn analyze_mutation_support_with_depth(
    &self,
    type_name: &BrpTypeName,
    depth: RecursionDepth,
) -> MutationSupport {
    // ... existing validation logic ...
    
    match type_kind {
        TypeKind::List | TypeKind::Array => {
            match self.extract_list_element_type(schema) {
                Some(elem_type) => {
                    let elem_support = self.analyze_mutation_support_with_depth(&elem_type, depth.increment());
                    match elem_support {
                        MutationSupport::Supported => MutationSupport::Supported,
                        // Preserve error context: don't just report elem_type failure,
                        // but propagate the deepest failing type while noting this container
                        failing_result => MutationSupport::NonMutatableElements {
                            container_type: type_name.clone(),
                            element_type: failing_result.get_deepest_failing_type().unwrap_or(elem_type)
                        }
                    }
                }
                None => MutationSupport::UnknownType(type_name.clone()),
            }
        }
        // Similar pattern for Map, Option, Tuple, etc.
    }
}
```

#### Supporting Methods for Error Propagation:

```rust
impl MutationSupport {
    /// Extract the deepest failing type from nested error contexts
    fn get_deepest_failing_type(&self) -> Option<BrpTypeName> {
        match self {
            Self::Supported => None,
            Self::MissingSerializationTraits(type_name) => Some(type_name.clone()),
            Self::UnknownType(type_name) => Some(type_name.clone()),
            Self::RecursionLimitExceeded(type_name) => Some(type_name.clone()),
            Self::NonMutatableElements { element_type, .. } => Some(element_type.clone()),
        }
    }
    
    /// Check if this represents a deep nested failure that should be propagated
    fn should_propagate_error(&self) -> bool {
        !matches!(self, Self::Supported)
    }
}
```

#### Expected Error Propagation Examples:

- `Vec<NonSerializable>` → `"Type Vec<NonSerializable> contains non-mutatable element type: NonSerializable"`
- `Vec<HashMap<String, TypeId>>` → `"Type Vec<HashMap<String, core::any::TypeId>> contains non-mutatable element type: core::any::TypeId"`  
- `Option<SmallVec<[TypeId; 1]>>` → `"Type Option<SmallVec<[core::any::TypeId; 1]>> contains non-mutatable element type: core::any::TypeId"`

This ensures users receive specific information about which nested type prevents mutation, enabling them to understand and potentially resolve the validation failure.

### Step 1: Replace MutationSupport Enum

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`
**Location**: Replace existing enum at lines 24-46

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationSupport {
    /// Type fully supports mutation operations
    Supported,
    /// Type lacks required serialization traits
    MissingSerializationTraits(BrpTypeName),
    /// Container type has non-mutatable element type
    NonMutatableElements { 
        container_type: BrpTypeName, 
        element_type: BrpTypeName 
    },
    /// Type not found in registry
    UnknownType(BrpTypeName),
    /// Recursion depth limit exceeded during analysis
    RecursionLimitExceeded(BrpTypeName),
}

impl MutationSupport {
    pub const fn is_supported(&self) -> bool {
        matches!(self, Self::Supported)
    }
}

impl Display for MutationSupport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Supported => write!(f, "Type supports mutation"),
            Self::MissingSerializationTraits(type_name) => 
                write!(f, "Type {type_name} lacks Serialize/Deserialize traits required for mutation"),
            Self::NonMutatableElements { container_type, element_type } => 
                write!(f, "Type {container_type} contains non-mutatable element type: {element_type}"),
            Self::UnknownType(type_name) => 
                write!(f, "Type {type_name} not found in schema registry"),
            Self::RecursionLimitExceeded(type_name) => 
                write!(f, "Type {type_name} analysis exceeded maximum recursion depth"),
        }
    }
}
```

### Step 2: Update mutation_path_builders.rs Integration

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`

## Integration with Existing Infrastructure

The enhanced `MutationSupport` enum must integrate with the existing `type_supports_mutation_with_depth` method to avoid code duplication. The plan should modify the existing method to capture detailed failure reasons internally while maintaining its boolean return signature:

### Enhanced Type Validation Method:
```rust
fn type_supports_mutation_with_depth(
    &self,
    type_name: &BrpTypeName,
    depth: RecursionDepth,
) -> bool {
    // Store detailed validation result in context for later retrieval
    let detailed_result = self.analyze_mutation_support_with_depth(type_name, depth);
    self.store_validation_context(type_name, detailed_result.clone());
    detailed_result.is_supported()
}

// New helper method for detailed analysis
fn analyze_mutation_support_with_depth(
    &self,
    type_name: &BrpTypeName, 
    depth: RecursionDepth
) -> MutationSupport {
    // All the existing validation logic, returning structured reasons
}
```

Add methods to return detailed mutation support information:

```rust
/// Get detailed mutation support information
pub fn type_supports_mutation_detailed(&self, type_name: &BrpTypeName) -> MutationSupport {
    self.type_supports_mutation_with_depth_detailed(type_name, RecursionDepth::ZERO)
}

/// Internal implementation with detailed results
fn type_supports_mutation_with_depth_detailed(
    &self, 
    type_name: &BrpTypeName, 
    depth: RecursionDepth
) -> MutationSupport {
    use super::type_info::MutationSupport;
    
    if depth.exceeds_limit() {
        return MutationSupport::RecursionLimitExceeded(type_name.clone());
    }
    
    let Some(schema) = self.get_type_schema(type_name) else {
        return MutationSupport::UnknownType(type_name.clone());
    };
    
    let type_kind = TypeKind::from_schema(schema, type_name);
    
    match type_kind {
        TypeKind::Value => {
            if self.value_type_has_serialization(type_name) {
                MutationSupport::Supported
            } else {
                MutationSupport::MissingSerializationTraits(type_name.clone())
            }
        }
        TypeKind::List | TypeKind::Array => {
            match self.extract_list_element_type(schema) {
                Some(elem_type) => {
                    let elem_support = self.type_supports_mutation_with_depth_detailed(&elem_type, depth.increment());
                    match elem_support {
                        MutationSupport::Supported => MutationSupport::Supported,
                        failing_result => MutationSupport::NonMutatableElements {
                            container_type: type_name.clone(),
                            element_type: failing_result.get_deepest_failing_type().unwrap_or(elem_type)
                        }
                    }
                }
                None => MutationSupport::UnknownType(type_name.clone()),
            }
        }
        TypeKind::Map => {
            match self.extract_map_value_type(schema) {
                Some(val_type) => {
                    let val_support = self.type_supports_mutation_with_depth_detailed(&val_type, depth.increment());
                    match val_support {
                        MutationSupport::Supported => MutationSupport::Supported,
                        failing_result => MutationSupport::NonMutatableElements {
                            container_type: type_name.clone(),
                            element_type: failing_result.get_deepest_failing_type().unwrap_or(val_type)
                        }
                    }
                }
                None => MutationSupport::UnknownType(type_name.clone()),
            }
        }
        TypeKind::Option => {
            match self.extract_option_inner_type(schema) {
                Some(inner_type) => {
                    let inner_support = self.type_supports_mutation_with_depth_detailed(&inner_type, depth.increment());
                    match inner_support {
                        MutationSupport::Supported => MutationSupport::Supported,
                        failing_result => MutationSupport::NonMutatableElements {
                            container_type: type_name.clone(),
                            element_type: failing_result.get_deepest_failing_type().unwrap_or(inner_type)
                        }
                    }
                }
                None => MutationSupport::UnknownType(type_name.clone()),
            }
        }
        TypeKind::Tuple | TypeKind::TupleStruct => {
            match self.extract_tuple_element_types(schema) {
                Some(elements) => {
                    // For tuples, find the first non-mutatable element type
                    for elem in elements.iter() {
                        let elem_support = self.type_supports_mutation_with_depth_detailed(elem, depth.increment());
                        match elem_support {
                            MutationSupport::Supported => continue,
                            failing_result => return MutationSupport::NonMutatableElements {
                                container_type: type_name.clone(),
                                element_type: failing_result.get_deepest_failing_type().unwrap_or_else(|| elem.clone())
                            },
                        }
                    }
                    MutationSupport::Supported
                }
                None => MutationSupport::UnknownType(type_name.clone()),
            }
        }
        TypeKind::Struct | TypeKind::Enum => MutationSupport::Supported,
    }
}
```

### Step 3: Update Error Path Building

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`

Update the `build_not_mutatable_path` method to use detailed error messages:

```rust
fn build_not_mutatable_path(ctx: &MutationPathContext<'_>) -> MutationPathInternal {
    let detailed_support = ctx.get_stored_validation_result(ctx.type_name())
        .unwrap_or(MutationSupport::UnknownType(ctx.type_name().clone()));
    
    let error_message = format!("{}", detailed_support);
    
    match &ctx.location {
        RootOrField::Root { type_name } => {
            MutationPathInternal {
                path: String::new(),
                example: json!({
                    "NotMutatable": error_message,  // Enhanced structured error message
                    "agent_directive": "This type cannot be mutated - see error message for details"
                }),
                enum_variants: None,
                type_name: type_name.clone(),
                path_kind: MutationPathKind::NotMutatable,
            }
        }
        RootOrField::Field { field_name, field_type, .. } => {
            MutationPathInternal {
                path: field_name.clone(),
                example: json!({
                    "NotMutatable": error_message,  // Enhanced structured error message
                    "agent_directive": "This field cannot be mutated - see error message for details"
                }),
                enum_variants: None,
                type_name: field_type.clone(),
                path_kind: MutationPathKind::NotMutatable,
            }
        }
    }
}
```

### Step 4: Migrate Existing Error Generation Sites

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`

The plan must address how existing hardcoded error messages throughout the mutation path building system will be updated to use the new structured error approach:

#### Current Hardcoded Error Sites to Update:

1. **TypeKind::build_not_mutatable_path** (lines 381-397):
```rust
// CURRENT: Generic hardcoded message
let reason = format!(
    "Type {} cannot be mutated - contains non-serializable inner types",
    ctx.type_name()
);

// UPDATED: Use stored validation context
let detailed_support = ctx.get_stored_validation_result(ctx.type_name())
    .unwrap_or(MutationSupport::UnknownType(ctx.type_name().clone()));
let reason = format!("{}", detailed_support);
```

2. **StructMutationBuilder::build_not_mutatable_path** (lines 1282-1297):
```rust
// CURRENT: Takes generic string parameter
fn build_not_mutatable_path(
    field_name: &str,
    field_type: &BrpTypeName,
    reason: String,  // Remove this parameter
) -> MutationPathInternal

// UPDATED: Use stored validation context
fn build_not_mutatable_path(
    field_name: &str,
    field_type: &BrpTypeName,
    ctx: &MutationPathContext<'_>,  // Add context to access stored validation
) -> MutationPathInternal {
    let detailed_support = ctx.get_stored_validation_result(field_type)
        .unwrap_or(MutationSupport::UnknownType(field_type.clone()));
    let reason = format!("{}", detailed_support);
    // ... rest of implementation
}
```

3. **TupleMutationBuilder::build_tuple_element_path** (line 1098):
```rust
// CURRENT: Hardcoded format string
"NotMutatable": format!("Type {} does not support mutation", element_type),

// UPDATED: Use stored validation context  
let detailed_support = ctx.get_stored_validation_result(&element_type)
    .unwrap_or(MutationSupport::UnknownType(element_type.clone()));
"NotMutatable": format!("{}", detailed_support),
```

4. **StructMutationBuilder::build_type_based_paths** (line 965):
```rust
// CURRENT: Hardcoded "not found" message
let reason = format!("Type {} not found in schema registry", field_type.as_str());

// UPDATED: Use structured error
let reason = format!("{}", MutationSupport::UnknownType(field_type.clone()));
```

#### Migration Steps:
1. Update `TypeKind::build_not_mutatable_path` to use stored validation context instead of generic message
2. Change `StructMutationBuilder::build_not_mutatable_path` signature to accept context instead of string parameter
3. Replace all hardcoded `format!("Type {} does not support mutation")` calls with stored validation results
4. Update "not found in schema registry" errors to use `MutationSupport::UnknownType`
5. Ensure consistent error format across all path builders by routing through centralized enum system

### Step 5: Update type_info.rs from_paths Method

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`
**Location**: Replace the existing `from_paths` implementation

```rust
impl MutationSupport {
    fn from_paths(paths: &HashMap<String, MutationPath>) -> Self {
        let has_mutatable = paths.values().any(|path| {
            !matches!(
                path.path_kind,
                super::response_types::MutationPathKind::NotMutatable
            )
        });
        if has_mutatable {
            Self::Supported
        } else {
            // For post-build analysis, we can't determine the specific reason or type name,
            // so we use a generic non-mutatable status with a placeholder type name.
            // This should be replaced with actual detailed analysis in most cases.
            Self::MissingSerializationTraits(BrpTypeName::new("UnknownType".to_string()))
        }
    }
}
```

## Implementation Strategy

### Phase 1: Core Enum Implementation
1. Replace the simple `MutationSupport` enum with the comprehensive version
2. Update the `from_paths` method to maintain compatibility
3. Test that existing functionality still works

### Phase 2: Integration with Mutation Path Builders  
1. Add the detailed support checking methods to `MutationPathContext`
2. Update `build_not_mutatable_path` to use stored validation context
3. Test that enhanced error messages appear correctly in JSON output

### Phase 3: Migration of Existing Error Generation Sites
1. Update all hardcoded error generation sites to use stored validation context
2. Change method signatures to accept context instead of string parameters
3. Ensure all container type validation uses the enhanced enum throughout the system

### Phase 4: Enhanced Error Reporting Validation
1. Update tuple element path building to use stored validation results
2. Ensure all error generation flows through the centralized enum system
3. Validate that error messages are actionable and consistent across all path builders

## Expected Outcomes

### Before (Current State):
```json
{
  "path": "",
  "example": {
    "NotMutatable": "All tuple elements are not mutatable",
    "agent_directive": "This tuple struct cannot be mutated - all fields contain non-mutatable types"
  },
  "path_kind": "NotMutatable"
}
```

### After (Enhanced Messages):
```json
{
  "path": "",
  "example": {
    "NotMutatable": "Type bevy_render::view::visibility::VisibilityClass contains non-mutatable element type: smallvec::SmallVec<[core::any::TypeId; 1]>",
    "agent_directive": "This type cannot be mutated - see error message for details"
  },
  "path_kind": "NotMutatable"
}
```

## Success Criteria

1. **Detailed Error Messages**: All non-mutatable types show specific reasons for mutation failure
2. **Type-Specific Context**: Container types list their problematic element types
3. **Actionable Information**: Users can understand what makes a type non-mutatable
4. **No Regression**: All existing functionality continues to work
5. **Performance**: No significant performance impact from enhanced error reporting

## Risk Mitigation

- **Backward Compatibility**: Keep `is_supported()` method for existing boolean checks
- **Fallback Behavior**: Post-build analysis can fall back to generic error messages
- **Performance**: Detailed analysis only runs when building error paths
- **Testing**: Validate with existing VisibilityClass and SmallVec test cases

## Implementation Order

1. Implement comprehensive `MutationSupport` enum in `type_info.rs`
2. Update `from_paths` method to maintain compatibility
3. Add detailed support checking and validation context storage to `mutation_path_builders.rs`
4. Update error path building to use stored validation context
5. Migrate all existing hardcoded error generation sites to use centralized enum system
6. Test with VisibilityClass and other non-mutatable types
7. Validate JSON output contains actionable, consistent error messages across all path builders