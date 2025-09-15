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

### Step 1: Add New Trait Methods to MutationPathBuilder ✅ COMPLETED
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
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        Ok(vec![])  // Default: no children (leaf types)
    }

    /// Assemble parent example from child examples (post-order assembly)
    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<MutationPathDescriptor, Value>) -> Result<Value> {
        // Default: fallback to old build_schema_example for unmigrated builders
        Ok(self.build_schema_example(ctx, RecursionDepth::ZERO))
    }

    /// Controls path creation action for child elements
    ///
    /// Container types (Map, Set) that only support whole-value replacement
    /// should return PathAction::Skip to prevent exposing invalid mutation paths
    /// for child elements that cannot be individually addressed through BRP's
    /// reflection system.
    ///
    /// Default: PathAction::Create (include child paths for structured types)
    fn child_path_action(&self) -> PathAction {
        PathAction::Create  // Default: create paths for structured types like Struct, Array, Tuple
    }
```

}
```

### Step 2: Create ProtocolEnforcer Wrapper ✅ COMPLETED
Create new file `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`:


### Step 3: Update TypeKind::builder() to Check Migration Status  ✅ COMPLETED

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
            Self::Value => Box::new(ValueMutationBuilder),
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

### Step 4: Add Protocol Enforcer Module ✅ COMPLETED
In `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`:

```rust
mod protocol_enforcer;
use protocol_enforcer::ProtocolEnforcer;
```

### Phase 5a TODO List ✅ COMPLETED

1. ✅ Add is_migrated(), collect_children(), assemble_from_children() to MutationPathBuilder trait
   - **Adjustment**: Default `assemble_from_children()` returns `json!(null)` directly instead of calling `self.build_schema_example()` due to Rust trait object safety constraints
   - **Addition**: Added `child_path_action()` method to control whether child mutation paths are exposed in the final result. Default is `PathAction::Create` for structured types, but container types (Map, Set) override to `PathAction::Skip`
2. ✅ Create protocol_enforcer.rs file with ProtocolEnforcer implementation
   - **Updated**: ProtocolEnforcer now checks `child_path_action()` before extending the paths list with child paths
3. ✅ Update TypeKind::builder() to wrap migrated builders
   - **Adjustment**: Added explicit type annotation `let base_builder: Box<dyn MutationPathBuilder>` to resolve type compatibility between match arms
4. ✅ Add protocol_enforcer module to mod.rs
5. ✅ Stop and ask user to validate infrastructure setup

## Phase 5b: Remove ExampleBuilder

### Overview
Remove all ExampleBuilder references and replace with trait dispatch through TypeKind.

### Current ExampleBuilder Usage Locations
Found 16 references across 9 files that need conversion:

1. **value_builder.rs:26** - In `build_paths()` method
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
4. **IMPORTANT**: Keep `build_paths()` but make it return `Error::InvalidState`
   - This ensures it's never called when wrapped by ProtocolEnforcer
   - The error message should include the type name for debugging
   - Following error handling from plan-schema-error.md: no panics, only errors
5. **CRITICAL**: Update TypeKind::build_paths() to use trait dispatch for this type
   - Change from direct call: `BuilderName.build_paths(ctx, depth)`
   - To trait dispatch: `self.builder().build_paths(ctx, depth)`
   - This ensures the ProtocolEnforcer wrapper is used
6. **IMPORTANT**: Handle NotMutable conditions properly
   - Do NOT create NotMutable paths directly in the builder
   - Instead, return `Error::NotMutable(reason)` from `assemble_from_children()`
   - ProtocolEnforcer will catch these errors and create the NotMutable path
   - This ensures consistent NotMutable path formatting across all builders
7. Delete old methods (build_schema_example, static helper methods)

### Completed Builder Migrations (Phase 5b)

1. ✅ **ValueMutationBuilder** - Leaf type, no children, returns null
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/value_builder.rs`
   - Note: Previously named default_builder.rs, renamed to better reflect its purpose as the Value type builder
   - TypeKind: `Self::Value => self.builder().build_paths(ctx, builder_depth)`
2. ✅ **MapMutationBuilder** - Container with key/value pairs, skips child paths (BRP limitation)
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
   - TypeKind: Already using trait dispatch
   - Special: `child_path_action() -> PathAction::Skip`

3. ✅ **SetMutationBuilder** - Unordered collection, skips child paths (no stable indices)
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`
   - TypeKind: `Self::Set => self.builder().build_paths(ctx, builder_depth)`
   - Special: `child_path_action() -> PathAction::Skip`

4. ✅ **ListMutationBuilder** - Dynamic array with indexed access, exposes child paths
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/list_builder.rs`
   - TypeKind: `Self::List => self.builder().build_paths(ctx, builder_depth)`
   - Note: No `child_path_action()` override - exposes paths like `[0].field`

5. ✅ **ArrayMutationBuilder** - Fixed-size array with indexed access, exposes child paths
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/array_builder.rs`
   - TypeKind: `Self::Array => self.builder().build_paths(ctx, builder_depth)`
   - Note: No `child_path_action()` override - exposes paths like `[0].field`

6. ✅ **StructMutationBuilder** - Named fields, exposes field paths
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`
   - TypeKind: `Self::Struct => self.builder().build_paths(ctx, builder_depth)`
   - Note: No `child_path_action()` override - exposes paths like `.field_name`

7. ✅ **TupleMutationBuilder** - Fixed-length ordered elements with indexed access, exposes child paths
   - Implementation: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/tuple_builder.rs`
   - TypeKind: `Self::Tuple | Self::TupleStruct => self.builder().build_paths(ctx, builder_depth)`
   - Special: Handle wrapper detection in `assemble_from_children()` - returns NotMutable error for single-element Handle wrappers
   - Note: No `child_path_action()` override - exposes paths like `[0].field`

## Next: EnumMutationBuilder Migration

8. **EnumMutationBuilder** - Most complex variant-based type
   - Fix lines 170, 193, implement protocol methods
   - **COMPLEX VARIANT HANDLING - REQUIRES SPECIAL ATTENTION**:
     - **PRESERVE**: Core variant processing logic needs careful adaptation, NOT simple removal:
       - `extract_enum_variants()` - Extracts variant info from schema
       - `deduplicate_variant_signatures()` - Critical for avoiding duplicate work (only process unique signatures)
       - `process_tuple_variant()` - Handles tuple variants
       - `process_struct_variant()` - Handles struct variants
       - `build_enum_example_from_accumulated()` - Builds final consolidated example
     - **collect_children()**: Must return ONLY unique variant signatures to avoid redundant recursion
     - **assemble_from_children()**: Must handle different output formats:
       - Root path needs consolidated enum example format
       - Mutation paths have different format than root
     - **INVESTIGATION NEEDED**: Some responsibilities may need to move to ProtocolEnforcer
     - This is the most complex migration - may require additional design work when reached
   - **STATIC HELPER METHOD**:
     - `build_enum_spawn_example()` (line ~685) is a large static helper that will need removal/conversion
     - Contains BRP_MUTATION_KNOWLEDGE check at line ~694 that should be removed
   - **RECURSION CHECK REMOVAL**:
     - **SPECIFIC TO ENUM**: Has TWO depth.exceeds_limit() checks to remove:
       - Line 346 in `build_paths()` - returns NotMutable path
       - Line 420 in `build_schema_example()` - returns "..."
     - REMOVE both recursion limit checks - ProtocolEnforcer now handles all recursion limiting
     - Note: `build_schema_example()` and its helper `build_enum_spawn_example()` will be deleted entirely after migration
   - **Note**: No need to override `child_path_action()` - Enums expose variant field paths
   - **TypeKind**: Update `Self::Enum => self.builder().build_paths(ctx, builder_depth)`
   - **STOP and ask user to validate and discuss**
   - **CODE REVIEW**: After validation, stop and ask user to review the EnumMutationBuilder implementation before proceeding to next builder

## 🎯 Responsibilities After Migration

### ProtocolEnforcer Now Handles ALL:
1. **Depth limit checking** - No builder should check depth
2. **Registry validation** - No builder should validate registry presence
3. **Knowledge lookups** - No builder should access BRP_MUTATION_KNOWLEDGE
4. **NotMutable path creation** - Builders return errors, never create paths
5. **Mutation status computation** - Computed from child statuses with detailed breakdowns
   - Automatically determines Mutable vs PartiallyMutable vs NotMutable
   - Provides detailed mutation_status_reason with mutable/not_mutable path lists
   - See actual implementation in protocol_enforcer.rs::determine_parent_mutation_status()
6. **Child path filtering** - Via `child_path_action()` method

### Builders ONLY Handle:
1. **Identifying children** → Return `Result<Vec<PathKind>>` from `collect_children()`
2. **Assembling examples** → Return `Result<Value>` from `assemble_from_children()`
3. **Path control (optional)** → Override `child_path_action()` for containers

### Critical Pattern:
```rust
// Migrated builder pattern (Map/Set as examples)
impl MutationPathBuilder for SomeBuilder {
    fn is_migrated(&self) -> bool { true }

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        // Extract children, return PathKinds with type_name/parent_type
    }

    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<MutationPathDescriptor, Value>) -> Result<Value> {
        // Assemble parent from children
    }

    // Optional: for containers that don't expose child paths
    fn child_path_action(&self) -> PathAction {
        PathAction::Skip  // For Map, Set, etc.
    }
}
```

9. **mod.rs default trait** - Must be last
    - Fix line 79 default implementation
    - No TypeKind change needed (trait default)
    - do a `cargo build` to check for issues
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
- value_builder.rs
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
    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<MutationPathDescriptor, Value>) -> Value;
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
                mutation_status: MutationStatus::NotMutable,
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
                mutation_status: MutationStatus::mutable,
                error_reason: None,
            }]);
        }

        // 3. Collect children
        let child_path_kinds = self.collect_children(ctx);
        let mut all_paths = vec![];
        let mut child_examples = HashMap::<MutationPathDescriptor, Value>::new();

        // 4. Recurse to children
        for path_kind in child_path_kinds {
            let child_ctx = ctx.create_recursion_context(path_kind.clone(), PathAction::Create);
            let child_descriptor = path_kind.to_mutation_path_descriptor();

            let child_schema = child_ctx.require_registry_schema().unwrap_or_else(|_| &json!(null));
            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            let child_builder = child_kind.builder();

            let child_paths = child_builder.build_paths(&child_ctx, depth.increment())?;
            let child_example = child_paths.first()
                .map(|p| p.example.clone())
                .unwrap_or(json!(null));

            child_examples.insert(child_descriptor, child_example);
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
            mutation_status: MutationStatus::mutable,
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
**Completed**: 7 of 8 builders migrated
- ✅ ValueMutationBuilder, MapMutationBuilder, SetMutationBuilder, ListMutationBuilder, ArrayMutationBuilder, StructMutationBuilder, TupleMutationBuilder
- See implementations in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/`

**Remaining**: 1 builder + trait default
8. EnumMutationBuilder
9. mod.rs default trait implementation

For each migration:
- Remove ExampleBuilder references
- Implement is_migrated() -> true
- Implement collect_children()
- Implement assemble_from_children()
- **For Map and Set only**: Override child_path_action() -> PathAction::Skip with explanatory comment
- Keep build_paths() but make it return Error::InvalidState (no panics)
- **Update TypeKind::build_paths() to use self.builder() for this type**
- **Handle NotMutable conditions**: Return `Error::NotMutable(reason)` instead of creating paths directly
- Delete build_schema_example() override
- Delete static helper methods
- Remove ExampleBuilder import
- do a `cargo build` to check for issues
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

#### remove the check PathAction::Create in ProtcolEnforcer
once we migrate all builders to ProtocolEnforcer, then children of a parent that has a PathAction::Skip will not build paths for themselves.  At that time we can remove the check for PathAction::Create in ProtocolEnforcer - currently in the last step of the build_paths() method.

```rust
//
// Only extend paths when in Create mode
// WE NEED TO REMOVE THE CONDITIONAL WHEN ALL ARE MIGRATED
// as the new create_recursion_context will ensure children DON'T build paths
if matches!(ctx.path_action, PathAction::Create) {
    all_paths.extend(child_paths);
}

// should just be
// because if a child doesn't build any paths because it was using PathAction::Skip, then
// extend is a no-op
all_paths.extend(child_paths);

```

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
- Centralized NotMutable path creation in ProtocolEnforcer
- Builders return errors, ProtocolEnforcer handles path formatting
