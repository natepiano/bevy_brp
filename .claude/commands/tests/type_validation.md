# Type Schema Comprehensive Validation Test

## Objective
Systematically validate ALL available types by discovering their schema and attempting to spawn/insert and mutate using every mutation path. This test ensures type schema information is accurate and all operations function correctly.

**NOTE**: The extras_plugin app is already running on the specified port - focus on comprehensive type validation.

## CRITICAL INSTRUCTIONS
- **USE ONLY MCP BRP TOOLS** - Never use bash commands, jq, or external JSON parsing
- **WORK WITH TOOL RESPONSES DIRECTLY** - Extract data from the structured JSON responses returned by MCP tools
- **NO EXTERNAL COMMANDS** - All data manipulation must be done within the test logic using tool response data

## Test Strategy
1. Discover all available types using `mcp__brp__bevy_list`
2. Get type schema for ALL discovered types using `mcp__brp__brp_type_schema`
3. For each type that supports spawn/insert: attempt to spawn using the schema format
4. For each type that supports mutate: test EVERY mutation path
5. **STOP ON FIRST FAILURE** to identify issues immediately

## Test Steps

### 1. Discover All Available Component Types

Execute `mcp__brp__bevy_list` with the test port:
```json
{
  "port": [TEST_PORT]
}
```

**Extract component types from the MCP tool response**: Use the `result` array directly from the tool response - this contains the list of all component type names.

### 2. Batch Type Schema Discovery

Execute `mcp__brp__brp_type_schema` with ALL discovered types from step 1:
```json
{
  "types": [/* use the entire result array from bevy_list response */],
  "port": [TEST_PORT]
}
```

**Extract schema data from the MCP tool response**: Use `result.type_info` object directly from the tool response. Each key is a type name, each value contains the schema information.

**VALIDATION CHECKPOINT**: Verify the tool response has `status: "success"` and `result.type_info` contains entries for all requested types.

### 3. Component Spawn/Insert Validation

**Work with schema data directly from tool response**: For each type in the `result.type_info` object from the `brp_type_schema` response:

#### 3a. Check Spawn Support
If the type's `supported_operations` array contains "spawn":
- **Extract spawn format directly**: Use the `spawn_format` value from the type's schema data
- Execute `mcp__brp__bevy_spawn`:
  ```json
  {
    "components": {
      "[TYPE_NAME]": [SPAWN_FORMAT_VALUE_FROM_SCHEMA]
    },
    "port": [TEST_PORT]
  }
  ```
- **CRITICAL SPAWN FAILURE PROTOCOL**:
  - If spawn fails, IMMEDIATELY:
    1. Display the actual `spawn_format` JSON value from the schema
    2. Display the error message from BRP
    3. Explain WHY it failed (e.g., "Schema provided `[null]` but BRP expects plain u64")
    4. STOP TESTING - do not continue to other types
  - If spawn succeeds, extract and record entity ID from the successful response (`result.entity`)

#### 3b. Check Insert Support  
If the type's `supported_operations` array contains "insert" and we have a spawned entity:
- **Use the same spawn format**: Take the `spawn_format` value from the schema data
- Execute `mcp__brp__bevy_insert`:
  ```json
  {
    "entity": [ENTITY_ID_FROM_SPAWN_RESPONSE],
    "components": {
      "[TYPE_NAME]": [SPAWN_FORMAT_VALUE_FROM_SCHEMA]
    },
    "port": [TEST_PORT]
  }
  ```
- **STOP IF INSERT FAILS**

### 4. Comprehensive Mutation Path Validation

**Work with schema mutation data directly**: For each type where `supported_operations` array contains "mutate":

#### 4a. Prepare Test Entity
If no entity exists with this component:
- Spawn entity with the component using the `spawn_format` from schema (if spawn is supported)
- Or find existing entity with this component using `mcp__brp__bevy_query`

#### 4b. Test Every Mutation Path
**Use mutation data directly from schema**: For EACH path in the type's `mutation_info` object:

**For standard paths (have `example` field):**
- **Extract example directly**: Use the `example` value from the mutation path data
- Execute `mcp__brp__bevy_mutate_component`:
  ```json
  {
    "entity": [ENTITY_ID_FROM_SPAWN_OR_QUERY],
    "component": "[TYPE_NAME]",
    "path": "[PATH_KEY_FROM_MUTATION_INFO]",
    "value": [EXAMPLE_VALUE_FROM_MUTATION_INFO],
    "port": [TEST_PORT]
  }
  ```
- **STOP IF MUTATION FAILS**

**For enum paths (have `enum_variants` field):**
- **Use first variant directly**: Take the first value from the `enum_variants` array
- Execute mutation with the enum variant value
- **STOP IF MUTATION FAILS**

**For Option paths (have `example_some` and `example_none` fields):**
- **Test both examples**: Use both `example_some` and `example_none` values from the mutation path data
- Execute mutations for both cases
- **STOP IF EITHER MUTATION FAILS**

### 5. Mutation Verification

**Verify changes using MCP tool responses**: After each successful mutation:
- Execute `mcp__brp__bevy_get` to retrieve the component:
  ```json
  {
    "entity": [ENTITY_ID],
    "components": ["[TYPE_NAME]"],
    "port": [TEST_PORT]
  }
  ```
- **Compare values directly**: Use the component data from the `result` object of the get response to verify the mutation took effect
- **STOP IF VERIFICATION FAILS** - if the retrieved value doesn't match the expected mutated value

### 6. Progress Tracking

Log progress after each type:
```
✅ TYPE_NAME: [spawn_status] [insert_status] [X/Y_mutations_tested]
```

Where:
- `spawn_status`: "SPAWNED" or "NO_SPAWN_SUPPORT" or "SPAWN_FAILED"
- `insert_status`: "INSERTED" or "NO_INSERT_SUPPORT" or "INSERT_FAILED"  
- `X/Y_mutations_tested`: "5/5_PASSED" or "3/5_FAILED_AT_PATH_.translation.x"

### 7. Final Summary

Provide comprehensive results:
- Total types tested: X
- Types with spawn support: Y (Z successful)
- Types with insert support: A (B successful)
- Types with mutation support: C (D successful)
- Total mutation paths tested: E (F successful)
- First failure point (if any): [TYPE_NAME] [OPERATION] [PATH] [ERROR]

## Success Criteria

✅ Test passes when:
- All discoverable types can be loaded via type schema
- All spawn operations succeed using schema-provided formats
- All insert operations succeed using schema-provided formats  
- ALL mutation paths work using schema-provided examples
- All mutations can be verified via component retrieval
- No unexpected failures or inconsistencies

## Failure Investigation

**IMMEDIATE STOP CONDITIONS**:
- Type schema discovery fails
- Spawn fails for any type that claims spawn support
- Insert fails for any type that claims insert support
- ANY mutation path fails using schema-provided examples
- Mutation verification shows values didn't change as expected

**SPAWN FORMAT FAILURE REPORTING**:
When a spawn operation fails using the schema-provided format:
1. **SHOW THE ACTUAL JSON**: Display the exact `spawn_format` value from the schema
2. **SHOW THE ERROR**: Display the complete error message from BRP
3. **EXPLAIN THE MISMATCH**: Clearly explain why the schema format doesn't match BRP's expectations
   - Example: "Schema provides `[null]` (array format) but BRP expects `0` (plain u64)"
   - Example: "Schema provides `{}` (empty object) but BRP requires all fields populated"
4. **STOP IMMEDIATELY**: Do not continue testing other types after the first spawn format failure

**Investigation Steps**:
1. Record exact failure point: [TYPE_NAME] [OPERATION] [PATH] [INPUT] [ERROR]
2. Check if issue is with test logic or schema generation
3. Verify component/entity state before operation
4. Compare expected vs actual mutation behavior
5. Check for serialization/deserialization issues

## Notes

- This test is **exhaustive** - it validates every type and every mutation path
- **STOP ON FIRST FAILURE** principle ensures rapid issue identification
- Test entities are created as needed for mutation testing
- Progress logging helps identify where validation stands
- Both positive testing (operations should work) and verification (changes took effect)