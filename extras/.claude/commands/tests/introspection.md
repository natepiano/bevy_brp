# Introspection Tests

## Objective
Validate BRP introspection capabilities including RPC discovery, schema operations, and component/resource listing.

## Test Steps

### 1. RPC Method Discovery
- Execute `mcp__brp__bevy_rpc_discover`
- Verify comprehensive method list is returned
- Check method schemas are properly formatted

### 2. Registry Schema Discovery
- Execute `mcp__brp__bevy_registry_schema` with filters (avoid large response)
- Use `with_crates: ["bevy_transform"]` filter
- Verify schema objects include shortPath, typePath, properties

### 3. Component Listing
- Execute `mcp__brp__bevy_list` (without entity parameter)
- Verify all registered components are listed
- Check for presence of common components like Transform

### 4. Resource Listing  
- Execute `mcp__brp__bevy_list_resources`
- Verify registered resources are returned
- Check for typical resources like Time

### 5. Entity-Specific Component Listing
- Execute `mcp__brp__bevy_list` with entity ID (try entity 0)
- Verify components on specific entity are listed
- Check response format is correct

## Expected Results
- ✅ RPC discovery returns complete method list
- ✅ Registry schema provides filtered type information
- ✅ Component listing shows registered types
- ✅ Resource listing shows available resources  
- ✅ Entity-specific listing works correctly

## Failure Criteria
STOP if: RPC discovery fails, schema operations error, or listing methods return malformed responses.