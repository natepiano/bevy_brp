# Query Filtered Operations Tests

## Objective
Validate `world.query` BRP method with large-result operations that require script-based validation: the "all" option, filter combinations, and filtered "all" queries.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing query operations, not app management.

## CRITICAL: Validation Script Usage

**NEVER use jq, grep, or any bash commands directly to validate results.**

All validation MUST use the pre-approved script: `.claude/scripts/integration_tests/query_validate.sh`

Available validation commands:
- `count_entities` - Count entities in result
- `validate_all_query` - Verify "all" query returns entities with multiple components
- `has_camera_excluded` - Verify no Camera entities present
- `validate_name_filter` - Verify all entities have Name and multiple components

**Do NOT issue ANY bash commands for validation - use the script exclusively.**

**IMPORTANT: Inline result fallback** — If a query result is returned inline (no file path), verify it directly from the tool output instead of using the script. Do NOT spend time trying to save inline results to files.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Query with "all" Syntax (New in 0.17.2)
Test the new `ComponentSelector::All` variant:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "option": "all"
    }
  }
  ```
- **Note**: Result will be written to temp file due to size
- Validate using script: `.claude/scripts/integration_tests/query_validate.sh validate_all_query <result_file>`
- Verify: Returns many entities from the app
- Verify: Each entity includes ALL its components in the response
- Verify: Response includes component data for all registered component types on each entity

### 3. Query with Filter: with + without
Test filter combinations:

- Execute `mcp__brp__world_query`:
  ```json
  {
    "data": {
      "option": "all"
    },
    "filter": {
      "with": ["bevy_transform::components::transform::Transform"],
      "without": ["bevy_camera::camera::Camera"]
    }
  }
  ```
- Validate using script: `.claude/scripts/integration_tests/query_validate.sh has_camera_excluded <result_file>`
- Verify: Returns entities with Transform but not Camera
- Verify: Camera entities are excluded from results

### 4. Query with "all" Option and Filter
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
- **Note**: Result will be written to temp file due to size
- Validate using script: `.claude/scripts/integration_tests/query_validate.sh validate_name_filter <result_file>`
- Verify: Returns only entities with Name component
- Verify: All components on matching entities are included in response

## Expected Results
- "all" syntax for `option` returns all components
- Filter with `with` and `without` works correctly
- "all" option combined with filter works

## Failure Criteria
STOP if: Query returns incorrect data, serialization fails, or the "all" option doesn't work as specified in Bevy 0.17.2.
