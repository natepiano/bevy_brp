# Mutation Test Executor Instructions

## ⚠️ CRITICAL: AVAILABLE TOOLS (READ THIS FIRST)

**YOU HAVE ACCESS TO EXACTLY 7 TOOLS IN THIS ENVIRONMENT:**

✅ **THE ONLY TOOLS YOU CAN USE:**
1. `Read` - Read the test plan file ONCE at start
2. `Bash` - ONLY to execute: `python3 .claude/scripts/mutation_test_operation_update.py`
3. `mcp__brp__world_spawn_entity` - Spawn entities
4. `mcp__brp__world_query` - Query entities (including entity ID substitution)
5. `mcp__brp__world_mutate_components` - Mutate component fields
6. `mcp__brp__world_mutate_resources` - Mutate resource fields
7. `mcp__brp__world_insert_resources` - Insert/update resources

🚫 **TOOLS THAT DO NOT EXIST IN THIS ENVIRONMENT:**
- curl or HTTP requests - NOT AVAILABLE
- jq, sed, awk, or JSON manipulation - NOT AVAILABLE

**TEST PLAN UPDATES:**
- The ONLY way to update the test plan: `Bash` tool with `mutation_test_operation_update.py`

**NEVER**
- NEVER create a custom script of any sort - NO PYTHON3, NO BASH, NOTHING!!

---

## Configuration Parameters

This subagent receives configuration from the parent command via Task prompt:
- TEST_PLAN_FILE: Path to the JSON test plan file to execute
- PORT: BRP port number for MCP tool operations

These values are provided by mutation_test.md when launching subagents.

## Your Job

**Execute the test plan and update results after each operation.**

## Test Plan Updates

**CRITICAL**: Each operation has an `operation_id` field. You MUST update after every operation using <UpdateOperationViaScript/>.

## Execution Steps

1. **Read test plan once**:
   - Use Read tool on TEST_PLAN_FILE path
   - Parse the JSON to identify operations and their `operation_id` fields

2. **Execute operations sequentially**:
   - For each test in `tests` array:
     - For each operation in `operations` array:
       - **Note the operation's `operation_id` field** (you'll need it for <UpdateOperationViaScript/>)
       - Apply entity ID substitution if `entity_id_substitution` field exists (see <EntityIdSubstitution/>)
       - Execute the MCP tool specified in `tool` field (see <OperationExecution/>)
       - If operation succeeds:
         - Update operation per <UpdateOperationViaScript/> with status SUCCESS
         - Continue to next operation
       - If operation fails:
         - Apply error recovery if applicable (see <ErrorRecovery/>)
         - If recovery succeeds: update with SUCCESS and continue
         - If recovery fails or not applicable:
           - Update operation per <UpdateOperationViaScript/> with status FAIL
           - **STOP IMMEDIATELY** - return without processing remaining operations

3. **Finish execution**:
   - After all operations complete successfully, or after first failure, execution is done
   - No final output needed - all results are in the test plan file

## Entity ID Substitution

<EntityIdSubstitution>
**BEFORE executing any operation that has `entity_id_substitution` field:**

1. **Get available entities using MCP tool**:
   ```
   CORRECT: Use mcp__brp__world_query(data={}, filter={}, port=PORT)
   WRONG: curl -X POST http://localhost:PORT/brp (FORBIDDEN!)
   WRONG: Bash command="curl ..." (FORBIDDEN!)
   WRONG: Python script to call curl (FORBIDDEN!)
   ```
   - Extract entity IDs from the result's "entities" field
   - Use first entity ID for substitutions

2. **Apply substitutions**:
   - For each `path → marker` in `entity_id_substitution`:
     - If marker is `"QUERY_ENTITY"`:
       - Navigate to the path in operation params
       - Replace the placeholder value with the first available entity ID

   **Example**:
   ```
   Original operation:
   {
     "tool": "mcp__brp__world_spawn_entity",
     "components": {"bevy_ecs::hierarchy::Children": [8589934670]},
     "entity_id_substitution": {"components.bevy_ecs::hierarchy::Children[0]": "QUERY_ENTITY"}
   }

   After substitution (using entity ID 4294967297 from query):
   {
     "components": {"bevy_ecs::hierarchy::Children": [4294967297]}
   }
   ```

3. **For operations with `"entity": "USE_QUERY_RESULT"`**:
   - Replace with actual entity ID from previous query operation's `result_entities[0]`
</EntityIdSubstitution>

## Operation Execution

<OperationExecution>
**For each operation, invoke the MCP tool specified in the `tool` field:**

<UpdateOperationViaScript>
**THE ONLY WAY to update the test plan after an operation:**

Use the Bash tool to execute ONLY this exact command pattern:

```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS_OR_FAIL \
  [conditional parameters below]
```

**Required parameters (ALWAYS include):**
- `--file TEST_PLAN_FILE` - Path to test plan JSON file
- `--operation-id OPERATION_ID_FROM_JSON` - The operation's `operation_id` field value from JSON
- `--status SUCCESS|FAIL` - Result status

**Conditional parameters (include based on operation type and result):**
- `--entity-id ENTITY_ID` - For spawn operations that succeed
- `--entities "ID1,ID2,..."` - For query operations that succeed (comma-separated entity IDs)
- `--error "MESSAGE"` - For operations that fail
- `--retry-count N` - If this is a retry after error recovery

**This is the ONLY acceptable method. NO other approaches are allowed.**

</UpdateOperationViaScript>

### mcp__brp__world_spawn_entity

**Execute MCP tool**:
- Tool: `mcp__brp__world_spawn_entity`
- Parameters: `components` (from operation), `port` (from operation)

<UpdateOperationViaScript/>:

IF SUCCESS:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --entity-id ENTITY_ID_FROM_TOOL_RESULT
```

IF FAILURE:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```

### mcp__brp__world_query

**Execute MCP tool**:
- Tool: `mcp__brp__world_query`
- Parameters: `filter`, `data`, `port` (all from operation)

<UpdateOperationViaScript/>:

IF SUCCESS:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --entities "ENTITY_IDS_COMMA_SEPARATED"
```
Example: `--entities "4294967200,8589934477"`

IF FAILURE:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```

### mcp__brp__world_mutate_components

**Execute MCP tool**:
- Tool: `mcp__brp__world_mutate_components`
- Parameters: `entity`, `component`, `path`, `value`, `port` (all from operation)
  - Note: `entity` should be after USE_QUERY_RESULT substitution if applicable

<UpdateOperationViaScript/>:

IF SUCCESS:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS
```

IF FAILURE:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```

### mcp__brp__world_mutate_resources

**Execute MCP tool**:
- Tool: `mcp__brp__world_mutate_resources`
- Parameters: `resource`, `path`, `value`, `port` (all from operation)

<UpdateOperationViaScript/>:

IF SUCCESS:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS
```

IF FAILURE:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```

### mcp__brp__world_insert_resources

**Execute MCP tool**:
- Tool: `mcp__brp__world_insert_resources`
- Parameters: `resource`, `value`, `port` (all from operation)

<UpdateOperationViaScript/>:

IF SUCCESS:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS
```

IF FAILURE:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```
</OperationExecution>

## Error Recovery

<ErrorRecovery>
**When an operation fails, check the error message and apply recovery:**

### Invalid Type String Error

**Pattern**: Error contains `"invalid type: string"`

**Cause**: You sent a number/boolean as a string (YOUR bug, not BRP's)

**Critical Requirements**:
- ALL numeric values MUST be JSON numbers, NOT strings: `{"value": 42}` NOT `{"value": "42"}`
- ALL boolean values MUST be JSON booleans, NOT strings: `{"value": true}` NOT `{"value": "true"}`
- Applies to ALL numeric types (f32, f64, u32, i32, etc.) and booleans
- Common mistake: Converting values to strings via `str()`, `f"{}"`, or string interpolation
- Correct approach: Use example values DIRECTLY from type guide without conversion

**Recovery**:
1. Parse error to identify which parameter has the wrong type
2. Convert to proper JSON type (remove quotes from primitives)
3. Re-execute operation with corrected value
4. Update per <UpdateOperationViaScript/> with `--retry-count 1`
5. DO NOT report as test failure - this is YOUR bug, not BRP's
6. Only fail if retry produces DIFFERENT error

**Before EVERY mutation**: Verify no quotes around numbers/booleans in value field.

### UUID Parsing Error

**Pattern**: Error contains `"UUID parsing failed"` AND `'found \`"\` at'`

**Cause**: You double-quoted a UUID string

**Recovery**:
1. Find UUID value in operation params
2. Remove extra quotes: `"\"550e8400-e29b-41d4-a716-446655440000\""` → `"550e8400-e29b-41d4-a716-446655440000"`
3. Re-execute operation
4. Update per <UpdateOperationViaScript/> with `--retry-count 1`

### Enum Variant Error

**Pattern**: Error contains `"unknown variant"` with escaped quotes like `\"VariantName\"`

**Cause**: You double-quoted an enum variant

**Recovery**:
1. Remove extra quotes: `"\"Low\""` → `"Low"`
2. Re-execute operation
3. Update per <UpdateOperationViaScript/> with `--retry-count 1`
4. DO NOT report as test failure - this is YOUR bug

### Parameter Extraction Error

**Pattern**: Error contains `"Unable to extract parameters"`

**Cause**: Tool framework issue with parameter order

**Recovery**:
1. Reorder parameters in your tool call (change the order you pass them)
2. Re-execute operation with reordered parameters
3. Update per <UpdateOperationViaScript/> with `--retry-count 1`

### All Other Errors

No recovery - mark FAIL, record error per <UpdateOperationViaScript/>, and **STOP IMMEDIATELY**.

**CRITICAL**: Stop execution immediately on first failure. Do NOT process any remaining operations. Mark only the failed operation and return.
</ErrorRecovery>
