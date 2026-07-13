# Introspection Tests

## Objective
Validate BRP introspection capabilities including RPC discovery, schema operations, and component/resource listing.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing introspection functionality, not app management.

## Test Steps

### 1. RPC Method Discovery
- Execute `mcp__brp__rpc_discover` with port parameter
- Verify response includes at least 20 methods
- Check for presence of core methods: `world.list_components`, `world.query`, `world.spawn_entity`, `rpc.discover`
- Check for exact presence of brp_extras methods: `brp_extras/screenshot`, `brp_extras/shutdown`
- Check for exact absence of `brp_extras/screenshot_entity` and
  `brp_extras/find_entities_by_name`
- Do not use MCP tool-list inspection as proof here; this case covers only the BRP
  methods returned by `rpc.discover`
- Check for the application-defined method: `test/multiply`
- Verify response includes OpenRPC version and server info

### 2. Application-defined Method Execution
- Read `test-app/examples/extras_plugin.rs` to determine the parameters accepted by `test/multiply`
- Execute `mcp__brp__brp_execute` with that method, integer `value` and `factor` parameters, and the assigned port
- Verify the response echoes `value` and `factor` and returns their product
- Call `test/multiply` without `factor`
- Verify the application error retains JSON-RPC code `-32602` and data describing the expected fields
- Call an unregistered `test/missing` method
- Verify the error identifies the discovery stage and lists `test/multiply` among the available methods

### 3. Registry Schema Discovery
- Execute `mcp__brp__registry_schema` with port parameter and filters:
  - Use `with_crates: ["bevy_transform"]` filter to avoid large response
- Verify response returns around 50 schemas
- Check for specific schemas: `Transform`, `GlobalTransform`
- Verify schema objects include required fields: `shortPath`, `typePath`, `kind`, `reflectTypes`
- Verify Transform schema has `properties` with `translation`, `rotation`, `scale` fields

### 4. Component Listing
- Execute `mcp__brp__world_list_components` with port parameter (without entity parameter)
- Verify response returns around 95+ registered components
- Check for presence of specific components: `Transform`, `Name`, `Camera`, `Visibility`
- Verify response format includes component count in metadata

### 5. Resource Listing
- Execute `mcp__brp__world_list_resources` with port parameter
- Verify response returns around 10+ registered resources
- Check for specific resources: `ClearColor`, `Time<()>`, `Time<Real>`, `Time<Virtual>`
- Verify response format includes resource count in metadata

### 6. Entity-Specific Component Listing (Positive Case)
- Execute `mcp__brp__world_query` with proper parameters:
  - `filter: {"with": ["bevy_transform::components::transform::Transform"]}`
  - `data: {"components": ["bevy_transform::components::transform::Transform"]}`
- Verify query returns entities with Transform components
- Pick the first valid entity ID from the results
- Execute `mcp__brp__world_list_components` with that valid entity ID parameter
- Verify components are listed for the existing entity (should include Transform, GlobalTransform, etc.)
- Check response format includes component count and component list

### 7. Entity-Specific Component Listing (Negative Case)
- Execute `mcp__brp__world_list_components` with port parameter and invalid entity ID (0)
- Verify proper error response is returned (not a crash)
- Confirm error message contains "not a valid entity" or similar indication
- Verify response has status "error" and proper call_info

## Expected Results
- ✅ RPC discovery returns complete method list
- ✅ RPC discovery includes `brp_extras/screenshot` and excludes the prohibited
  screenshot-entity and extras name-discovery BRP methods
- ✅ Application-defined BRP method execution accepts typed parameters and preserves handler errors
- ✅ Registry schema provides filtered type information
- ✅ Component listing shows registered types
- ✅ Resource listing shows available resources
- ✅ Entity-specific listing works with valid entities
- ✅ Entity-specific listing properly rejects invalid entities with informative errors

## Failure Criteria
STOP if: RPC discovery fails, schema operations error, or listing methods return malformed responses.
