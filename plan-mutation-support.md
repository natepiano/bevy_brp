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

### Step 1: Replace MutationSupport Enum

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`
**Location**: Replace existing enum at lines 24-46

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationSupport {
    /// Type fully supports mutation operations
    Supported,
    /// Type lacks required serialization traits
    MissingSerializationTraits,
    /// Container type has non-mutatable element type
    NonMutatableElements(BrpTypeName),
    /// Type not found in registry
    UnknownType,
    /// Recursion depth limit exceeded during analysis
    RecursionLimitExceeded,
}

impl MutationSupport {
    pub const fn is_supported(&self) -> bool {
        matches!(self, Self::Supported)
    }
    
    /// Generate user-facing error message for this support status
    pub fn to_error_message(&self, type_name: &BrpTypeName) -> String {
        match self {
            Self::Supported => String::new(),
            Self::MissingSerializationTraits => 
                format!("Type {} lacks Serialize/Deserialize traits required for mutation", type_name),
            Self::NonMutatableElements(element_type) => 
                format!("Type {} contains non-mutatable element type: {}", type_name, element_type),
            Self::UnknownType => 
                format!("Type {} not found in schema registry", type_name),
            Self::RecursionLimitExceeded => 
                format!("Type {} analysis exceeded maximum recursion depth", type_name),
        }
    }
}
```

### Step 2: Update mutation_path_builders.rs Integration

**File**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`

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
        return MutationSupport::RecursionLimitExceeded;
    }
    
    let Some(schema) = self.get_type_schema(type_name) else {
        return MutationSupport::UnknownType;
    };
    
    let type_kind = TypeKind::from_schema(schema, type_name);
    
    match type_kind {
        TypeKind::Value => {
            if self.value_type_has_serialization(type_name) {
                MutationSupport::Supported
            } else {
                MutationSupport::MissingSerializationTraits
            }
        }
        TypeKind::List | TypeKind::Array => {
            match self.extract_list_element_type(schema) {
                Some(elem_type) => {
                    let elem_support = self.type_supports_mutation_with_depth_detailed(&elem_type, depth.increment());
                    if elem_support.is_supported() {
                        MutationSupport::Supported
                    } else {
                        MutationSupport::NonMutatableElements(elem_type)
                    }
                }
                None => MutationSupport::UnknownType,
            }
        }
        TypeKind::Map => {
            match self.extract_map_value_type(schema) {
                Some(val_type) => {
                    let val_support = self.type_supports_mutation_with_depth_detailed(&val_type, depth.increment());
                    if val_support.is_supported() {
                        MutationSupport::Supported
                    } else {
                        MutationSupport::NonMutatableElements(val_type)
                    }
                }
                None => MutationSupport::UnknownType,
            }
        }
        TypeKind::Option => {
            match self.extract_option_inner_type(schema) {
                Some(inner_type) => {
                    let inner_support = self.type_supports_mutation_with_depth_detailed(&inner_type, depth.increment());
                    if inner_support.is_supported() {
                        MutationSupport::Supported
                    } else {
                        MutationSupport::NonMutatableElements(inner_type)
                    }
                }
                None => MutationSupport::UnknownType,
            }
        }
        TypeKind::Tuple | TypeKind::TupleStruct => {
            match self.extract_tuple_element_types(schema) {
                Some(elements) => {
                    // For tuples, find the first non-mutatable element type
                    for elem in elements.iter() {
                        let elem_support = self.type_supports_mutation_with_depth_detailed(elem, depth.increment());
                        if !elem_support.is_supported() {
                            return MutationSupport::NonMutatableElements(elem.clone());
                        }
                    }
                    MutationSupport::Supported
                }
                None => MutationSupport::UnknownType,
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
    let support = ctx.type_supports_mutation_detailed(ctx.type_name());
    let reason = support.to_error_message(ctx.type_name());
    
    match &ctx.location {
        RootOrField::Root { type_name } => {
            MutationPathInternal {
                path: String::new(),
                example: json!({
                    "NotMutatable": reason,
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
                    "NotMutatable": reason,
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

### Step 4: Update type_info.rs from_paths Method

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
            // For post-build analysis, we can't determine the specific reason,
            // so we use a generic non-mutatable status
            Self::MissingSerializationTraits // This is the most common case
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
2. Update `build_not_mutatable_path` to use detailed error messages
3. Test that error messages appear correctly in JSON output

### Phase 3: Enhanced Error Reporting
1. Update tuple element path building to use detailed support checking
2. Ensure all container type validation uses the enhanced enum
3. Validate that error messages are actionable and clear

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
3. Add detailed support checking to `mutation_path_builders.rs`
4. Update error path building to use enhanced messages
5. Test with VisibilityClass and other non-mutatable types
6. Validate JSON output contains actionable error messages