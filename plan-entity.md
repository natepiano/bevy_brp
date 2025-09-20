# Plan: Add Entity ID Warning to Type Guides

## Problem

When type guides generate examples for types containing Entity IDs (like `EntityHashMap`), they use placeholder entity IDs from `mutation_knowledge.rs` (e.g., `8589934670`). These placeholder IDs don't exist in running apps, causing crashes when users copy examples directly.

Example case: `bevy_pbr::light::Cascades` contains an `EntityHashMap` field where the keys are Entity IDs. The type guide shows:
```json
{
  "cascades": {
    "8589934670": [...]  // This entity doesn't exist!
  }
}
```

## Solution

Add a simple boolean flag that tracks whether Entity types were encountered during mutation path building, then display a single warning at the type guide level.

## Implementation

### 1. Add `is_entity()` method to BrpTypeName

```rust
// In BrpTypeName
impl BrpTypeName {
    pub fn is_entity(&self) -> bool {
        self.as_str() == "bevy_ecs::entity::Entity"
    }
}
```

### 2. Modify mutation path builder to track Entity usage

Change the signature of `recurse_mutation_paths` to return a tuple:

```rust
// In mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs
pub fn recurse_mutation_paths(
    type_kind: TypeKind,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(Vec<MutationPathInternal>, bool)> {  // Added bool for contains_entity

    // Check if current type is Entity
    let is_entity = ctx.type_name().is_entity();

    // ... existing dispatch logic to get paths ...

    Ok((paths, is_entity))
}
```

### 3. Propagate Entity flag through recursion

Update the child processing to track if any child contains entities:

```rust
// In process_child method
fn process_child(
    descriptor: &MutationPathDescriptor,
    child_ctx: &mut RecursionContext,
    depth: RecursionDepth,
) -> Result<(Vec<MutationPathInternal>, Value, bool)> {  // Add bool return

    // ... existing code ...

    let (child_paths, child_has_entity) = recurse_mutation_paths(
        child_kind,
        child_ctx,
        depth.increment()
    )?;

    // ... extract example ...

    Ok((child_paths, child_example, child_has_entity))
}
```

Update `process_all_children` to aggregate the flag:

```rust
// In process_all_children
fn process_all_children(
    &self,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(ChildProcessingResult, bool)> {  // Add bool return

    let mut contains_entity = false;

    // ... existing child iteration ...
    for item in child_items {
        // ... existing code ...

        let (child_paths, child_example, child_has_entity) =
            Self::process_child(&child_key, &mut child_ctx, depth)?;

        contains_entity = contains_entity || child_has_entity;

        // ... rest of existing code ...
    }

    Ok((ChildProcessingResult { ... }, contains_entity))
}
```

### 4. Thread through build_paths

Update the main `build_paths` method to handle the flag:

```rust
// In build_paths
fn build_paths(
    &self,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<(Vec<MutationPathInternal>, bool)> {  // Add bool return

    // ... existing early return checks ...

    let (ChildProcessingResult { ... }, children_contain_entity) =
        self.process_all_children(ctx, depth)?;

    // Check if this type itself is Entity
    let is_entity = ctx.type_name().is_entity();
    let contains_entity = is_entity || children_contain_entity;

    // ... rest of existing logic ...

    Ok((paths, contains_entity))
}
```

### 5. Add warning field to type guide output

In the type guide generation (likely in the tool implementation):

```rust
// Where TypeGuide is assembled (in the BRP tool handler)
pub struct TypeGuide {
    pub type_name: String,
    pub mutation_paths: HashMap<String, MutationPath>,
    pub entity_warning: Option<String>,  // NEW FIELD
    // ... other fields
}

// When creating the type guide
let (paths, contains_entity) = generate_mutation_paths(...)?;

let entity_warning = if contains_entity {
    Some(
        "⚠️ Entity ID Warning: This type's examples contain Entity IDs (e.g., 8589934670). \
         These are placeholders that won't exist in your app. You MUST replace them with \
         real entity IDs obtained from spawn operations or queries."
    )
} else {
    None
};
```

## Benefits

1. **Simple Implementation**: Just thread a boolean through existing recursion
2. **Clear User Warning**: Single, prominent warning at the type guide level
3. **No Complex Propagation**: No need to track warnings per-path
4. **Catches All Cases**: Works for EntityHashMap, direct Entity fields, nested entities, etc.

## Test Cases

1. `bevy_pbr::light::Cascades` - Contains EntityHashMap
2. Direct Entity field in a component
3. Vec<Entity> or other collections of entities
4. Nested structures containing entities

## Alternative Considered

We considered adding per-path warnings but decided against it because:
- More complex to implement
- Redundant (if one path has entities, usually many do)
- Single warning at type guide level is clearer for users