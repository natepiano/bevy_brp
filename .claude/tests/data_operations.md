# Data Operations Tests

## Objective
Validate entity, component, and resource CRUD operations through BRP.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing operations, not app management.

## Test Steps

### 1. Entity Spawning
- Execute `mcp__brp__world_spawn_entity` with Transform component
- Verify new entity ID is returned
- Use simple Transform format: `{"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}`

### 2. Component Insertion
- Execute `mcp__brp__bevy_insert` to add Name component to spawned entity
- Use format: `{"bevy_ecs::name::Name": "TestEntity"}`
- Verify operation succeeds

### 3. Component Retrieval
- Execute `mcp__brp__bevy_get` to retrieve components from entity
- Request both Transform and Name components
- Verify data matches what was inserted

### 4. Component Mutation
- Execute `mcp__brp__bevy_mutate_component` to modify Transform translation
- Use path `.translation.x` with new value
- Verify mutation succeeds

### 5. Component Removal
- Execute `mcp__brp__bevy_remove` to remove Name component
- Verify component is removed from entity
- Confirm Transform component remains

### 6. Resource Operations with Type Guide Discovery
- Execute `mcp__brp__brp_type_guide` with `["bevy_render::camera::clear_color::ClearColor"]` to discover resource structure
- Verify schema returns mutation paths and spawn format information
- Execute `mcp__brp__world_get_resources` to retrieve current ClearColor resource value
- Execute `mcp__brp__bevy_mutate_resource` using discovered structure:
  - Path: `.0` (the Color field, as revealed by type schema)
  - Value: `{"Srgba": {"red": 0.8, "green": 0.2, "blue": 0.1, "alpha": 1.0}}`
- Execute `mcp__brp__world_get_resources` again to verify the mutation took effect
- Confirm the color value changed to the new Srgba values

### 7. Entity Cleanup
- Execute `mcp__brp__bevy_destroy` to remove test entity
- Verify entity is properly destroyed

## Expected Results
- ✅ Entity spawning returns valid entity ID
- ✅ Component insertion succeeds
- ✅ Component retrieval returns correct data
- ✅ Component mutation works as expected
- ✅ Component removal functions properly
- ✅ Type schema reveals correct resource structure and mutation paths
- ✅ Resource access and mutation using discovered paths is functional
- ✅ Entity destruction completes cleanly

## Failure Criteria
STOP if: Any CRUD operation fails unexpectedly, data corruption occurs, or operations return malformed responses.
