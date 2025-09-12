# Plan: Eliminate Path Creation Waste Through Protocol-Driven Recursion

**Migration Strategy: Phased** - This implementation requires sequential steps due to complex interdependencies between RecursionContext changes, trait signature updates, and ProtocolEnforcer rewrite. Each step validates the previous before proceeding.

## Core Design Principle
The goal is to have ProtocolEnforcer handle ALL recursion and path creation, while migrated builders ONLY provide examples and mutation status. This eliminates wasted path creation and simplifies builders.

## Problem Statement

### Current State (INCORRECT)
When recursing through types like `HashMap<String, Transform>`:
1. Transform's ProtocolEnforcer creates 4+ MutationPathInternal objects
2. Map's ProtocolEnforcer extracts just the example from the first path
3. All 4+ paths are discarded because `include_child_paths()` returns false
4. **Root cause**: Migrated builders create RecursionContexts themselves instead of ProtocolEnforcer creating them with proper flags

### Correct Design (WHAT WE WANT)
1. **ProtocolEnforcer creates ALL RecursionContexts** - Builders should NOT create contexts
2. **RecursionContext gets a new field**: `path_action: PathAction` enum
3. **Builders only return examples and status** - No path creation in builders
4. **Path creation happens ONLY in ProtocolEnforcer** - Based on context's `path_action` field

## Key Design Innovation: Positional Ordering

The critical insight is that we don't need arbitrary string labels to match children with their examples. Instead:

1. **Builders define order** - Each builder returns PathKinds in a specific order it defines
2. **ProtocolEnforcer preserves order** - Recurses to children in order, collects values in order
3. **Assembly uses position** - Builder receives values in same order, knows what each position means

Example for MapMutationBuilder:
- Returns `vec![key_path_kind, value_path_kind]`
- Receives `vec![key_value, value_value]`
- Knows position 0 is key, position 1 is value

This eliminates:
- Arbitrary string labels like "key", "value", "items"
- Unsound use of `SchemaField::to_string()`
- Need for HashMap in assembly (just Vec)
- Confusion about what identifies a child

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

4. **BUILD**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt
   ```

5. **VALIDATE**: Wait for user to confirm the build succeeded

6. **TEST** (if applicable): Run validation tests specific to that step

7. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

8. **PROCEED**: Move to next step only after confirmation
</Instructions>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### STEP 1: Add BuilderExample Struct
**Status:** ⏳ PENDING

**Objective:** Create lightweight struct for returning examples without paths

**Changes to make:**
1. Add `BuilderExample` struct with value, status, and optional reason
2. Add convenience constructors for common cases
3. Place in types.rs with other core types

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Code to add:**
```rust
/// Lightweight result from builders containing just example and status
#[derive(Debug, Clone)]
pub struct BuilderExample {
    /// The example JSON value
    pub value: Value,
    /// Mutation status of this example
    pub mutation_status: MutationStatus,
    /// Optional reason if not mutatable
    pub mutation_status_reason: Option<String>,
}

impl BuilderExample {
    /// Create a mutatable example
    pub fn mutatable(value: Value) -> Self {
        Self {
            value,
            mutation_status: MutationStatus::Mutatable,
            mutation_status_reason: None,
        }
    }
    
    /// Create a not-mutatable example with reason
    pub fn not_mutatable(reason: String) -> Self {
        Self {
            value: json!(null),
            mutation_status: MutationStatus::NotMutatable,
            mutation_status_reason: Some(reason),
        }
    }
    
    /// Create a partially mutatable example
    pub fn partially_mutatable(value: Value, reason: String) -> Self {
        Self {
            value,
            mutation_status: MutationStatus::PartiallyMutatable,
            mutation_status_reason: Some(reason),
        }
    }
}
```

**Expected outcome:**
- New struct available for use
- No functional changes yet
- Code compiles successfully

---

### STEP 2: Add PathAction enum and update RecursionContext
**Status:** ⏳ PENDING

**Objective:** Add enum to control path creation during recursion

**Changes to make:**
1. Define `PathAction` enum in types.rs
2. Add `path_action: PathAction` field to RecursionContext
3. Initialize to `PathAction::Create` in constructor (default behavior)
4. ProtocolEnforcer will set this based on `include_child_paths()`

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

**Code changes in types.rs:**
```rust
/// Action to take regarding path creation during recursion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAction {
    /// Create mutation paths during recursion
    Create,
    /// Skip path creation during recursion
    Skip,
}
```

**Code changes in recursion_context.rs:**
```rust
use super::types::PathAction;

pub struct RecursionContext {
    /// The building context (root or field)
    pub path_kind: PathKind,
    /// Reference to the type registry
    pub registry: Arc<HashMap<BrpTypeName, Value>>,
    /// The accumulated mutation path as we recurse through the type
    pub mutation_path: String,
    /// Parent's mutation knowledge for extracting component examples
    pub parent_knowledge: Option<&'static MutationKnowledge>,
    /// Action to take regarding path creation (set by ProtocolEnforcer)
    /// Design Review: Using enum instead of boolean for clarity and type safety
    pub path_action: PathAction,
}

impl RecursionContext {
    /// Create a new mutation path context
    pub const fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            path_kind,
            registry,
            mutation_path: String::new(),
            parent_knowledge: None,
            path_action: PathAction::Create,  // Default to creating paths
        }
    }
    
    // In create_field_context, preserve the action:
    pub fn create_field_context(&self, path_kind: PathKind) -> Self {
        // ... existing code ...
        Self {
            path_kind,
            registry: Arc::clone(&self.registry),
            mutation_path: new_path_prefix,
            parent_knowledge: field_knowledge,
            path_action: self.path_action,  // Preserve parent's setting
        }
    }
}
```

**Expected outcome:**
- Type-safe path action control
- Clear semantics with enum values
- ProtocolEnforcer can set this based on include_child_paths()
- Still backward compatible

---

### STEP 3: Update collect_children() signature
**Status:** ⏳ PENDING

**Objective:** Change to return PathKinds instead of creating RecursionContexts

**Changes to make:**
1. Change return type from `Vec<(String, RecursionContext)>` to `Vec<PathKind>`
2. Builders return just PathKinds in a defined order
3. ProtocolEnforcer will create the contexts

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

**Code changes in trait:**
```rust
    /// Collect PathKinds for child elements in a defined order
    /// 
    /// Migrated builders should return PathKinds without creating contexts.
    /// The order of PathKinds defines the positional mapping for assembly.
    fn collect_children(&self, ctx: &RecursionContext) -> Vec<PathKind> {
        // Default implementation for backward compatibility
        // Unmigrated builders still return contexts, extract PathKinds
        vec![]
    }
```

**Expected outcome:**
- New signature ready
- Still backward compatible
- Sets foundation for positional ordering

---

### STEP 4: Update assemble_from_children() signature
**Status:** ⏳ PENDING

**Objective:** Change to accept Vec<Value> instead of HashMap<String, Value>

**Changes to make:**
1. Change parameter from `HashMap<String, Value>` to `Vec<Value>`
2. Values are provided in same order as PathKinds from collect_children()
3. Each builder knows its own positional convention

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

**Code changes in trait:**
```rust
    /// Assemble a parent value from child examples
    /// 
    /// Receives Values in the SAME ORDER as PathKinds were returned from collect_children().
    /// Each builder defines its own positional convention.
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: Vec<Value>,
    ) -> Result<Value> {
        // Default implementation for backward compatibility
        // This will be removed once all builders are migrated
        Err(Error::InvalidState(
            "assemble_from_children not implemented".to_string()
        ).into())
    }
```

**Expected outcome:**
- New signature ready for positional ordering
- Clean, deterministic child identification
- No more HashMap overhead

---

### STEP 5: Rewrite ProtocolEnforcer to Create Contexts
**Status:** ⏳ PENDING

**Objective:** ProtocolEnforcer creates all contexts and controls path creation

**Changes to make:**
1. Update to create RecursionContexts itself
2. Set `path_action` based on `include_child_paths()`
3. Check `path_action` when creating paths on ascent
4. Use positional ordering for child values

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Major code rewrite in build_paths method:**
```rust
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        tracing::debug!("ProtocolEnforcer processing type: {}", ctx.type_name());

        // Check depth limit for THIS level
        if let Some(result) = Self::check_depth_limit(ctx, depth) {
            return result;
        }

        // Check if type is in registry
        if let Some(result) = Self::check_registry(ctx) {
            return result;
        }

        // Check knowledge for THIS level
        if let Some(result) = Self::check_knowledge(ctx) {
            return result;
        }

        // Get child PathKinds (not contexts!)
        let child_path_kinds = self.inner.collect_children(ctx);
        let mut all_paths = vec![];
        let mut child_values = Vec::new();

        // Recurse to each child
        for path_kind in child_path_kinds {
            // ProtocolEnforcer creates the context
            let mut child_ctx = ctx.create_field_context(path_kind);
            
            // Set the path action based on parent's include_child_paths()
            child_ctx.path_action = if self.inner.include_child_paths() {
                PathAction::Create
            } else {
                PathAction::Skip
            };

            tracing::debug!(
                "ProtocolEnforcer recursing to child of type '{}' (path_action: {:?})",
                child_ctx.type_name(),
                child_ctx.path_action
            );

            // Get child's schema and create its builder
            let child_schema = child_ctx.require_registry_schema().unwrap_or(&json!(null));
            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            let child_builder = child_kind.builder();

            // Recurse (child handles its OWN protocol)
            let child_paths = child_builder.build_paths(&child_ctx, depth.increment())?;
            
            // Extract child's example from its root path
            let child_example = child_paths
                .first()
                .map(|p| p.example.clone())
                .unwrap_or(json!(null));

            child_values.push(child_example);

            // Only include child paths if the builder wants them
            if self.inner.include_child_paths() {
                all_paths.extend(child_paths);
            }
        }

        // Assemble THIS level from children (positional ordering!)
        let parent_example = match self.inner.assemble_from_children(ctx, child_values) {
            Ok(example) => example,
            Err(e) => {
                return Self::handle_assemble_error(ctx, e);
            }
        };

        // Compute parent's mutation status from children's statuses
        let parent_status = Self::determine_parent_mutation_status(&all_paths);

        // Set appropriate error reason based on computed status
        let error_reason = match parent_status {
            MutationStatus::NotMutatable => Some("all_children_not_mutatable".to_string()),
            MutationStatus::PartiallyMutatable => Some("mixed_mutability_children".to_string()),
            MutationStatus::Mutatable => None,
        };

        // Add THIS level's path at the beginning (only if path_action is Create)
        if matches!(ctx.path_action, PathAction::Create) {
            all_paths.insert(
                0,
                Self::build_mutation_path(ctx, parent_example, parent_status, error_reason),
            );
        }

        Ok(all_paths)
    }
```

**Expected outcome:**
- Smart recursion ready
- No wasted path creation for Map/Set children
- Backward compatible with unmigrated builders

---

### STEP 6: Update MapMutationBuilder with build_example
**Status:** ⏳ PENDING

**Objective:** Implement efficient build_example for Map

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`

**Code to add (in impl block):**
```rust
    fn build_example(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<BuilderExample> {
        // Use the same logic as assemble_from_children but recurse for examples
        let children = self.collect_children(ctx);
        let mut child_examples = HashMap::new();

        for (name, child_ctx) in children {
            let child_schema = child_ctx.require_registry_schema().unwrap_or(&json!(null));
            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            let child_builder = child_kind.builder();

            // Get just the example - no paths needed
            let child_example = child_builder.build_example(&child_ctx, depth.increment())?;
            child_examples.insert(name, child_example.value);
        }

        // Assemble the map
        let map_value = self.assemble_from_children(ctx, child_examples)?;
        Ok(BuilderExample::mutatable(map_value))
    }
```

**Expected outcome:**
- Map no longer creates wasted Transform paths
- Significant performance improvement for nested maps

---

### STEP 7: Update SetMutationBuilder with build_example
**Status:** ⏳ PENDING

**Objective:** Implement efficient build_example for Set

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`

**Code to add (in impl block):**
```rust
    fn build_example(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<BuilderExample> {
        // Similar to Map - get children and assemble
        let children = self.collect_children(ctx);
        let mut child_examples = HashMap::new();

        for (name, child_ctx) in children {
            let child_schema = child_ctx.require_registry_schema().unwrap_or(&json!(null));
            let child_type = child_ctx.type_name();
            let child_kind = TypeKind::from_schema(child_schema, child_type);
            let child_builder = child_kind.builder();

            // Get just the example
            let child_example = child_builder.build_example(&child_ctx, depth.increment())?;
            child_examples.insert(name, child_example.value);
        }

        // Assemble the set
        let set_value = self.assemble_from_children(ctx, child_examples)?;
        Ok(BuilderExample::mutatable(set_value))
    }
```

**Expected outcome:**
- Set also optimized
- All migrated builders now efficient

---

### STEP 8: Test and Validate
**Status:** ⏳ PENDING

**Objective:** Verify the optimization works

**Test commands:**
```bash
# Run the test app to ensure no functionality broken
cd test-app
cargo build --example complex_types
cargo run --example complex_types

# Check that HashMap<String, Transform> works correctly
# (Will need to verify through MCP once installed)
```

**Validation points:**
1. Build succeeds
2. Test app runs without crashes
3. Map and Set examples generate correctly
4. No wasted path creation (verify with logging)

---

### STEP 9: Update plan-recursion.md for Remaining Builders
**Status:** ⏳ PENDING

**Objective:** Update migration instructions for unmigrated builders

**File to modify:**
- `plan-recursion.md`

**Key changes to document:**
1. Builders should implement `build_example()` returning `Result<BuilderExample>`
2. The method should use `collect_children()` and recursively call `build_example()`
3. No path creation happens in builders anymore
4. Status computation happens in ProtocolEnforcer

**Expected outcome:**
- Clear migration path for remaining builders
- Consistent pattern documented

---

## Benefits Achieved

1. **Performance**: No wasted path creation for Map/Set children
2. **Memory**: Fewer allocations during recursion
3. **Clarity**: Clear separation between example building and path creation
4. **Simplicity**: Builders focus only on example assembly

## Design Review Skip Notes

### TYPE-SYSTEM-1: Positional Vec indexing lacks compile-time safety guarantees - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: STEP 4: Update assemble_from_children() signature
- **Issue**: Concern that Vec<Value> with positional ordering lacks compile-time safety compared to HashMap<String, Value>
- **Reasoning**: After investigation, the positional Vec approach is actually the CORRECT design for this specific use case
- **Decision**: The positional ordering approach is sound because:
  1. **ProtocolEnforcer maintains order invariant**: It collects PathKinds from `collect_children()` and passes Values back to `assemble_from_children()` in the SAME order
  2. **Each builder defines its own convention**: Map knows position 0=key, 1=value; Set knows position 0=items
  3. **String keys were already arbitrary**: The current HashMap uses `SchemaField::to_string()` which produces arbitrary labels like "key", "value", "items"
  4. **Critical insight**: After investigating all 7 builder types (Struct, Enum, Tuple, Array, List, Map, Set), we found that only Map and Set are currently migrated, and both have FIXED positional semantics that work perfectly with Vec
  5. **Struct/Enum will use different approach**: When migrated, StructBuilder will return named PathKinds with field names embedded in `PathKind::StructField(field_name, ...)`, allowing reconstruction
  6. **The plan correctly simplifies**: Removes HashMap overhead and arbitrary string labels while maintaining safety through ProtocolEnforcer's order preservation

## Future Cleanup (After All Builders Migrated)

1. Remove default implementations from trait methods
2. Remove special handling for unmigrated builders
3. Consider removing `build_paths()` from individual builders entirely
4. Simplify ProtocolEnforcer once all builders use positional ordering