# Plan: Complete ExampleBuilder Removal and Implement Enforced Recursion Protocol

## Current State
We've successfully implemented Phases 1-4 of the spawn format unification:
- Broke circular dependencies with temporary ExampleBuilder
- Added trait methods for example building
- Switched to trait dispatch
- Unified spawn format to use root mutation path example
- Added compact JSON formatting

Now we need to complete Phase 5 (ExampleBuilder removal) and potentially implement a better recursion protocol.

## Phase 5a: Setup Protocol Enforcer Infrastructure

### Overview
Add the infrastructure for incremental migration to enforced protocol, allowing builders to migrate one at a time while removing ExampleBuilder references.

### Step 1: Add New Trait Methods to MutationPathBuilder
In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`:

```rust
pub trait MutationPathBuilder {
    // Existing methods stay unchanged
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth)
        -> Result<Vec<MutationPathInternal>>;

    fn build_example_with_knowledge(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // Existing implementation stays
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // Current default with ExampleBuilder - will be removed as we migrate
        use super::example_builder::ExampleBuilder;
        ExampleBuilder::build_example(ctx.type_name(), &ctx.registry, depth)
    }

    // NEW METHODS FOR PROTOCOL MIGRATION

    /// Indicates if this builder has been migrated to the new protocol
    fn is_migrated(&self) -> bool {
        false  // Default: not migrated
    }

    /// Collect child contexts that need recursion (for depth-first traversal)
    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        vec![]  // Default: no children (leaf types)
    }

    /// Assemble parent example from child examples (post-order assembly)
    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<String, Value>) -> Value {
        // Default: fallback to old build_schema_example for unmigrated builders
        self.build_schema_example(ctx, RecursionDepth::ZERO)
    }

    /// Controls whether child paths are included in the final mutation paths result
    ///
    /// Container types (Map, Set) that only support whole-value replacement should return false.
    /// This prevents exposing invalid mutation paths for child elements that cannot be
    /// individually addressed through BRP's reflection system.
    ///
    /// Default: true (include child paths for structured types)
    fn include_child_paths(&self) -> bool {
        true  // Default: include child paths (for structured types like Struct, Array, Tuple)
    }
```

}
```

### Step 2: Create ProtocolEnforcer Wrapper
Create new file `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`:

```rust
use std::collections::HashMap;
use serde_json::{Value, json};
use super::{MutationPathBuilder, RecursionContext, MutationPathInternal, MutationStatus};
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::type_kind::TypeKind;
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

pub struct ProtocolEnforcer {
    inner: Box<dyn MutationPathBuilder>,
}

impl ProtocolEnforcer {
    pub fn new(inner: Box<dyn MutationPathBuilder>) -> Self {
        Self { inner }
    }
}

impl MutationPathBuilder for ProtocolEnforcer {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth)
        -> Result<Vec<MutationPathInternal>> {
        // 1. Check depth limit for THIS level
        if depth.exceeds_limit() {
            return Ok(vec![MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example: json!({"error": "recursion limit exceeded"}),
                type_name: ctx.type_name().clone(),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::NotMutatable,
                error_reason: Some("Recursion limit exceeded".to_string()),
            }]);
        }

        // 2. Check knowledge for THIS level
        if let Some(example) = KnowledgeKey::find_example_for_type(ctx.type_name()) {
            return Ok(vec![MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example: example.clone(),
                type_name: ctx.type_name().clone(),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            }]);
        }

        // 3. Collect children for depth-first traversal
        let children = self.inner.collect_children(ctx);
        let mut all_paths = vec![];
        let mut child_examples = HashMap::new();

        // 4. Recurse to each child (they handle their own protocol)
        for (name, child_ctx) in children {
            // Get child's schema and create its builder
            let child_schema = child_ctx.require_schema()
                .unwrap_or(&json!(null));
            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            let child_builder = child_kind.builder();

            // Child handles its OWN depth increment and protocol
            // If child is migrated -> wrapped with ProtocolEnforcer
            // If not migrated -> uses old implementation
            let child_paths = child_builder.build_paths(&child_ctx, depth.increment())?;

            // Extract child's example from its root path
            let child_example = child_paths.first()
                .map(|p| p.example.clone())
                .unwrap_or(json!(null));

            child_examples.insert(name, child_example);

            // Only include child paths if the builder wants them
            // Container types (like Maps) don't want child paths exposed
            if self.inner.include_child_paths() {
                all_paths.extend(child_paths);
            }
        }

        // 5. Assemble THIS level from children (post-order)
        let parent_example = self.inner.assemble_from_children(ctx, child_examples);

        // 6. Add THIS level's path at the beginning
        all_paths.insert(0, MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example: parent_example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        });

        Ok(all_paths)
    }

    // Delegate all other methods to inner builder
    fn is_migrated(&self) -> bool {
        self.inner.is_migrated()
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        self.inner.collect_children(ctx)
    }

    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<String, Value>) -> Value {
        self.inner.assemble_from_children(ctx, children)
    }

    fn build_example_with_knowledge(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        self.inner.build_example_with_knowledge(ctx, depth)
    }

    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        self.inner.build_schema_example(ctx, depth)
    }
}
```

### Step 3: Update TypeKind::builder() to Check Migration Status
In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/type_kind.rs`:

```rust
use super::protocol_enforcer::ProtocolEnforcer;

impl TypeKind {
    pub fn builder(&self) -> Box<dyn MutationPathBuilder> {
        let base_builder = match self {
            Self::Struct => Box::new(StructMutationBuilder),
            Self::Array => Box::new(ArrayMutationBuilder),
            Self::List => Box::new(ListMutationBuilder),
            Self::Set => Box::new(SetMutationBuilder),
            Self::Map => Box::new(MapMutationBuilder),
            Self::Tuple | Self::TupleStruct => Box::new(TupleMutationBuilder),
            Self::Enum => Box::new(EnumMutationBuilder),
            Self::Value => Box::new(DefaultMutationBuilder),
        };

        // Wrap with protocol enforcer if migrated
        if base_builder.is_migrated() {
            Box::new(ProtocolEnforcer::new(base_builder))
        } else {
            base_builder
        }
    }
}
```

### Step 4: Add Protocol Enforcer Module
In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`:

```rust
mod protocol_enforcer;
use protocol_enforcer::ProtocolEnforcer;
```

### Phase 5a TODO List ✅ COMPLETED

1. ✅ Add is_migrated(), collect_children(), assemble_from_children() to MutationPathBuilder trait
   - **Adjustment**: Default `assemble_from_children()` returns `json!(null)` directly instead of calling `self.build_schema_example()` due to Rust trait object safety constraints
   - **Addition**: Added `include_child_paths()` method to control whether child mutation paths are exposed in the final result. Default is `true` for structured types, but container types (Map, Set) override to `false`
2. ✅ Create protocol_enforcer.rs file with ProtocolEnforcer implementation
   - **Updated**: ProtocolEnforcer now checks `include_child_paths()` before extending the paths list with child paths
3. ✅ Update TypeKind::builder() to wrap migrated builders
   - **Adjustment**: Added explicit type annotation `let base_builder: Box<dyn MutationPathBuilder>` to resolve type compatibility between match arms
4. ✅ Add protocol_enforcer module to mod.rs
5. ✅ Stop and ask user to validate infrastructure setup

## Phase 5b: Remove ExampleBuilder

### Overview
Remove all ExampleBuilder references and replace with trait dispatch through TypeKind.

### Current ExampleBuilder Usage Locations
Found 16 references across 9 files that need conversion:

1. **default_builder.rs:26** - In `build_paths()` method
2. **map_builder.rs:161** - Error path fallback
3. **array_builder.rs:220** - Static method for array elements
4. **list_builder.rs:165** - Static method for list elements
5. **set_builder.rs:120** - Static method for set elements
6. **tuple_builder.rs:390** - In `build_schema_example()`
7. **tuple_builder.rs:285,317** - Static methods
8. **struct_builder.rs:403** - Static method for struct fields
9. **map_builder.rs:79-80** - In `build_schema_example()` for key/value
10. **map_builder.rs:132-133** - Static methods for key/value
11. **enum_builder.rs:170,193** - In `build_schema_example()`
12. **mod.rs:79** - Default trait implementation (must be done last)

### Replacement Pattern

#### For references in `build_paths()`:
```rust
// OLD:
let example = ExampleBuilder::build_example(ctx.type_name(), &ctx.registry, depth);

// NEW:
let example = self.build_example_with_knowledge(ctx, depth);
```

#### For references in `build_schema_example()` (recursive calls):
```rust
// OLD:
let field_example = ExampleBuilder::build_example(&field_type, &ctx.registry, depth);

// NEW:
let field_kind = TypeKind::from_schema(field_schema, &field_type);
let field_example = field_kind.builder().build_example_with_knowledge(&field_ctx, depth);
```

#### For static methods:
```rust
// OLD (in static method):
let item_example = ExampleBuilder::build_example(&item_type_name, registry, depth.increment());

// NEW (convert to use registry to get schema first):
registry.get(&item_type_name)
    .map_or(json!(null), |item_schema| {
        let item_kind = TypeKind::from_schema(item_schema, &item_type_name);
        // Note: static methods don't have context, so we need to create one
        // This is why static methods are problematic and should be removed eventually
        item_kind.builder().build_schema_example(&temp_ctx, depth.increment())
    })
```

### Important Notes

1. **build_example_with_knowledge vs build_schema_example**:
   - `build_example_with_knowledge()` - Entry point that checks knowledge FIRST
   - `build_schema_example()` - Type-specific logic ONLY (no knowledge checks)
   - Never call `build_schema_example()` directly except from within `build_example_with_knowledge()`

2. **Default trait implementation in mod.rs**:
   - Must be fixed LAST after all builders are converted
   - Currently falls back to ExampleBuilder for types not yet migrated

## Phase 5b: Migrate Each Builder Individually

### Migration Pattern for Each Builder
Each builder migration follows this pattern:
1. Remove ExampleBuilder usage
2. Implement protocol methods
3. Set is_migrated() to true
4. **IMPORTANT**: Keep `build_paths()` but make it panic with tracing::error! and panic!
   - This ensures it's never called when wrapped by ProtocolEnforcer
   - The panic message should include the type name for debugging
5. **CRITICAL**: Update TypeKind::build_paths() to use trait dispatch for this type
   - Change from direct call: `BuilderName.build_paths(ctx, depth)`
   - To trait dispatch: `self.builder().build_paths(ctx, depth)`
   - This ensures the ProtocolEnforcer wrapper is used
6. Delete old methods (build_schema_example, static helper methods)

### Builder 1: DefaultMutationBuilder ✅ COMPLETED

**Status**: Migration complete with fixes applied
**Commit**: 87d9e77 (WIP: Camera crash fixed but enum regression in spawn_format)
**TypeKind Dispatch**: ✅ Updated to use `self.builder().build_paths()` for Value type

#### Final Implementation:
```rust
impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(&self, ctx: &RecursionContext, _depth: RecursionDepth)
        -> Result<Vec<MutationPathInternal>> {
        tracing::error!("DefaultMutationBuilder::build_paths() called directly! Type: {}", ctx.type_name());
        panic!("DefaultMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}", ctx.type_name());
    }

    fn is_migrated(&self) -> bool {
        true // MIGRATED!
    }

    fn collect_children(&self, _ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        vec![] // Leaf type - no children
    }

    fn assemble_from_children(&self, _ctx: &RecursionContext, _children: HashMap<String, Value>) -> Value {
        json!(null) // Leaf types return null, knowledge handled by ProtocolEnforcer
    }
}
```

**Lessons Learned**:
- ✅ Protocol enforcer pattern works correctly
- ✅ Panic guards prevent direct build_paths() calls
- ✅ Simple leaf type implementation confirmed
- ⚠️ Revealed output format regressions requiring fixes in enum/struct builders
### Builder 2: StructMutationBuilder (Container Type Example)

#### After Migration:

**CRITICAL**: Also update TypeKind::build_paths() in type_kind.rs:
```rust
// Change line for Struct type:
Self::Struct => self.builder().build_paths(ctx, builder_depth),
```

```rust
impl MutationPathBuilder for StructMutationBuilder {
    fn build_paths(&self, ctx: &RecursionContext, _depth: RecursionDepth)
        -> Result<Vec<MutationPathInternal>> {
        // IMPORTANT: Add panic to ensure this is never called when migrated
        tracing::error!("StructMutationBuilder::build_paths() called directly! Type: {}", ctx.type_name());
        panic!("StructMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}", ctx.type_name());
    }

    fn is_migrated(&self) -> bool {
        true  // MIGRATED!
    }

    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        // Identify all struct fields that need recursion
        ctx.require_schema()
            .and_then(|s| s.get_field(SchemaField::Properties))
            .and_then(Value::as_object)
            .map_or(vec![], |properties| {
                properties.iter()
                    .filter_map(|(field_name, field_value)| {
                        SchemaField::extract_field_type(field_value)
                            .map(|field_type| {
                                let field_path_kind = PathKind::new_struct_field(
                                    field_name.clone(),
                                    field_type,
                                    ctx.type_name().clone(),
                                );
                                let field_ctx = ctx.create_field_context(field_path_kind);
                                (field_name.clone(), field_ctx)
                            })
                    })
                    .collect()
            })
    }

    fn assemble_from_children(&self, _ctx: &RecursionContext, children: HashMap<String, Value>) -> Value {
        // Assemble struct from field examples
        let mut obj = serde_json::Map::new();
        for (field_name, example) in children {
            obj.insert(field_name, example);
        }
        json!(obj)
    }

    // DELETE build_schema_example() - no longer needed
    // DELETE build_struct_example_from_properties() static method
}
```

### Complete Migration Order

Following the same order as the original ExampleBuilder removal:

1. ✅ **DefaultMutationBuilder** - COMPLETED in commit 87d9e77
   - ✅ TypeKind: `Self::Value => self.builder().build_paths(ctx, builder_depth)`
   - ✅ **Note**: No need to override `include_child_paths()` - Default/Value types are leaf nodes with no children

2. ✅ **MapMutationBuilder** - COMPLETED in commit e465607
   - ✅ Fixed line 161 error path
   - ✅ Implemented full protocol methods (is_migrated, collect_children, assemble_from_children)
   - ✅ **Special**: Added `include_child_paths() -> false` override to prevent exposing invalid child mutation paths
   - ✅ **TypeKind**: Already using trait dispatch
   - ✅ Comment added explaining why Maps don't expose child paths (BRP doesn't support string keys for map mutations)

3. **SetMutationBuilder** - Single child type
   - Fix line 120 static method, implement protocol methods
   - **ERROR HANDLING**: 
     - Review `set_builder.rs:58` - `return json!(null);` fallback
     - Use `Error::InvalidState` for protocol violations (missing required children)
     - Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
     - Follow patterns in DefaultMutationBuilder and MapMutationBuilder for reference
     - Update `assemble_from_children` to return `Result<Value>` not `Value`
   - **Special**: Add `include_child_paths() -> false` override (like MapMutationBuilder) with comment explaining Sets are terminal mutation points
   - **TypeKind**: Update `Self::Set => self.builder().build_paths(ctx, builder_depth)`
   - Run build-check.sh
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the SetMutationBuilder implementation before proceeding to next builder

4. **ListMutationBuilder** - Single child type
   - Fix line 165 static method, implement protocol methods
   - **ERROR HANDLING**: 
     - Review `list_builder.rs:113` - `return json!(null);` fallback
     - Use `Error::InvalidState` for protocol violations (missing required children)
     - Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
     - Follow patterns in DefaultMutationBuilder and MapMutationBuilder for reference
     - Update `assemble_from_children` to return `Result<Value>` not `Value`
   - **Note**: No need to override `include_child_paths()` - Lists expose indexed element paths like `[0].field`
   - **TypeKind**: Update `Self::List => self.builder().build_paths(ctx, builder_depth)`
   - Run build-check.sh
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the ListMutationBuilder implementation before proceeding to next builder

5. **ArrayMutationBuilder** - Single child type
   - Fix line 220 static method, implement protocol methods
   - **ERROR HANDLING**: 
     - Review `array_builder.rs:139` - `return json!(null);` fallback
     - Use `Error::InvalidState` for protocol violations (missing required children)
     - Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
     - Follow patterns in DefaultMutationBuilder and MapMutationBuilder for reference
     - Update `assemble_from_children` to return `Result<Value>` not `Value`
   - **Note**: No need to override `include_child_paths()` - Arrays expose indexed element paths
   - **TypeKind**: Update `Self::Array => self.builder().build_paths(ctx, builder_depth)`
   - Run build-check.sh
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the ArrayMutationBuilder implementation before proceeding to next builder

6. **TupleMutationBuilder** - Multiple children
   - Fix lines 390, 285, 317, implement protocol methods
   - **ERROR HANDLING**: 
     - Review `tuple_builder.rs:193` - `return json!(null);` fallback
     - Use `Error::InvalidState` for protocol violations (missing required children)
     - Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
     - Follow patterns in DefaultMutationBuilder and MapMutationBuilder for reference
     - Update `assemble_from_children` to return `Result<Value>` not `Value`
   - **Note**: No need to override `include_child_paths()` - Tuples expose indexed element paths
   - **TypeKind**: Update `Self::Tuple | Self::TupleStruct => self.builder().build_paths(ctx, builder_depth)`
   - Run build-check.sh
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the TupleMutationBuilder implementation before proceeding to next builder

7. **StructMutationBuilder** - Named fields
   - Fix line 403 static method, implement protocol methods
   - **ERROR HANDLING**: 
     - Review multiple `json!` fallbacks at lines 302, 306, 509, 513, 544, 548
     - Use `Error::InvalidState` for protocol violations (missing required children)
     - Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
     - Follow patterns in DefaultMutationBuilder and MapMutationBuilder for reference
     - Update `assemble_from_children` to return `Result<Value>` not `Value`
   - **Note**: No need to override `include_child_paths()` - Structs expose field paths
   - **TypeKind**: Update `Self::Struct => self.builder().build_paths(ctx, builder_depth)`
   - Run build-check.sh
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the StructMutationBuilder implementation before proceeding to next builder

8. **EnumMutationBuilder** - Most complex
   - Fix lines 170, 193, implement protocol methods
   - **ERROR HANDLING**: 
     - Review `enum_builder.rs:592, 597` - `return json!(null);` and `return json!("...");` fallbacks
     - Use `Error::InvalidState` for protocol violations (missing required children)
     - Use `Error::SchemaProcessing` for data processing issues (failed serialization, invalid schema)
     - Follow patterns in DefaultMutationBuilder and MapMutationBuilder for reference
     - Update `assemble_from_children` to return `Result<Value>` not `Value`
   - **Note**: No need to override `include_child_paths()` - Enums expose variant field paths
   - **TypeKind**: Update `Self::Enum => self.builder().build_paths(ctx, builder_depth)`
   - Run build-check.sh
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the EnumMutationBuilder implementation before proceeding to next builder

9. **mod.rs default trait** - Must be last
    - Fix line 79 default implementation
    - No TypeKind change needed (trait default)
    - Run build-check.sh
    - **STOP and ask user to validate and discuss**
    - **CODE REVIEW**: After validation, stop and ask user to review the final trait default implementation
25. Remove all ExampleBuilder import statements
26. Remove all static example building methods from builders
27. Delete example_builder.rs file entirely
28. Delete TypeGuide::build_type_example and build_spawn_format methods
29. Stop and ask user to validate final cleanup
30. DISCUSSION: Consider removing MutationPathBuilder implementation from TypeKind
31. FINAL VALIDATION: Install MCP and ask user to reconnect for complete validation

### Phase 5b Cleanup Details

#### Step 25: Remove all ExampleBuilder import statements
Remove these imports from all files:
- `use crate::brp_tools::brp_type_guide::example_builder::ExampleBuilder;`

Files to clean:
- default_builder.rs
- array_builder.rs
- list_builder.rs
- set_builder.rs
- tuple_builder.rs
- struct_builder.rs
- map_builder.rs
- enum_builder.rs
- mutation_path_builder/mod.rs

#### Step 26: Remove all static example building methods
Delete these static methods as they're no longer needed:
- `ArrayMutationBuilder::build_array_example_static()`
- `TupleMutationBuilder::build_tuple_example_static()`
- `TupleMutationBuilder::build_tuple_struct_example_static()`
- `StructMutationBuilder::build_struct_example_from_properties()`
- `ListMutationBuilder::build_list_example_static()`
- `SetMutationBuilder::build_set_example_static()`
- `MapMutationBuilder::build_map_example_static()`
- `EnumMutationBuilder::build_enum_example()`

#### Step 27: Delete example_builder.rs
- Delete file: `mcp/src/brp_tools/brp_type_guide/example_builder.rs`
- Remove module declaration from `mcp/src/brp_tools/brp_type_guide/mod.rs`:
  ```rust
  // DELETE: mod example_builder;
  ```

#### Step 28: Delete `TypeGuide` methods
In `mcp/src/brp_tools/brp_type_guide/"type_guide".rs`, delete:
- `build_type_example()` method (lines ~310-410)
- `build_spawn_format()` method (if it still exists)
- Any helper methods only used by these
- Update imports to remove unused dependencies

#### Step 30: DISCUSSION - TypeKind MutationPathBuilder
Consider whether to:
- Remove `impl MutationPathBuilder for TypeKind`
- Change `TypeGuide` to call `type_kind.builder().build_paths()` instead of `type_kind.build_paths()`
- This would make TypeKind purely a dispatcher/factory

## Phase 6: Atomic Change to PathBuilder Pattern

### Overview
Once all builders are migrated and using ProtocolEnforcer, make one atomic change to move protocol into trait.

### Step 1: Create New PathBuilder Trait
In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`:

```rust
/// Trait that builders actually implement
pub trait PathBuilder {
    fn collect_children(&self, ctx: &RecursionContext) -> Vec<(String, RecursionContext)>;
    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<String, Value>) -> Value;
}

/// Blanket implementation that provides build_paths with enforced protocol
impl<T: PathBuilder> MutationPathBuilder for T {
    fn build_paths(&self, ctx: &RecursionContext, depth: RecursionDepth)
        -> Result<Vec<MutationPathInternal>> {
        // EXACT copy of ProtocolEnforcer::build_paths logic
        // 1. Check depth limit
        if depth.exceeds_limit() {
            return Ok(vec![MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example: json!({"error": "recursion limit exceeded"}),
                type_name: ctx.type_name().clone(),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::NotMutatable,
                error_reason: Some("Recursion limit exceeded".to_string()),
            }]);
        }

        // 2. Check knowledge
        if let Some(example) = KnowledgeKey::find_example_for_type(ctx.type_name()) {
            return Ok(vec![MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example: example.clone(),
                type_name: ctx.type_name().clone(),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutatable,
                error_reason: None,
            }]);
        }

        // 3. Collect children
        let children = self.collect_children(ctx);
        let mut all_paths = vec![];
        let mut child_examples = HashMap::new();

        // 4. Recurse to children
        for (name, child_ctx) in children {
            let child_schema = child_ctx.require_schema().unwrap_or(&json!(null));
            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            let child_builder = child_kind.builder();

            let child_paths = child_builder.build_paths(&child_ctx, depth.increment())?;
            let child_example = child_paths.first()
                .map(|p| p.example.clone())
                .unwrap_or(json!(null));

            child_examples.insert(name, child_example);
            all_paths.extend(child_paths);
        }

        // 5. Assemble parent
        let parent_example = self.assemble_from_children(ctx, child_examples);

        // 6. Add parent path
        all_paths.insert(0, MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example: parent_example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        });

        Ok(all_paths)
    }

    // These methods are no longer needed
    fn is_migrated(&self) -> bool { true }
    fn build_example_with_knowledge(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // No longer used
        json!(null)
    }
    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // No longer used
        json!(null)
    }
}
```

### Step 2: Change All Builders from MutationPathBuilder to PathBuilder

For each builder, change:
```rust
// OLD
impl MutationPathBuilder for StructMutationBuilder {
    fn is_migrated(&self) -> bool { true }
    fn collect_children(&self, ...) -> Vec<...> { ... }
    fn assemble_from_children(&self, ...) -> Value { ... }
}

// NEW
impl PathBuilder for StructMutationBuilder {
    fn collect_children(&self, ...) -> Vec<...> { ... }
    fn assemble_from_children(&self, ...) -> Value { ... }
}
```

### Step 3: Update TypeKind::builder()
```rust
impl TypeKind {
    pub fn builder(&self) -> Box<dyn MutationPathBuilder> {
        match self {
            Self::Struct => Box::new(StructMutationBuilder),
            Self::Array => Box::new(ArrayMutationBuilder),
            // ... etc
            // No more ProtocolEnforcer wrapping!
        }
    }
}
```

### Step 4: Delete Obsolete Code
1. Delete `protocol_enforcer.rs`
2. Remove `mod protocol_enforcer;` from mod.rs
3. Remove `is_migrated()`, `build_example_with_knowledge()`, `build_schema_example()` from trait
4. Delete `example_builder.rs`
5. Remove all ExampleBuilder imports

### Phase 6 TODO List
1. Create PathBuilder trait with collect_children and assemble_from_children
2. Add blanket impl<T: PathBuilder> MutationPathBuilder for T with protocol logic
3. Change all `impl MutationPathBuilder` to `impl PathBuilder`
4. Update TypeKind::builder() to remove ProtocolEnforcer wrapping
5. Delete protocol_enforcer.rs and its module declaration
6. Remove obsolete trait methods
7. Delete example_builder.rs
8. Remove all ExampleBuilder imports
9. Final validation

## Phase 7: Final Cleanup

### Cleanup Tasks
1. Remove TypeKind's MutationPathBuilder implementation (optional)
2. Delete all static example methods from builders
3. Delete TypeGuide::build_type_example() and build_spawn_format()
4. Clean up all unused imports
5. Run final validation


## Complete Execution Order

### Phase 5a: Setup Infrastructure ✅ COMPLETED
1. ✅ Add is_migrated(), collect_children(), assemble_from_children() to MutationPathBuilder trait
2. ✅ Create protocol_enforcer.rs with ProtocolEnforcer implementation
3. ✅ Update TypeKind::builder() to check is_migrated() and wrap if true
4. ✅ Add protocol_enforcer module to mod.rs
5. ✅ Validate infrastructure compiles

### Phase 5b: Incremental Builder Migration
For each builder in order:
1. DefaultMutationBuilder (✅ completed)
2. MapMutationBuilder (✅ completed)
3. SetMutationBuilder
4. ListMutationBuilder
5. ArrayMutationBuilder
6. TupleMutationBuilder
7. StructMutationBuilder
8. EnumMutationBuilder
9. mod.rs default trait implementation

For each migration:
- Remove ExampleBuilder references
- Implement is_migrated() -> true
- Implement collect_children()
- Implement assemble_from_children()
- **For Map and Set only**: Override include_child_paths() -> false with explanatory comment
- Keep build_paths() but make it panic (with tracing::error! and panic!)
- **Update TypeKind::build_paths() to use self.builder() for this type**
- Delete build_schema_example() override
- Delete static helper methods
- Remove ExampleBuilder import
- Run build-check.sh
- **STOP: Ask user to validate and discuss each builder migration**

#### TypeKind::build_paths() Dispatch Pattern
In `type_kind.rs`, update the match arm for each migrated type:
```rust
// BEFORE migration (direct call):
Self::Map => MapMutationBuilder.build_paths(ctx, builder_depth),

// AFTER migration (trait dispatch through builder()):
Self::Map => self.builder().build_paths(ctx, builder_depth),
```
This ensures the ProtocolEnforcer wrapper is used for migrated builders.

### Phase 6: Atomic Change to PathBuilder
1. Create PathBuilder trait
2. Add blanket impl<T: PathBuilder> MutationPathBuilder for T
3. Change all builders from impl MutationPathBuilder to impl PathBuilder
4. Update TypeKind::builder() to remove ProtocolEnforcer wrapping
5. Delete protocol_enforcer.rs
6. Remove obsolete trait methods
7. Validate

### Phase 7: Final Cleanup
1. Delete example_builder.rs
2. Delete TypeGuide::build_type_example() and build_spawn_format()
3. Remove all static example methods
4. Clean up imports
5. Optional: Remove MutationPathBuilder from TypeKind
6. Final validation

## End Result
- Clean, enforced recursion protocol
- No way for builders to violate protocol
- ExampleBuilder completely removed
- Simple builder implementations (just collect_children and assemble_from_children)
- All complexity centralized in trait's build_paths()
