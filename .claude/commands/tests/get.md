# Get Strict Parameter Tests

## Objective
Validate the `strict` parameter behavior in `bevy_get` operations, testing both lenient (default) and strict modes with valid and invalid component requests.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing operations, not app management.

## Test Steps

### 1. Setup - Spawn Test Entity
- Execute `mcp__brp__bevy_spawn` with Transform and Name components
- Use format: 
  ```json
  {
    "bevy_transform::components::transform::Transform": {
      "translation": [5, 10, 15],
      "rotation": [0, 0, 0, 1],
      "scale": [1, 1, 1]
    },
    "bevy_ecs::name::Name": "StrictTestEntity"
  }
  ```
- Save the returned entity ID for all subsequent tests

### 2. Test: Strict false with 1 invalid component
- Execute `mcp__brp__bevy_get` with:
  - entity: [saved entity ID]
  - components: `["NonExistentComponent"]`
  - strict: `false` (or omit for default)
  - port: 20115
- **Expected**: Success response with errors object containing the invalid component
- **Verify**: 
  - Response has `errors` field with entry for "NonExistentComponent"
  - Message shows "Retrieved 0 components" (component_count: 0)

### 3. Test: Strict false with 1 invalid and 1 valid component
- Execute `mcp__brp__bevy_get` with:
  - entity: [saved entity ID]
  - components: `["bevy_transform::components::transform::Transform", "InvalidComponent::DoesNotExist"]`
  - strict: `false`
  - port: 20115
- **Expected**: Success response with Transform data and error for invalid component
- **Verify**: 
  - Response contains Transform component data
  - Response has `errors` field with entry for "InvalidComponent::DoesNotExist"
  - Message shows "Retrieved 1 components" (component_count: 1)

### 4. Test: Strict true with 1 invalid component
- Execute `mcp__brp__bevy_get` with:
  - entity: [saved entity ID]
  - components: `["ThisComponent::DoesNotExist"]`
  - strict: `true`
  - port: 20115
- **Expected**: Error response (not success)
- **Verify**: Operation fails with error about invalid component

### 5. Test: Strict true with 1 invalid and 1 valid component
- Execute `mcp__brp__bevy_get` with:
  - entity: [saved entity ID]
  - components: `["bevy_ecs::name::Name", "AnotherInvalid::Component"]`
  - strict: `true`
  - port: 20115
- **Expected**: Error response (not success)
- **Verify**: Operation fails even though Name component exists, because of the invalid component

### 6. Test: Strict true with 2 valid components
- Execute `mcp__brp__bevy_get` with:
  - entity: [saved entity ID]
  - components: `["bevy_transform::components::transform::Transform", "bevy_ecs::name::Name"]`
  - strict: `true`
  - port: 20115
- **Expected**: Success response with both components
- **Verify**: 
  - Response contains Transform component data matching spawned values
  - Response contains Name component with "StrictTestEntity"
  - No errors field in response
  - Message shows "Retrieved 2 components" (component_count: 2)

### 7. Cleanup
- Execute `mcp__brp__bevy_destroy` to remove test entity
- Verify entity is properly destroyed

## Expected Results
- ✅ Strict false allows partial success with invalid components
- ✅ Strict false returns valid component data alongside errors
- ✅ Strict true fails entirely when any component is invalid
- ✅ Strict true succeeds only when all components are valid
- ✅ Default behavior matches strict false (lenient mode)

## Failure Criteria
STOP if: Strict parameter doesn't control error handling as expected, valid components aren't returned in lenient mode, or strict mode doesn't fail on invalid components.