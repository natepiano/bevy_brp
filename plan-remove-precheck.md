# Plan: Single-Pass Mutation Path Building

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

### Step 2: Rewrite TypeKind::build_paths() for Single Recursion

```rust
impl MutationPathBuilder for TypeKind {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        match self {
            Self::Struct => {
                // For each field, try build_paths() recursively
                // Collect results or NotMutatable as we go
                StructMutationBuilder.build_paths(ctx)
            }
            Self::Tuple | Self::TupleStruct => {
                // For each element, try build_paths() recursively
                TupleMutationBuilder.build_paths(ctx)
            }
            Self::Array => {
                // Try element_type.build_paths() once
                ArrayMutationBuilder.build_paths(ctx)
            }
            Self::List => {
                // Try element_type.build_paths() once
                ListMutationBuilder.build_paths(ctx)
            }
            Self::Map => {
                // Try value_type.build_paths() once
                MapMutationBuilder.build_paths(ctx)
            }
            Self::Enum => {
                // For each variant, try build_paths() recursively
                EnumMutationBuilder.build_paths(ctx)
            }
            Self::Option => {
                // Keep current Option handling (will be changed to enum in separate plan)
                // Try inner_type.build_paths() once
                DefaultMutationBuilder.build_paths(ctx)
            }
            Self::Value => {
                // Check serialization inline, no recursion needed
                if !ctx.value_type_has_serialization(ctx.type_name()) {
                    Ok(vec![build_not_mutatable_path(ctx, "Missing serialization traits")])
                } else {
                    DefaultMutationBuilder.build_paths(ctx)
                }
            }
        }
    }
}
```

### Step 3: Update Individual Builders for Single Recursion

Each builder becomes responsible for:
1. Extracting child types (fields, elements, etc.)
2. Calling `child_type.build_paths()` recursively
3. Handling the Result - building NotMutatable paths for failures

**Example - TupleMutationBuilder**:
```rust
impl MutationPathBuilder for TupleMutationBuilder {
    fn build_paths(&self, ctx: &MutationPathContext<'_>) -> Result<Vec<MutationPathInternal>> {
        let mut paths = Vec::new();

        for (index, element_info) in tuple_elements.enumerate() {
            let element_type = extract_element_type(element_info);

            // SINGLE RECURSION: Try to build paths for this element
            let element_kind = TypeKind::from_schema(element_schema, &element_type);
            match element_kind.build_paths(&element_ctx) {
                Ok(element_paths) if !all_not_mutatable(element_paths) => {
                    // Element is mutatable, add normal paths
                    paths.extend(build_element_paths(index, element_paths));
                }
                _ => {
                    // Element failed, add NotMutatable path
                    paths.push(build_not_mutatable_element_path(index, element_type));
                }
            }
        }

        paths
    }
}
```

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
    // ... existing fields (example, enum_variants, etc.)
}
```

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
