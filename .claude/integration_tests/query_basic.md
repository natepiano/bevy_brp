# Query Basic Operations Tests

## Objective
Validate `world.query` BRP method functionality for inline-result operations including array syntax, empty option, small entity sets, mixed fields, filter equivalence, and error handling.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing query operations, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

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
- Verify by reading the JSON response directly (it's small enough to fit in context)
- Check: Returns multiple entities with Transform
- Check: Entities with Name component include Name data
- Check: Entities without Name have null/absent Name field
- Check: All entities include Transform data
- **Do NOT use jq or bash commands** - the response is returned directly in the tool output

### 3. Query with Empty Option (Default)
Test default behavior when `option` is omitted:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"]
    }
  }
  ```
- Verify by reading the JSON response directly (it's small enough to fit in context)
- Check: Returns all entities with Transform
- Check: Only Transform component data is returned (no optional components)
- **Do NOT use jq or bash commands** - the response is returned directly in the tool output

### 4. Query with "all" Option on Small Entity Set
Test the "all" option with a filter that produces a small inline response:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "option": "all"
    },
    "filter": {
      "with": ["bevy_camera::camera::Camera"]
    }
  }
  ```
- Verify by reading the JSON response directly (Camera entities are few, fits in context)
- Check: Returns 1-2 Camera entities with all their components
- Check: Each entity includes many components (Transform, Camera, Visibility, etc.)
- Check: Component data is present (not empty `{}`)
- **Do NOT use jq or bash commands** - the response is returned directly in the tool output

### 5. Query with Mixed Fields (components + option + has)
Test all query data fields together:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"],
      "option": ["bevy_ecs::name::Name", "bevy_render::view::visibility::Visibility"],
      "has": ["bevy_camera::camera::Camera"]
    }
  }
  ```
- Verify by reading the JSON response directly (it's small enough to fit in context)
- Check: Returns entities with required Transform component
- Check: Optional Name and Visibility data included if present
- Check: `has` field returns boolean for Camera component presence (not component data)
- **Do NOT use jq or bash commands** - the response is returned directly in the tool output

### 6. Query with Filter Omitted vs Empty Object
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

- Verify by comparing the two JSON responses directly (both are small enough to fit in context)
- Check: Both produce identical results (omitted and empty object are equivalent)
- Check: Same entity count and entity IDs in both results
- **Do NOT use jq or bash commands** - compare the responses returned directly in the tool output

### 7. Error Case: Invalid Option Value
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
- Verify by reading the error message directly from the tool output
- Check: Returns clear error message about invalid ComponentSelector format
- Check: Error indicates expected "all" or array of strings
- **Do NOT use jq or bash commands** - the error is returned directly in the tool output

## Expected Results
- Array syntax for `option` works (backward compatibility)
- Empty option defaults correctly
- "all" option with Camera filter returns small inline response with full component data
- Mixed usage of components, option, and has fields succeeds
- Filter omission and empty object are equivalent
- Invalid option values produce clear error messages

## Failure Criteria
STOP if: Query returns incorrect data, serialization fails, backward compatibility breaks, or error handling doesn't produce clear messages.
