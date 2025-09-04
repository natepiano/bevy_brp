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
            return Ok(vec![build_not_mutatable_path(ctx, "Recursion limit exceeded")]);
        }
        
        let next_depth = depth.increment();
        
        match self {
            Self::Struct => {
                // For each field, try build_paths() recursively with incremented depth
                StructMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::Tuple | Self::TupleStruct => {
                // For each element, try build_paths() recursively with incremented depth
                TupleMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::Array => {
                // Try element_type.build_paths() with incremented depth
                ArrayMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::List => {
                // Try element_type.build_paths() with incremented depth
                ListMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::Map => {
                // Try value_type.build_paths() with incremented depth
                MapMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::Enum => {
                // For each variant, try build_paths() recursively with incremented depth
                EnumMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::Option => {
                // Keep current Option handling (will be changed to enum in separate plan)
                // Try inner_type.build_paths() with incremented depth
                DefaultMutationBuilder.build_paths(ctx, next_depth)
            }
            Self::Value => {
                // Check serialization inline, no recursion needed (no depth increment)
                if !ctx.value_type_has_serialization(ctx.type_name()) {
                    Ok(vec![build_not_mutatable_path(ctx, "Missing serialization traits")])
                } else {
                    DefaultMutationBuilder.build_paths(ctx, depth) // Pass current depth, no increment
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
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        let Some(element_type) = extract_element_type(schema) else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Cannot determine array element type")]);
        };
        
        // RECURSE DEEPER - don't stop at array level
        let element_schema = ctx.get_type_schema(&element_type)?;
        let element_kind = TypeKind::from_schema(element_schema, &element_type);
        let element_ctx = ctx.with_element_context(&element_type);
        
        // Continue recursion to actual mutation endpoints
        element_kind.build_paths(&element_ctx, depth)  // depth already incremented by TypeKind
    }
}

// MapMutationBuilder - Extract value type and recurse
impl MutationPathBuilder for MapMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        let Some(value_type) = extract_map_value_type(schema) else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Cannot determine map value type")]);
        };
        
        // Maps are currently treated as opaque (cannot mutate individual keys)
        // So we just validate value type has serialization and build a single path
        if !ctx.value_type_has_serialization(&value_type) {
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Map values lack serialization")]);
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
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        let mut paths = Vec::new();
        let properties = extract_struct_properties(schema);
        
        for (field_name, field_info) in properties {
            let Some(field_type) = extract_field_type(field_info) else {
                paths.push(build_not_mutatable_field(field_name, "Cannot determine field type"));
                continue;
            };
            
            // Build actual mutation path for this field
            let field_ctx = ctx.create_field_context(&field_name, &field_type);
            
            // Check if field is a Value type needing serialization
            let field_schema = ctx.get_type_schema(&field_type)?;
            let field_kind = TypeKind::from_schema(field_schema, &field_type);
            
            if matches!(field_kind, TypeKind::Value) {
                if !ctx.value_type_has_serialization(&field_type) {
                    paths.push(build_not_mutatable_field(field_name, "Missing serialization"));
                } else {
                    paths.push(build_field_mutation_path(field_name, field_type));
                }
            } else {
                // Recurse for nested containers or structs
                match field_kind.build_paths(&field_ctx, depth) {
                    Ok(field_paths) => paths.extend(field_paths),
                    Err(_) => paths.push(build_not_mutatable_field(field_name, "Cannot mutate field"))
                }
            }
        }
        
        Ok(paths)
    }
}

// TupleMutationBuilder - Creates paths for tuple elements
impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        let mut paths = Vec::new();
        let elements = extract_tuple_elements(schema);
        
        // Build root tuple path first
        paths.push(build_root_tuple_path(ctx));
        
        // Build paths for each element
        for (index, element_type) in elements.enumerate() {
            let element_ctx = ctx.create_element_context(index, &element_type);
            let element_schema = ctx.get_type_schema(&element_type)?;
            let element_kind = TypeKind::from_schema(element_schema, &element_type);
            
            // Similar to struct fields - check Value types for serialization
            if matches!(element_kind, TypeKind::Value) {
                if !ctx.value_type_has_serialization(&element_type) {
                    paths.push(build_not_mutatable_element(index, "Missing serialization"));
                } else {
                    paths.push(build_element_mutation_path(index, element_type));
                }
            } else {
                // Recurse for nested types
                match element_kind.build_paths(&element_ctx, depth) {
                    Ok(element_paths) => paths.extend(element_paths),
                    Err(_) => paths.push(build_not_mutatable_element(index, "Cannot mutate element"))
                }
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
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        // Enums can always be replaced entirely - no recursion into variants needed
        Ok(vec![build_enum_mutation_path(ctx, extract_enum_variants(schema))])
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
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        // Rest of implementation...
    }
}

impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>, depth: RecursionDepth) -> Result<Vec<MutationPathInternal>> {
        let Some(schema) = ctx.require_schema() else {
            // CONSISTENT: Return NotMutatable path, not empty vector  
            return Ok(vec![Self::build_not_mutatable_path(ctx, "Schema not found")]);
        };
        
        // Rest of implementation...
    }
}

// Add standard helper method for consistent error path construction:
impl MutationPathBuilder {
    fn build_not_mutatable_path(ctx: &MutationPathContext<'_>, reason: &str) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.current_path(),
            example: json!({
                "NotMutatable": reason,
                "agent_directive": format!("This {} cannot be mutated - {}", ctx.type_description(), reason)
            }),
            enum_variants: None,
            type_name: ctx.type_name().clone(),
            path_kind: MutationPathKind::NotMutatable,  // Will become mutation_status with MutationStatus separation
            error_reason: Some(reason.to_string()),      // NEW: Structured error reason
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

## Files to Modify

### Single-Pass Recursion Changes
- **Primary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Secondary**: Update any other files that call the deleted precheck methods (if any exist)

### MutationStatus Separation Changes
- **Primary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/response_types.rs`
- **Secondary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
- **Tertiary**: `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`

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

### SIMPLIFICATION-1: MutationStatus separation adds complexity without clear immediate benefit
- **Status**: SKIPPED
- **Category**: SIMPLIFICATION
- **Location**: /Users/natemccoy/rust/bevy_brp/plan-remove-precheck.md:157-256
- **Issue**: Review suggested removing the MutationStatus separation as unnecessary complexity
- **Proposed Change**: Keep single-enum approach instead of separating MutationStatus from MutationPathKind
- **Verdict**: CONFIRMED
- **Reasoning**: While the separation adds API complexity, the user indicates there's a hidden complexity issue that the separation addresses, making it worth keeping
- **Decision**: User elected to skip this recommendation - MutationStatus separation remains in plan

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
