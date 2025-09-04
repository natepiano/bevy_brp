# Plan: Single-Pass Mutation Path Building

## GOAL
This change is not about performance, it is about making simple code that is easy to reason about and maintain.

## Problem Statement

The current implementation has **multiple recursive traversals** of the same type structure:

1. **Precheck pass**: `validate_mutation_capability()` → `type_supports_mutation_detailed()` → full recursion
2. **Builder pass**: Each individual builder (Tuple, Map, etc.) calls `type_supports_mutation_detailed()` again → full recursion per element
3. **Result**: For a tuple `(Transform, Handle<Mesh>)`, we recursively traverse Transform's Vec3 fields multiple times

## Target Architecture: Single Recursion Tree

```
TypeKind::build_paths(type)
├── Match on type structure (Struct/Tuple/Array/etc.)
├── For each field/element:
│   ├── Try to build_paths() recursively  ← SINGLE RECURSION
│   ├── If recursion fails → NotMutatable path
│   └── If recursion succeeds → Normal path
└── Return all collected paths
```

**Key Principle**: Start at root type, recurse through fields once, discover errors during path building.

## Call Flow Diagram

### Current (Multiple Recursions)
```
TypeKind::build_paths()
├── validate_mutation_capability() ──┐
│   └── type_supports_mutation_detailed() ──┐
│       └── FULL RECURSION #1           ├─── WASTE
├── TupleMutationBuilder.build_paths()      │
│   └── type_supports_mutation_detailed()──┐
│       └── FULL RECURSION #2           ├─── WASTE
└── More builders with more recursions... ──┘
```

### Target (Single Recursion)
```
TypeKind::build_paths()
├── Struct → for each field: child_type.build_paths() ──┐
├── Tuple → for each element: element_type.build_paths()│── SINGLE
├── Array → element_type.build_paths()                  │   RECURSION
├── Value → check serialization inline                  │   TREE
└── Return paths or NotMutatable based on results    ───┘
```

## Implementation Strategy

### Step 1: Remove All Precheck Methods
Delete these methods entirely:
- `validate_mutation_capability()`
- `type_supports_mutation_detailed()`
- `type_supports_mutation_with_depth_detailed()`

### Step 2: Add RecursionDepth Parameter (Following Existing Pattern)

The current codebase passes `RecursionDepth` as a separate parameter, not embedded in structs. We'll follow this established pattern:

```rust
pub trait MutationPathBuilder {
    /// Build mutation paths with depth tracking for recursion safety
    fn build_paths(
        &self, 
        ctx: &MutationPathContext<'_>,
        depth: RecursionDepth  // NEW: Track recursion depth
    ) -> Result<Vec<MutationPathInternal>>;
}
```

Note: The lifetime `'a` on `MutationPathContext` is necessary because it holds a reference to the registry HashMap, avoiding expensive cloning on each recursive call.

### Step 3: Rewrite TypeKind::build_paths() for Single Recursion

```rust
impl MutationPathBuilder for TypeKind {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        // Check recursion limit first
        if depth.exceeds_limit() {
            return Ok(vec![build_not_mutatable_path(ctx, MutationSupport::RecursionLimitExceeded(ctx.type_name()))]);
        }
        
        // Only increment depth for container types that recurse into nested structures
        let builder_depth = match self {
            // Container types that recurse - increment depth
            Self::Struct | Self::Tuple | Self::TupleStruct | 
            Self::Array | Self::List | Self::Map | Self::Enum => depth.increment(),
            // Leaf types and wrappers - preserve current depth
            Self::Value | Self::Option => depth,
        };
        
        match self {
            Self::Struct => {
                // For each field, try build_paths() recursively
                StructMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::Tuple | Self::TupleStruct => {
                // For each element, try build_paths() recursively
                TupleMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::Array => {
                // Try element_type.build_paths()
                ArrayMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::List => {
                // Try element_type.build_paths()
                ListMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::Map => {
                // Try value_type.build_paths()
                MapMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::Enum => {
                // For each variant, try build_paths() recursively
                EnumMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::Option => {
                // Keep current Option handling (will be changed to enum in separate plan)
                // Try inner_type.build_paths()
                DefaultMutationBuilder.build_paths(ctx, builder_depth)
            }
            Self::Value => {
                // Check serialization inline, no recursion needed
                if !ctx.value_type_has_serialization(ctx.type_name()) {
                    Ok(vec![build_not_mutatable_path(ctx, MutationSupport::MissingSerializationTraits(ctx.type_name()))])
                } else {
                    DefaultMutationBuilder.build_paths(ctx, builder_depth)
                }
            }
        }
    }
}
```

### Step 4: Update Individual Builders for Single Recursion

The key principle: **Containers recurse down**, **Endpoints build paths**.

#### Container Types (Array, Map, List) - Extract and Recurse

Containers extract their inner type and recurse deeper. They don't create mutation paths themselves:

```rust
// ArrayMutationBuilder - Extract element type and recurse
impl MutationPathBuilder for ArrayMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        let Some(element_type) = MutationPathContext::extract_list_element_type(schema) else {
            // This case is actually impossible - if we have a schema, we can extract the type
            // But if extraction somehow fails, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(element_type))]);
        };
        
        // RECURSE DEEPER - don't stop at array level
        let Some(element_schema) = ctx.get_type_schema(&element_type) else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(element_type))]);
        };
        let element_kind = TypeKind::from_schema(element_schema, &element_type);
        // Create a new root context for the element type
        let element_ctx = MutationPathContext::new(
            RootOrField::root(&element_type),
            ctx.registry,
            None  // No wrapper for element contexts
        );
        
        // Continue recursion to actual mutation endpoints
        let element_paths = element_kind.build_paths(&element_ctx, depth);  // depth already incremented by TypeKind
        paths.extend(element_paths);  // build_paths never returns Err
    }
}

// MapMutationBuilder - Extract value type and recurse
impl MutationPathBuilder for MapMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        let Some(value_type) = MutationPathContext::extract_map_value_type(schema) else {
            // This case is actually impossible - if we have a schema, we can extract the type
            // But if extraction somehow fails, treat as NotInRegistry
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(value_type))]);
        };
        
        // Maps are currently treated as opaque (cannot mutate individual keys)
        // So we just validate value type has serialization and build a single path
        if !ctx.value_type_has_serialization(&value_type) {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::MissingSerializationTraits(value_type))]);
        }
        
        // Build single opaque mutation path for the entire map
        DefaultMutationBuilder.build_paths(ctx, depth)
    }
}
```

#### Endpoint Types (Struct, Tuple, Enum) - Build Actual Paths

Endpoints create the actual mutation paths:

```rust
// StructMutationBuilder - THIS creates actual mutation paths for fields
impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        let mut paths = Vec::new();
        let properties = StructMutationBuilder::extract_properties(ctx);
        
        for (field_name, field_info) in properties {
            let Some(field_type) = SchemaField::extract_field_type(field_info) else {
                // This case is actually impossible - if we have a schema, we can extract the type
                // But if extraction somehow fails, treat as NotInRegistry
                paths.push(StructMutationBuilder::build_not_mutatable_field(field_name, MutationSupport::NotInRegistry(field_name.clone())));
                continue;
            };
            
            // Build actual mutation path for this field
            // Use existing create_field_context method (requires wrapper_info parameter)
            let field_ctx = ctx.create_field_context(&field_name, &field_type, None);
            
            // Check if field is a Value type needing serialization
            let Some(field_schema) = ctx.get_type_schema(&field_type) else {
                paths.push(StructMutationBuilder::build_not_mutatable_field(field_name, MutationSupport::NotInRegistry(field_type)));
                continue;
            };
            let field_kind = TypeKind::from_schema(field_schema, &field_type);
            
            if matches!(field_kind, TypeKind::Value) {
                if !ctx.value_type_has_serialization(&field_type) {
                    paths.push(StructMutationBuilder::build_not_mutatable_field(field_name, MutationSupport::MissingSerializationTraits(field_type)));
                } else {
                    paths.push(StructMutationBuilder::build_field_mutation_path(field_name, field_type, parent_type, ctx));
                }
            } else {
                // Recurse for nested containers or structs
                // build_paths always returns Ok(Vec<...>) with errors as NotMutatable paths
                let field_paths = field_kind.build_paths(&field_ctx, depth);
                paths.extend(field_paths);
            }
        }
        
        Ok(paths)
    }
}

// TupleMutationBuilder - Creates paths for tuple elements
impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        let mut paths = Vec::new();
        let elements = MutationPathContext::extract_tuple_element_types(schema).unwrap_or_default();
        
        // Build root tuple path inline (not a separate function in implementation)
        match &ctx.location {
            RootOrField::Root { type_name } => {
                paths.push(MutationPathInternal {
                    path: String::new(),
                    example: TupleMutationBuilder::build_tuple_example(schema.get_field(SchemaField::PrefixItems).unwrap_or(&json!([]))),
                    enum_variants: None,
                    type_name: type_name.clone(),
                    path_kind: MutationPathKind::RootValue { type_name: type_name.clone() },
                });
            }
            RootOrField::Field { field_name, field_type, parent_type } => {
                paths.push(MutationPathInternal {
                    path: format!(".{field_name}"),
                    example: TupleMutationBuilder::build_tuple_example(schema.get_field(SchemaField::PrefixItems).unwrap_or(&json!([]))),
                    enum_variants: None,
                    type_name: field_type.clone(),
                    path_kind: MutationPathKind::StructField { field_name: field_name.clone(), parent_type: parent_type.clone() },
                });
            }
        }
        
        // Build paths for each element
        for (index, element_type) in elements.iter().enumerate() {
            // Use existing create_field_context with index as field name
            let element_ctx = ctx.create_field_context(&index.to_string(), &element_type, None);
            let Some(element_schema) = ctx.get_type_schema(&element_type) else {
                // Build not mutatable element path for missing registry entry
                paths.push(MutationPathInternal {
                    path: format!(".{index}"),
                    example: json!({
                        "NotMutatable": format!("{}", MutationSupport::NotInRegistry(element_type.clone())),
                        "agent_directive": "Element type not found in registry"
                    }),
                    enum_variants: None,
                    type_name: element_type.clone(),
                    path_kind: MutationPathKind::NotMutatable,
                });
                continue;
            };
            let element_kind = TypeKind::from_schema(element_schema, &element_type);
            
            // Similar to struct fields - check Value types for serialization
            if matches!(element_kind, TypeKind::Value) {
                if !ctx.value_type_has_serialization(&element_type) {
                    // Build not mutatable element path inline
                    paths.push(MutationPathInternal {
                        path: format!(".{index}"),
                        example: json!({
                            "NotMutatable": format!("{}", MutationSupport::MissingSerializationTraits(element_type.clone())),
                            "agent_directive": "Element type cannot be mutated through BRP"
                        }),
                        enum_variants: None,
                        type_name: element_type.clone(),
                        path_kind: MutationPathKind::NotMutatable,
                    });
                } else {
                    // Use TupleMutationBuilder::build_tuple_element_path
                    if let Some(element_path) = TupleMutationBuilder::build_tuple_element_path(ctx, index, element_info, "", &ctx.type_name()) {
                        paths.push(element_path);
                    }
                }
            } else {
                // Recurse for nested types
                // build_paths always returns Ok(Vec<...>) with errors as NotMutatable paths
                let element_paths = element_kind.build_paths(&element_ctx, depth);
                paths.extend(element_paths);
            }
        }
        
        // Propagate mixed mutability status to root path
        propagate_tuple_mutability(&mut paths);
        Ok(paths)
    }
}

// EnumMutationBuilder - Enums are always mutatable as atomic units
impl MutationPathBuilder for EnumMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        // Enums can always be replaced entirely - no recursion into variants needed
        // Build enum mutation path inline (following the existing implementation)
        let enum_variants = EnumMutationBuilder::extract_enum_variants(schema);
        let enum_example = EnumMutationBuilder::build_enum_example(schema, ctx.registry, Some(ctx.type_name()));
        
        match &ctx.location {
            RootOrField::Root { type_name } => {
                Ok(vec![MutationPathInternal {
                    path: String::new(),
                    example: enum_example,
                    enum_variants,
                    type_name: type_name.clone(),
                    path_kind: MutationPathKind::RootValue { type_name: type_name.clone() },
                }])
            }
            RootOrField::Field { field_name, field_type, parent_type } => {
                Ok(vec![MutationPathInternal {
                    path: format!(".{field_name}"),
                    example: ctx.wrap_example(enum_example),
                    enum_variants,
                    type_name: field_type.clone(),
                    path_kind: MutationPathKind::StructField { field_name: field_name.clone(), parent_type: parent_type.clone() },
                }])
            }
        }
    }
}
```

#### Summary of Recursion Pattern

1. **TypeKind** checks depth and delegates to appropriate builder
2. **Container builders** (Array/Map) extract inner types and recurse deeper (or error if extraction fails)  
3. **Recursion continues** through nested containers until reaching endpoints
4. **Endpoint builders** (Struct/Tuple/Enum) create the actual mutation paths
5. **Value types** check serialization at the endpoints

This creates a single recursion tree where paths are built bottom-up from the actual mutation points.

### Step 5: Establish Consistent Error Handling

**CRITICAL**: All builders must return NotMutatable paths for error conditions, never empty vectors. This ensures consistent error reporting across the system.

```rust
// Standard error handling pattern for ALL builders:
impl MutationPathBuilder for ArrayMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            // CONSISTENT: Return NotMutatable path, not empty vector
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        // Rest of implementation...
    }
}

impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            // CONSISTENT: Return NotMutatable path, not empty vector  
            return Ok(vec![Self::build_not_mutatable_path(ctx, MutationSupport::NotInRegistry(ctx.type_name()))]);
        };
        
        // Rest of implementation...
    }
}

// Add standard helper method for consistent error path construction:
impl MutationPathBuilder {
    fn build_not_mutatable_path(ctx: &MutationPathContext<'_>, support: MutationSupport) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.current_path(),
            example: json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This {} cannot be mutated - {}", ctx.type_description(), support)
            }),
            enum_variants: None,
            type_name: ctx.type_name().clone(),
            path_kind: MutationPathKind::NotMutatable,  // Will become mutation_status with MutationStatus separation
            error_reason: Option::<String>::from(&support),  // NEW: Structured error reason
        }
    }
}
```

**Error Handling Rules**:
1. **Never return empty vectors** - Always return at least one NotMutatable path explaining why
2. **Consistent error messages** - Use standardized reason strings ("Schema not found", "Recursion limit exceeded", etc.)
3. **Propagate error_reason** - Include structured error information for better debugging
4. **Use shared helper** - All builders use the same `build_not_mutatable_path` helper for consistency

## Success Criteria

1. **Single traversal**: Each type structure is visited exactly once
2. **No precheck methods**: All validation happens during path building
3. **Same output**: JSON results identical to current implementation
4. **Simpler debugging**: One call stack to trace for any mutation issue

## Option Type Handling Note

**Important**: `Option` types are currently handled as containers but will be converted to work like regular enums in a **separate future plan**. This plan keeps Option handling exactly as-is to avoid scope creep. The future Option-to-enum conversion will be a separate atomic change.



## MutationPathKind vs MutationStatus Separation

### Problem: Mixed Concerns in MutationPathKind

Currently `MutationPathKind` mixes two distinct concepts:
1. **Mutation classification**: What type of path this is (struct field, array element, etc.)
2. **Error reporting**: Whether mutation is possible (`NotMutatable`, `PartiallyMutatable`)

This creates confusing API responses where `path_kind: "NotMutatable"` doesn't describe *what* to mutate.

### Solution: Separate Enums

#### New MutationStatus Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Path can be fully mutated
    Mutatable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutatable,
    /// Path is partially mutatable (some elements mutable, others not)
    PartiallyMutatable,
}
```

#### Cleaned MutationPathKind
```rust
pub enum MutationPathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField { field_name: String, parent_type: BrpTypeName },
    /// Mutate an element in a tuple by index
    TupleElement { index: usize, parent_type: BrpTypeName },
    /// Mutate an element in an array
    ArrayElement { index: usize, parent_type: BrpTypeName },
    /// Complex nested path (fallback for complicated paths)
    NestedPath { components: Vec<String>, final_type: BrpTypeName },
}
```

#### Updated Response Structure
```rust
pub struct MutationPath {
    pub description: String,
    #[serde(rename = "type")]
    pub type_name: BrpTypeName,
    pub mutation_status: MutationStatus,  // NEW: Whether mutation is possible
    pub path_kind: MutationPathKind,      // EXISTING: What type of mutation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,     // NEW: Only in error responses
    // ... existing fields (example, enum_variants, etc.)
}
```

#### Structured Error Details (Complementary Improvement)

While separating MutationStatus from MutationPathKind improves API clarity, we should also preserve structured error information internally instead of converting to strings:

```rust
// Internal: Add error_reason to MutationPathInternal
pub struct MutationPathInternal {
    pub path: String,
    pub example: Value,
    pub enum_variants: Option<Vec<String>>,
    pub type_name: BrpTypeName,
    pub path_kind: MutationPathKind,
    pub error_reason: Option<String>,  // NEW: Populated from MutationSupport
}

// Convert MutationSupport to error_reason string
impl From<&MutationSupport> for Option<String> {
    fn from(support: &MutationSupport) -> Self {
        match support {
            MutationSupport::Supported => None,
            MutationSupport::MissingSerializationTraits(_) => Some("missing_serialization_traits".to_string()),
            MutationSupport::NonMutatableElements { .. } => Some("non_mutatable_elements".to_string()),
            MutationSupport::RecursionLimitExceeded(_) => Some("recursion_limit_exceeded".to_string()),
        }
    }
}

// External API: MutationPath includes error_reason
pub struct MutationPath {
    pub description: String,
    pub type_name: BrpTypeName,
    pub mutation_status: MutationStatus,
    pub path_kind: MutationPathKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,  // NEW: Only appears when there's an error
    // ... existing fields
}
```

##### Example Usage in Builder
```rust
// When building NotMutatable paths:
fn build_not_mutatable_path_from_support(
    ctx: &MutationPathContext<'_>,
    support: &MutationSupport,
) -> MutationPathInternal {
    MutationPathInternal {
        path: /* ... */,
        example: /* ... */,
        enum_variants: None,
        type_name: /* ... */,
        path_kind: MutationPathKind::StructField { /* ... */ },
        error_reason: Option::<String>::from(support),  // NEW: Preserve error reason
    }
}
```

##### API Response with Error Reason
```json
{
  "description": "Cannot mutate field 'mesh' of MeshBundle",
  "mutation_status": "not_mutatable",
  "path_kind": "struct_field",
  "type": "Handle<Mesh>",
  "error_reason": "missing_serialization_traits"
}
```

This preserves error information without string formatting, enabling consistent error handling.

### Implementation Changes

#### 1. Remove Error Variants from MutationPathKind
- Delete `NotMutatable` and `PartiallyMutatable` variants
- Update `description()` method to only handle actual mutation types
- Remove error handling logic from `Display` and `Serialize` implementations

#### 2. Add MutationStatus to Response Types
- Add `mutation_status` field to `MutationPath` struct
- Add `mutation_status` field to `MutationPathInternal` struct
- Update `from_mutation_path()` method to set status based on path viability

#### 3. Update Path Builders
- Builders set `mutation_status: MutationStatus::NotMutatable` instead of `path_kind: NotMutatable`
- Tuple builder logic for partial mutability uses `mutation_status: PartiallyMutatable`
- All successful paths get `mutation_status: Mutatable`

### API Response Changes

#### Before (Confusing)
```json
{
  "description": "Path cannot be mutated - see example for explanation",
  "path_kind": "NotMutatable",
  "type": "unknown"
}
```

#### After (Clear)
```json
{
  "description": "Mutate the translation field of Transform",
  "mutation_status": "not_mutatable",
  "path_kind": "StructField",
  "type": "Vec3"
}
```

### Implementation Independence

This `MutationStatus` separation is **completely independent** from the single-pass recursion changes. It can be implemented:
- **Before** the precheck removal (current status quo + clean API)
- **After** the precheck removal (new architecture + clean API)
- **Separately** from precheck changes entirely

The two improvements address different architectural concerns and can be sequenced in any order.

## DESIGN-1: Error Propagation Consistency ✅
- **Category**: DESIGN
- **Status**: APPROVED - To be implemented  
- **Location**: plan-remove-precheck.md lines 218-225, 260-263
- **Issue Identified**: Plan shows Err(_) handling but build_paths should never return Err
- **Verdict**: CONFIRMED
- **Reasoning**: With MutationSupport, all errors are represented as NotMutatable paths inside Ok(Vec<...>)

### Approved Change:
Remove all Err(_) handling from build_paths calls. The method always returns Ok(Vec<MutationPathInternal>) where errors are represented as NotMutatable paths with MutationSupport details. This makes the plan consistent with TYPE-SYSTEM-1.

## DESIGN-2: Consistent Recursion Depth Management ✅
- **Category**: DESIGN
- **Status**: APPROVED - To be implemented
- **Location**: plan-remove-precheck.md lines 87-94, 126-132
- **Issue Identified**: Inconsistent depth increment logic - universally incremented then selectively used
- **Verdict**: CONFIRMED
- **Reasoning**: Clear rules needed for when to increment depth to prevent recursion limit issues

### Approved Change:
Only increment depth for container types that actually recurse into nested structures (Struct, Tuple, TupleStruct, Array, List, Map, Enum). Leaf types (Value) and simple wrappers (Option) preserve current depth. This is now implemented with a clear match expression that documents the logic.

## IMPLEMENTATION-1: Simplified Context Creation ✅
- **Category**: IMPLEMENTATION
- **Status**: APPROVED - To be implemented
- **Location**: plan-remove-precheck.md lines 163-167, 220, 260
- **Issue Identified**: Plan proposed adding unnecessary context creation helper methods
- **Verdict**: MODIFIED
- **Reasoning**: The proposed helper methods are over-engineering - simple constructor calls are clearer

### Approved Change:
Instead of adding new context creation methods:
- For arrays/lists: Use `MutationPathContext::new(RootOrField::root(&element_type), ctx.registry, None)` directly
- For tuples: Use existing `create_field_context(&index.to_string(), &element_type, None)`
- For structs: Use existing `create_field_context(&field_name, &field_type, None)`

## IMPLEMENTATION-2: Use Existing Helper Functions ✅
- **Category**: IMPLEMENTATION
- **Status**: APPROVED - To be implemented
- **Location**: plan-remove-precheck.md lines 155, 181, 210-230, 252-302, 325-348
- **Issue Identified**: Plan used pseudocode names for functions that already exist
- **Verdict**: CONFIRMED
- **Reasoning**: The functions aren't missing, they exist with different names in the implementation

### Approved Change:
Updated plan to use actual implementation method names:
- `extract_element_type()` → `MutationPathContext::extract_list_element_type()`
- `extract_struct_properties()` → `StructMutationBuilder::extract_properties()`
- `extract_field_type()` → `SchemaField::extract_field_type()`
- `extract_tuple_elements()` → `MutationPathContext::extract_tuple_element_types()`
- `build_field_mutation_path()` → `StructMutationBuilder::build_field_mutation_path()`
- `build_element_mutation_path()` → `TupleMutationBuilder::build_tuple_element_path()`
- `build_root_tuple_path()` → inline construction
- `build_enum_mutation_path()` → inline construction

## TYPE-SYSTEM-1: Use MutationSupport Enum Directly ✅
- **Category**: TYPE-SYSTEM  
- **Status**: APPROVED - To be implemented
- **Location**: plan-remove-precheck.md lines 84, 122, 145, 149, 194, 202, 215, 254, 301-347
- **Issue Identified**: Plan uses string literals for error reasons instead of the existing MutationSupport enum
- **Verdict**: CONFIRMED
- **Reasoning**: The existing MutationSupport enum already provides all needed error cases with proper type safety

### Approved Change:

1. **Replace all `ctx.require_schema()` error handling** (lines 122, 145, 194, 305, 314):
   - When `ctx.require_schema()` returns None, it means the type isn't in the registry
   - Change from: `"Schema not found"` 
   - Change to: `MutationSupport::NotInRegistry(ctx.type_name())`

2. **Remove impossible "Cannot determine" error paths** (lines 149, 202):
   - Delete these entirely - if we have a schema, we can always extract the type name
   - The real error is when the extracted type isn't in the registry (use `NotInRegistry`)

3. **Use MutationSupport for other error cases**:
   - `"Recursion limit exceeded"` → `MutationSupport::RecursionLimitExceeded(type_name)`
   - `"Missing serialization traits"` → `MutationSupport::MissingSerializationTraits(type_name)`

4. **Update function signature** (lines 325-339):
   - From: `fn build_not_mutatable_path(ctx, reason: &str)`
   - To: `fn build_not_mutatable_path(ctx, support: MutationSupport)`

### Implementation Notes:
Store MutationSupport directly in error_reason field and convert to string only at API boundary

## Files to Modify

### Single-Pass Recursion Changes
- **Primary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Secondary**: Update any other files that call the deleted precheck methods (if any exist)

### MutationStatus Separation Changes
- **Primary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/response_types.rs`
- **Secondary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Tertiary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`

## Deviation from Plan: SPECIFICATION-1 - Use of ? operator in build_paths
- **Category**: SPECIFICATION
- **Status**: ACCEPTED AS BUILT
- **Location**: /Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs lines 861 and 1357
- **Plan Specification**: Remove all Err(_) handling from build_paths calls as the method should always return Ok(Vec<MutationPathInternal>) with errors as NotMutatable paths
- **Actual Implementation**: The code uses `?` operator when calling build_paths(), but investigation revealed that no implementation of build_paths ever returns Err, making the `?` operators harmless and the behavior identical to the plan's intent
- **Verdict**: ALIGN RECOMMENDED (initially), then ACCEPT AS BUILT after investigation
- **Reasoning**: While the code technically uses `?` operators which could propagate errors, all implementations of build_paths always return Ok(...) with errors represented as NotMutatable paths. The Result return type appears to be vestigial, and the code already behaves exactly as the plan specifies. The `?` operators have no actual effect on control flow.
- **Decision**: Implementation deviation accepted and documented

## Design Review Skip Notes

### TYPE-SYSTEM-1: Option Type Special-Casing
**⚠️ PREJUDICE WARNING - DO NOT SUGGEST AGAIN**
The Option<T> handling is intentionally kept as-is to avoid scope creep. This will be addressed in a separate future plan as explicitly stated in lines 148-151. The current container-style handling works correctly and doesn't violate the core 'single recursion' performance goal.

### TYPE-SYSTEM-1 (2): Primitive counting logic in tuple mutability propagation
**Issue**: Review suggested replacing primitive counters (mutable_count, immutable_count) with a state enum in the `propagate_tuple_mixed_mutability` function
**Location**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs:1200-1241`
**Reason for skipping**: The current implementation using tuple pattern matching `(mutable_count, immutable_count)` is idiomatic Rust and clearly expresses the three cases: (0, _) for all immutable, (_, 0) for all mutable, and (_, _) for mixed. An enum abstraction would be over-engineering for this small, focused function that's already readable and maintainable.

### TYPE-SYSTEM-1 (3): Standalone utility functions should be methods on appropriate types
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: /Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs:34-43 and 1451-1471
- **Issue**: Review suggested moving `extract_type_ref_with_schema_field` to SchemaField and `build_not_mutatable_path` to MutationPathContext
- **Proposed Change**: Convert standalone utility functions to methods on their respective types
- **Verdict**: REJECTED
- **Reasoning**: The existing architecture is intentional: `extract_type_ref_with_schema_field` operates on generic JSON Values across contexts (SchemaField is just an enum of field names), and `build_not_mutatable_path` is already properly scoped within StructMutationBuilder impl block following the existing pattern where each builder has its own not-mutatable logic
- **Decision**: User elected to skip this recommendation

### TYPE-SYSTEM-2: Mixed success/error return violates Result type pattern
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: /Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs:1150-1197
- **Issue**: The `build_paths` method returns Vec<MutationPathInternal> mixing success and error states
- **Proposed Change**: Separate success and error cases using proper Result semantics
- **Verdict**: MODIFIED
- **Reasoning**: Already addressed by TYPE-SYSTEM-1 (use MutationSupport enum) and the existing MutationStatus separation (lines 361-533)
- **Decision**: User elected to skip - solution already present in plan

### TYPE-SYSTEM-3: Path construction logic should be centralized in a PathBuilder type
- **Status**: SKIPPED
- **Category**: TYPE-SYSTEM
- **Location**: /Users/natemccoy/rust/bevy_brp/plan-remove-precheck.md:189-283, 301-340
- **Issue**: Path building logic is scattered across multiple builder structs with duplicate pattern matching and construction logic
- **Proposed Change**: Add helper methods to MutationPathInternal for common construction patterns
- **Verdict**: MODIFIED
- **Reasoning**: The "duplication" is just normal struct construction with different field values - not a real problem needing abstraction
- **Decision**: User elected to skip - current approach is fine

### SIMPLIFICATION-1: MutationStatus separation adds complexity without clear immediate benefit
- **Status**: SKIPPED
- **Category**: SIMPLIFICATION
- **Location**: /Users/natemccoy/rust/bevy_brp/plan-remove-precheck.md:157-256
- **Issue**: Review suggested removing the MutationStatus separation as unnecessary complexity
- **Proposed Change**: Keep single-enum approach instead of separating MutationStatus from MutationPathKind
- **Verdict**: CONFIRMED
- **Reasoning**: While the separation adds API complexity, the user indicates there's a hidden complexity issue that the separation addresses, making it worth keeping
- **Decision**: User elected to skip this recommendation - MutationStatus separation remains in plan

### DESIGN-2: Inconsistent path building between container recursion and endpoint construction
- **Status**: SKIPPED
- **Category**: DESIGN
- **Location**: /Users/natemccoy/rust/bevy_brp/plan-remove-precheck.md:143-196, 202-242
- **Issue**: Container builders (Array, Map) recurse but don't build their own mutation paths
- **Proposed Change**: Add container replacement paths in addition to element paths
- **Verdict**: REJECTED
- **Reasoning**: False positive - actual implementation already creates both container replacement and element paths
- **Decision**: User elected to skip this recommendation

### SIMPLIFICATION-1 (2): Builder hierarchy follows identical patterns and could be simplified
- **Status**: SKIPPED
- **Category**: SIMPLIFICATION
- **Location**: plan-remove-precheck.md - Builder function specifications
- **Issue**: All builder functions follow identical setup patterns before diverging in their core logic
- **Proposed Change**: Extract common setup into shared helper or macro
- **Verdict**: REJECTED
- **Reasoning**: While all builders share the recursion limit check at the beginning, the rest of their logic is fundamentally different. The shared code is minimal (just the recursion check) and creating a complex abstraction to share this small piece of code would likely make the code harder to understand without meaningful benefit.
- **Decision**: User elected to skip this recommendation

## Implementation Requirements

### Single Atomic Update
- **No incremental rollout**: Implement all changes in one atomic commit
- **No fallback strategy**: Complete transformation or nothing
- **No sequencing**: All precheck method deletions and builder updates happen together
- **All-or-nothing**: Either the single recursion works completely or revert everything

### Testing Strategy
- **Agentic testing**: Will be conducted **outside this plan** using external test frameworks
- **Implementation requirement**: **STOP implementation when plan is complete** and hand off to agentic testing
- **No manual testing**: Plan implementation does not include writing or running tests
