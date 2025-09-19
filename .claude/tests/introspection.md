# Introspection Tests

## Objective
Validate BRP introspection capabilities including RPC discovery, schema operations, and component/resource listing.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing introspection functionality, not app management.

## Test Steps

### 1. RPC Method Discovery
- Execute `mcp__brp__bevy_rpc_discover` with port parameter
- Verify response includes at least 20 methods
- Check for presence of core methods: `bevy/list`, `bevy/query`, `bevy/spawn`, `rpc.discover`
- Check for brp_extras methods: `brp_extras/screenshot`, `brp_extras/shutdown`
- Verify response includes OpenRPC version and server info

### 2. Registry Schema Discovery
- Execute `mcp__brp__bevy_registry_schema` with port parameter and filters:
  - Use `with_crates: ["bevy_transform"]` filter to avoid large response
- Verify response returns around 25 schemas
- Check for specific schemas: `Transform`, `GlobalTransform`
- Verify schema objects include required fields: `shortPath`, `typePath`, `kind`, `reflectTypes`
- Verify Transform schema has `properties` with `translation`, `rotation`, `scale` fields

### 3. Component Listing
- Execute `mcp__brp__bevy_list` with port parameter (without entity parameter)
- Verify response returns around 95+ registered components
- Check for presence of specific components: `Transform`, `Name`, `Camera`, `Visibility`
- Verify response format includes component count in metadata

### 4. Resource Listing  
- Execute `mcp__brp__bevy_list_resources` with port parameter
- Verify response returns around 10+ registered resources
- Check for specific resources: `ClearColor`, `Time<()>`, `Time<Real>`, `Time<Virtual>`
- Verify response format includes resource count in metadata

### 5. Entity-Specific Component Listing (Positive Case)
- Execute `mcp__brp__bevy_query` with proper parameters:
  - `filter: {"with": ["bevy_transform::components::transform::Transform"]}`
  - `data: {"components": ["bevy_transform::components::transform::Transform"]}`
- Verify query returns entities with Transform components
- Pick the first valid entity ID from the results
- Execute `mcp__brp__bevy_list` with that valid entity ID parameter
- Verify components are listed for the existing entity (should include Transform, GlobalTransform, etc.)
- Check response format includes component count and component list

### 6. Entity-Specific Component Listing (Negative Case)
- Execute `mcp__brp__bevy_list` with port parameter and invalid entity ID (0)
- Verify proper error response is returned (not a crash)
- Confirm error message contains "not a valid entity" or similar indication
- Verify response has status "error" and proper call_info

## Expected Results
- ✅ RPC discovery returns complete method list
- ✅ Registry schema provides filtered type information
- ✅ Component listing shows registered types
- ✅ Resource listing shows available resources  
- ✅ Entity-specific listing works with valid entities
- ✅ Entity-specific listing properly rejects invalid entities with informative errors

## Failure Criteria
STOP if: RPC discovery fails, schema operations error, or listing methods return malformed responses.