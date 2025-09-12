# Plan: Eliminate Path Creation Waste Through Protocol-Driven Recursion

**Migration Strategy: Phased** - This implementation requires sequential steps due to complex interdependencies between RecursionContext changes, trait signature updates, and ProtocolEnforcer rewrite. Each step validates the previous before proceeding.

## Core Design Principle
The goal is to have ProtocolEnforcer handle ALL recursion and path creation, while migrated builders ONLY assemble examples from children. ProtocolEnforcer is the sole authority for determining mutation status and reasons. This eliminates wasted path creation and simplifies builders.

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
3. **Builders only assemble examples** - No path creation or status determination in builders
4. **Path creation happens ONLY in ProtocolEnforcer** - Based on context's `path_action` field
5. **Mutation status determined ONLY by ProtocolEnforcer** - Based on registry, traits, and child statuses

## Key Design Innovation: PathKind-Driven Child Identification

The critical insight is that PathKinds carry all the information needed to identify children:

1. **Builders define children** - Each builder returns PathKinds with embedded identifiers
2. **ProtocolEnforcer extracts keys** - Uses PathKind to build HashMap keys
3. **Assembly uses semantic keys** - Builder receives HashMap with meaningful keys

Example for MapMutationBuilder:
- Returns `vec![PathKind::StructField { field_name: "key", ... }, PathKind::StructField { field_name: "value", ... }]`
- ProtocolEnforcer extracts "key" and "value" from PathKinds
- Builder receives `HashMap { "key" => key_value, "value" => value_value }`

This provides:
- Semantic field identification ("translation", "rotation", "scale" for structs)
- Type-safe child identification through PathKind enum
- Consistent interface across all builder types
- Self-documenting code with meaningful keys

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

### STEP 1: Add PathAction enum and update RecursionContext
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

### STEP 2: Update collect_children() signature and migrated builders
**Status:** ⏳ PENDING

**Objective:** Change trait signature and update MapMutationBuilder/SetMutationBuilder to prevent build breakage

**Changes to make:**
1. Change trait signature from `Vec<(String, RecursionContext)>` to `Result<Vec<PathKind>>`
2. Update MapMutationBuilder's collect_children() to return PathKinds
3. Update SetMutationBuilder's collect_children() to return PathKinds
4. Update ProtocolEnforcer to handle the new Result type

**Files to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`

**Code changes in trait:**
```rust
    /// Collect PathKinds for child elements
    ///
    /// Migrated builders should return PathKinds without creating contexts.
    /// PathKinds contain the necessary information (field names, indices) for child identification.
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        // Default implementation for backward compatibility
        Ok(vec![])
    }
```

**Code changes in MapMutationBuilder:**
```rust
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Err(Error::InvalidState(format!(
                "No schema found for map type: {}",
                ctx.type_name()
            )).into());
        };

        // Extract key and value types from schema
        let key_type = schema.get_type(SchemaField::KeyType);
        let value_type = schema.get_type(SchemaField::ValueType);

        let Some(key_t) = key_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract key type from schema for type: {}",
                ctx.type_name()
            )).into());
        };

        let Some(val_t) = value_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract value type from schema for type: {}",
                ctx.type_name()
            )).into());
        };

        // Create PathKinds for key and value (ProtocolEnforcer will create contexts)
        Ok(vec![
            PathKind::StructField {
                field_name: SchemaField::Key.to_string(),
                field_type: key_t.clone(),
                optional: false,
            },
            PathKind::StructField {
                field_name: SchemaField::Value.to_string(),
                field_type: val_t.clone(),
                optional: false,
            },
        ])
    }
```

**Code changes in SetMutationBuilder:**
```rust
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let Some(schema) = ctx.require_registry_schema() else {
            return Err(Error::InvalidState(format!(
                "No schema found for set type: {}",
                ctx.type_name()
            )).into());
        };

        // Extract item type from schema
        let item_type = schema.get_type(SchemaField::Items);

        let Some(item_t) = item_type else {
            return Err(Error::InvalidState(format!(
                "Failed to extract item type from schema for type: {}",
                ctx.type_name()
            )).into());
        };

        // Create PathKind for items (ProtocolEnforcer will create context)
        Ok(vec![PathKind::StructField {
            field_name: SchemaField::Items.to_string(),
            field_type: item_t.clone(),
            optional: false,
        }])
    }
```

**Expected outcome:**
- New signature with proper error handling
- MapMutationBuilder and SetMutationBuilder updated to prevent build breakage
- PathKinds carry child identification info
- Errors properly propagated instead of silently logged

---

### STEP 3: Keep assemble_from_children() signature with HashMap
**Status:** ⏳ PENDING

**Objective:** Keep HashMap<String, Value> for semantic child identification

**Rationale:**
1. HashMap keys provide semantic identification of children
2. Works uniformly across all builder types (structs, maps, arrays)
3. ProtocolEnforcer extracts keys from PathKinds
4. Builders only assemble examples - no status determination

**No changes needed to existing signature:**
```rust
    /// Assemble a parent value from child examples
    ///
    /// Receives HashMap where keys are extracted from PathKinds:
    /// - StructField: uses field_name
    /// - IndexedElement/ArrayElement: uses index.to_string()
    /// Builders ONLY assemble examples - mutation status is determined by ProtocolEnforcer.
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<String, Value>,
    ) -> Result<Value> {
        // Existing implementation stays
        Err(Error::InvalidState(
            "assemble_from_children not implemented".to_string()
        ).into())
    }
```

**Expected outcome:**
- Semantic child identification preserved
- Works for all builder types
- Self-documenting with meaningful keys
- Builders remain simple - only assembly logic

---

### STEP 4: Rewrite ProtocolEnforcer to Create Contexts and Determine Mutation Status
**Status:** ⏳ PENDING

**Objective:** ProtocolEnforcer creates all contexts, controls path creation, and determines mutation status

**Changes to make:**
1. Update to create RecursionContexts itself
2. Set `path_action` based on `include_child_paths()`
3. Check `path_action` when creating paths on ascent
4. Extract keys from PathKinds to build HashMap for child values
5. **ProtocolEnforcer is sole authority for mutation status and reasons**

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
        let child_path_kinds = self.inner.collect_children(ctx)?;
        let mut all_paths = vec![];
        let mut child_examples = HashMap::new();

        // Recurse to each child
        for path_kind in child_path_kinds {
            // ProtocolEnforcer creates the context
            let mut child_ctx = ctx.create_field_context(path_kind.clone());

            // Set the path action based on parent's include_child_paths()
            child_ctx.path_action = if self.inner.include_child_paths() {
                PathAction::Create
            } else {
                PathAction::Skip
            };

            // Extract key from PathKind for HashMap
            let child_key = match &path_kind {
                PathKind::StructField { field_name, .. } => field_name.clone(),
                PathKind::IndexedElement { index, .. } => index.to_string(),
                PathKind::ArrayElement { index, .. } => index.to_string(),
                PathKind::RootValue { .. } => String::new(),
            };

            tracing::debug!(
                "ProtocolEnforcer recursing to child '{}' of type '{}' (path_action: {:?})",
                child_key,
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

            child_examples.insert(child_key, child_example);

            // Only include child paths if the builder wants them
            if self.inner.include_child_paths() {
                all_paths.extend(child_paths);
            }
        }

        // Assemble THIS level from children (HashMap with semantic keys)
        let parent_example = match self.inner.assemble_from_children(ctx, child_examples) {
            Ok(example) => example,
            Err(e) => {
                return Self::handle_assemble_error(ctx, e);
            }
        };

        // Compute parent's mutation status from children's statuses
        let parent_status = Self::determine_parent_mutation_status(&all_paths);

        // Generate appropriate mutation_status_reason using NotMutatableReason enum
        let parent_reason = match parent_status {
            MutationStatus::NotMutatable => {
                Some(NotMutatableReason::AllChildrenNotMutatable {
                    parent_type: ctx.type_name().clone()
                })
            },
            MutationStatus::PartiallyMutatable => {
                Some(NotMutatableReason::MixedChildMutability {
                    parent_type: ctx.type_name().clone()
                })
            },
            MutationStatus::Mutatable => None,
        }.map(|reason| Option::<String>::from(&reason)).flatten();

        // Add THIS level's path at the beginning (only if path_action is Create)
        if matches!(ctx.path_action, PathAction::Create) {
            all_paths.insert(
                0,
                Self::build_mutation_path_internal(ctx, parent_example, parent_status, parent_reason),
            );
        }

        Ok(all_paths)
    }
```

**Mutation Status Reason Generation:**

ProtocolEnforcer is the SOLE authority for generating mutation_status_reason by extending the `NotMutatableReason` enum.

**Current code to fix** (protocol_enforcer.rs lines 245-248):
```rust
// CURRENT - using ad-hoc strings
let mutation_status_reason = match parent_status {
    MutationStatus::NotMutatable => Some("all_children_not_mutatable".to_string()),
    MutationStatus::PartiallyMutatable => Some("mixed_mutability_children".to_string()),
    MutationStatus::Mutatable => None,
};
```

First, extend the enum in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotMutatableReason {
    // ... existing variants ...
    AllChildrenNotMutatable { parent_type: BrpTypeName },
    MixedChildMutability { parent_type: BrpTypeName },
}
```

Then use these enum variants instead of ad-hoc strings:
- **NotMutatableReason::RecursionLimitExceeded** - When depth limit is hit
- **NotMutatableReason::NotInRegistry** - When type lookup fails
- **NotMutatableReason::MissingSerializationTraits** - When type lacks required traits
- **NotMutatableReason::AllChildrenNotMutatable** - When all child paths are NotMutatable
- **NotMutatableReason::MixedChildMutability** - When some children are mutatable, others not

**Expected outcome:**
- Smart recursion ready
- No wasted path creation for Map/Set children
- Centralized mutation status logic in ProtocolEnforcer
- Meaningful, specific mutation reasons
- Backward compatible with unmigrated builders

---

### STEP 4.5: (Merged into STEP 2)
**Status:** ✅ COMPLETED IN STEP 2

**Note:** The MapMutationBuilder collect_children() changes have been moved to STEP 2 to prevent build breakage.

---

### STEP 4.6: (Merged into STEP 2)
**Status:** ✅ COMPLETED IN STEP 2

**Note:** The SetMutationBuilder collect_children() changes have been moved to STEP 2 to prevent build breakage.

---

### STEP 5: Update MapMutationBuilder's assemble_from_children
**Status:** ⏳ PENDING

**Objective:** Implement assemble_from_children with HashMap<String, Value>

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`

**Code to update (in impl block):**
```rust
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<String, Value>,
    ) -> Result<Value> {
        // Map expects SchemaField::Key and SchemaField::Value in the HashMap
        let key_value = children.get(&SchemaField::Key.to_string())
            .ok_or_else(|| Error::InvalidState(format!(
                "Map type {} missing required '{}' child example",
                ctx.type_name(),
                SchemaField::Key.to_string()
            )).into())?;

        let value_value = children.get(&SchemaField::Value.to_string())
            .ok_or_else(|| Error::InvalidState(format!(
                "Map type {} missing required '{}' child example", 
                ctx.type_name(),
                SchemaField::Value.to_string()
            )).into())?;

        // Create the map with the example key-value pair
        let mut map = serde_json::Map::new();

        // Convert key to string for JSON map
        let key_str = match key_value {
            Value::String(s) => s.clone(),
            _ => {
                return Err(Error::InvalidState(format!(
                    "Map type {} has non-string key type, cannot serialize to JSON map: {key_value:?}",
                    ctx.type_name()
                )).into());
            }
        };

        map.insert(key_str, value_value.clone());

        // Return just the assembled value - no status determination
        Ok(Value::Object(map))
    }
```

**Expected outcome:**
- Map uses semantic keys ("key", "value")
- No wasted Transform path creation
- Simple assembly logic only

---

### STEP 6: Update SetMutationBuilder's assemble_from_children
**Status:** ⏳ PENDING

**Objective:** Implement assemble_from_children with HashMap<String, Value>

**File to modify:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`

**Code to update (in impl block):**
```rust
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<String, Value>,
    ) -> Result<Value> {
        // Set expects SchemaField::Items in the HashMap
        let items_value = children.get(&SchemaField::Items.to_string())
            .ok_or_else(|| Error::InvalidState(format!(
                "Set type {} missing required '{}' child example",
                ctx.type_name(),
                SchemaField::Items.to_string()
            )).into())?;

        // Create array with single example item
        let set_array = vec![items_value.clone()];

        // Return just the assembled value - no status determination
        Ok(Value::Array(set_array))
    }
```

**Expected outcome:**
- Set uses semantic key ("items")
- Simple assembly logic only
- All migrated builders now efficient

---

### STEP 7: Test and Validate
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

### STEP 8: Update plan-recursion.md for Remaining Builders
**Status:** ⏳ PENDING

**Objective:** Update migration instructions for unmigrated builders with precise duplication removal

**File to modify:**
- `plan-recursion.md`

**Phase 1: Precise Duplication Patterns to Remove**

**REMOVE from ALL unmigrated builders (List, Array, Tuple, Struct, Enum):**

1. **Depth checking (lines like these):**
```rust
if depth.exceeds_limit() {
    return Ok(vec![Self::build_not_mutatable_path(
        ctx,
        NotMutatableReason::RecursionLimitExceeded(ctx.type_name().clone()),
    )]);
}
```

2. **Registry validation (lines like these):**
```rust
let Some(schema) = ctx.require_registry_schema() else {
    return Ok(vec![Self::build_not_mutatable_path(
        ctx,
        NotMutatableReason::NotInRegistry(ctx.type_name().clone()),
    )]);
};
```

3. **build_not_mutatable_path method (entire method):**
```rust
fn build_not_mutatable_path(
    ctx: &RecursionContext,
    reason: NotMutatableReason,
) -> MutationPathInternal {
    // ENTIRE METHOD - REMOVE
}
```

4. **Mutation status propagation logic:**
```rust
// REMOVE any code that sets mutation_status or mutation_status_reason
mutation_status: if all_mutatable { MutationStatus::Mutatable } else { ... },
mutation_status_reason: Some("...".to_string()),
```

5. **Direct knowledge checks:**
```rust
if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(type_name)) {
    // REMOVE - ProtocolEnforcer handles this
}
```

**Phase 2: Update plan-recursion.md Instructions**

For EACH unmigrated builder section (lines 468-640), add these SPECIFIC removal instructions:

```markdown
**CODE TO REMOVE:**
- Lines containing `depth.exceeds_limit()`
- Lines containing `ctx.require_registry_schema() else`
- The entire `build_not_mutatable_path` method (usually ~12 lines)
- Any line setting `mutation_status:` field
- Any line setting `mutation_status_reason:` field
- Any imports of `NotMutatableReason`

**CODE TO KEEP:**
- Schema field extraction logic
- Child type identification
- Array/List element handling
- Struct field iteration
- Enum variant handling
```

**Phase 3: Document What ProtocolEnforcer Now Handles**

Add to plan-recursion.md:
```markdown
## Responsibilities Now in ProtocolEnforcer

After migration, ProtocolEnforcer handles ALL of:
1. Depth limit checking (lines 98-107)
2. Registry validation (implicit via schema checks)
3. Knowledge lookups (lines 110-119)
4. NotMutatable path creation (line 162)
5. Mutation status computation (lines 241-248 in plan-builder-example)
6. Child path filtering via include_child_paths()

Builders ONLY handle:
- Identifying children via collect_children()
- Assembling examples via assemble_from_children()
```

**Expected outcome:**
- plan-recursion.md updated with exact code patterns to remove
- Each unmigrated builder section has specific removal instructions
- ProtocolEnforcer responsibilities clearly documented
- Builders simplified to only collect_children() and assemble_from_children()
- All duplication eliminated: depth checks, registry validation, NotMutatable paths, mutation status logic

---

## Benefits Achieved

1. **Performance**: No wasted path creation for Map/Set children
2. **Memory**: Fewer allocations during recursion
3. **Clarity**: Clear separation of concerns - builders assemble, ProtocolEnforcer determines status
4. **Simplicity**: Builders focus only on example assembly, no status logic
5. **Consistency**: Single source of truth for mutation status and reasons
6. **Maintainability**: Mutation logic centralized in ProtocolEnforcer
7. **Semantic clarity**: HashMap keys provide self-documenting child identification

## Design Review Skip Notes

### SIMPLIFICATION-1: Plan could reuse existing collect_children pattern instead of changing signatures
- **Status**: SKIPPED
- **Location**: Section: STEP 2: Update collect_children() signature
- **Issue**: Current collect_children returns Vec<(String, RecursionContext)> which already provides the context creation. Plan proposes changing to Vec<PathKind> requiring ProtocolEnforcer to recreate contexts, adding complexity
- **Reasoning**: While the finding identifies a possible short-term simplification, it conflicts with the plan's fundamental architectural goals. The plan explicitly aims to have ProtocolEnforcer control ALL context creation rather than having builders create contexts that ProtocolEnforcer then modifies. The PathKind approach enables key benefits: (1) cleaner separation where builders describe 'what' (PathKind) and ProtocolEnforcer handles 'how' (context creation), (2) PathKinds carry semantic information for child identification, and (3) consistent object ownership patterns. The suggested alternative creates an antipattern where ProtocolEnforcer modifies builder-created contexts, making ownership unclear.
- **Decision**: User elected to skip this recommendation

### DESIGN-3: Plan references wrong field name in ProtocolEnforcer rewrite ✅
- **Status**: APPROVED - To be implemented
- **Location**: Section: STEP 5: Rewrite ProtocolEnforcer to Create Contexts
- **Issue**: STEP 5 code references child_ctx.should_create_path and ctx.should_create_path but STEP 2 defines path_action: PathAction field, not should_create_path boolean
- **Reasoning**: The plan document contains inconsistent field references. Early sections properly define path_action: PathAction enum, but later code examples use should_create_path boolean field that doesn't exist in the defined types. This creates confusion and would cause compilation errors if someone tried to implement the plan as written.

### Approved Change:
The plan has been updated to consistently use `path_action: PathAction` throughout, including proper enum comparison using `matches!` macro.

### Implementation Notes:
All references to `should_create_path` have been replaced with `path_action`, and boolean comparisons have been updated to use `matches!(ctx.path_action, PathAction::Create)` for proper Rust enum handling.


### DESIGN-1: BuilderExample struct duplicates existing MutationPathInternal fields
- **Status**: SKIPPED
- **Location**: Section: STEP 1: Add BuilderExample Struct
- **Issue**: BuilderExample struct has identical fields to MutationPathInternal (value, mutation_status, mutation_status_reason) creating structural duplication
- **Reasoning**: This is a false positive. While the structs share 2 fields (mutation_status and mutation_status_reason), they serve completely different purposes and the duplication is minimal and semantically appropriate. MutationPathInternal is a comprehensive internal structure with 6 fields including path navigation and type information. BuilderExample is designed as a lightweight result with only 3 fields for simple value and status reporting. The shared fields represent the same logical concept (mutation capability) which both structs legitimately need. Creating shared abstractions for just 2 fields would add more complexity than it solves, and this pattern of having lightweight vs full-featured variants is common and acceptable in Rust.
- **Decision**: User elected to skip this recommendation


## Future Cleanup (After All Builders Migrated)

1. Remove default implementations from trait methods
2. Remove special handling for unmigrated builders
3. Consider removing `build_paths()` from individual builders entirely
4. Simplify ProtocolEnforcer once all builders are migrated
