# Query Operations Tests

## Objective
Validate `world.query` BRP method functionality including the new Bevy 0.17.2 `ComponentSelector` enum with `"all"` option support.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing query operations, not app management.

## Test Steps

### 1. Setup: Create Test Entities
Spawn 3 entities with different component combinations for query testing:

**Entity A** - Transform only:
- Execute `mcp__brp__world_spawn_entity` with Transform: `{"translation": [1, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}`
- Store entity ID as `entity_a`

**Entity B** - Transform + Name:
- Execute `mcp__brp__world_spawn_entity` with Transform: `{"translation": [2, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}`
- Store entity ID as `entity_b`
- Execute `mcp__brp__world_insert_components` to add Name: `{"bevy_ecs::name::Name": "EntityB"}`

**Entity C** - Transform + Visibility:
- Execute `mcp__brp__world_spawn_entity` with Transform: `{"translation": [3, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}`
- Store entity ID as `entity_c`
- Execute `mcp__brp__world_insert_components` to add Visibility: `{"bevy_render::view::visibility::Visibility": "Inherited"}`

### 2. Query with Array Syntax (Backward Compatibility)
Test the traditional array format for `option` field:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"],
      "option": ["bevy_ecs::name::Name"]
    }
  }
  ```
- Verify: Returns all 3 entities (A, B, C)
- Verify: Entity B includes Name data, entities A and C show null/absent Name
- Verify: All entities include Transform data

### 3. Query with "all" Syntax (New in 0.17.2)
Test the new `ComponentSelector::All` variant:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "option": "all"
    }
  }
  ```
- Verify: Returns many entities (including our 3 test entities plus existing app entities)
- Verify: Each entity includes ALL its components in the response
- Verify: Response includes component data for all registered component types on each entity

### 4. Query with Empty Option (Default)
Test default behavior when `option` is omitted:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"]
    }
  }
  ```
- Verify: Returns all entities with Transform
- Verify: Only Transform component data is returned (no optional components)

### 5. Query with Empty Data Object (Entity IDs Only)
Test the special case documented in help text:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {}
  }
  ```
- Verify: Returns entity IDs only
- Verify: No component data included in response
- Verify: Entity count matches expected total entities in app

### 6. Query with Filter: with + without
Test filter combinations:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "option": "all"
    },
    "filter": {
      "with": ["bevy_transform::components::transform::Transform"],
      "without": ["bevy_render::camera::camera::Camera"]
    }
  }
  ```
- Verify: Returns entities with Transform but not Camera
- Verify: Our test entities A, B, C are included
- Verify: Camera entities are excluded

### 7. Query with Mixed Fields (components + option + has)
Test all query data fields together:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"],
      "option": ["bevy_ecs::name::Name", "bevy_render::view::visibility::Visibility"],
      "has": ["bevy_render::camera::camera::Camera"]
    }
  }
  ```
- Verify: Returns entities with required Transform component
- Verify: Optional Name and Visibility data included if present
- Verify: `has` field returns boolean for Camera component presence (not component data)

### 8. Query with Filter Omitted vs Empty Object
Test serialization difference:

**Test A - Filter omitted**:
- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"]
    }
  }
  ```

**Test B - Filter as empty object**:
- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"]
    },
    "filter": {}
  }
  ```

- Verify: Both produce identical results (omitted and empty object are equivalent)

### 9. Query with "all" Option and Filter
Test combining the new "all" syntax with filtering:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "option": "all"
    },
    "filter": {
      "with": ["bevy_ecs::name::Name"]
    }
  }
  ```
- Verify: Returns only entities with Name component
- Verify: Should return Entity B from our test entities
- Verify: All components on Entity B are included in response

### 10. Error Case: Invalid Option Value
Test error handling for invalid `option` values:

**Note**: This test validates deserialization errors, not BRP errors. If the MCP tool successfully deserializes invalid values and sends them to BRP, the test should fail.

- Execute `mcp__brp__world_query` with invalid option (number):
  ```json
  {
    "data": {
      "option": 123
    }
  }
  ```
- Verify: Returns clear error message about invalid ComponentSelector format
- Verify: Error indicates expected "all" or array of strings

### 11. Cleanup: Remove Test Entities
- Execute `mcp__brp__world_despawn_entity` for entity_a
- Execute `mcp__brp__world_despawn_entity` for entity_b
- Execute `mcp__brp__world_despawn_entity` for entity_c
- Verify: All test entities are properly despawned

## Expected Results
- ✅ Array syntax for `option` works (backward compatibility)
- ✅ "all" syntax for `option` returns all components
- ✅ Empty option defaults correctly
- ✅ Empty data object returns entity IDs only
- ✅ Filter with `with` and `without` works correctly
- ✅ Mixed usage of components, option, and has fields succeeds
- ✅ Filter omission and empty object are equivalent
- ✅ "all" option combined with filter works
- ✅ Invalid option values produce clear error messages
- ✅ Entity cleanup completes successfully

## Failure Criteria
STOP if: Query returns incorrect data, serialization fails, backward compatibility breaks, or the new "all" option doesn't work as specified in Bevy 0.17.2.
