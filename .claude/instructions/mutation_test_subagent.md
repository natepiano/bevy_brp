# Mutation Test Executor Instructions

## Configuration Parameters

This subagent receives configuration from the parent command via Task prompt:
- TEST_PLAN_FILE: Path to the JSON test plan file to execute
- PORT: BRP port number for MCP tool operations

These values are provided by mutation_test.md when launching subagents.

## Your Job

**Execute the test plan and update results per <UpdateOperationViaScript/> after each operation.**

**CRITICAL**: You MUST update the test plan per <UpdateOperationViaScript/> after EVERY operation.

## MANDATORY TOOL USAGE RULES

**ABSOLUTE PROHIBITION**: NO curl, NO Write tool, NO custom scripts

❌ **FORBIDDEN - IMMEDIATE TEST FAILURE**:
- Using curl or making direct HTTP requests to any port
- Using Write tool to update test plan (use <UpdateOperationViaScript/> instead)
- Writing custom Python/Bash scripts or heredoc patterns
- Using jq, sed, awk, or any command-line JSON tools for test plan manipulation
- ANY tool not explicitly listed in the allowed list below

✅ **ALLOWED TOOLS ONLY** (these are the ONLY 7 tools you may use):
- `Read` - to read the test plan file ONCE at start
- `Bash` - ONLY to execute the exact command shown in <UpdateOperationViaScript/>
- `mcp__brp__world_spawn_entity` - for spawning entities
- `mcp__brp__world_query` - for querying entities (including entity ID substitution)
- `mcp__brp__world_mutate_components` - for mutating component fields
- `mcp__brp__world_mutate_resources` - for mutating resource fields
- `mcp__brp__world_insert_resources` - for inserting/updating resources

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
       - Apply error recovery if needed (see <ErrorRecovery/>)
       - **CRITICAL**: Update operation per <UpdateOperationViaScript/> with the operation's `operation_id`
       - Continue to next operation

3. **Finish execution**:
   - After all operations complete, execution is done
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

**Recovery**:
1. Parse the error to identify which parameter has the wrong type
2. Convert the value to proper JSON type:
   - `"42"` → `42` (number)
   - `"true"` → `true` (boolean)
   - `"[1,2]"` → `[1,2]` (array)
   - `"{\"x\":1}"` → `{"x":1}` (object)
3. Re-execute the operation with corrected value
4. Update per <UpdateOperationViaScript/> with retry count:

**If retry succeeds:**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --retry-count 1 \
  [--entity-id ENTITY_ID] or [--entities "CSV"]
```

**If retry fails:**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "NEW_ERROR_MESSAGE"
```

### UUID Parsing Error

**Pattern**: Error contains `"UUID parsing failed"` AND `'found \`"\` at'`

**Cause**: You double-quoted a UUID string

**Recovery**:
1. Find UUID value in operation params
2. Remove extra quotes: `"\"550e8400-e29b-41d4-a716-446655440000\""` → `"550e8400-e29b-41d4-a716-446655440000"`
3. Re-execute the operation
4. Update per <UpdateOperationViaScript/> with retry count:

**If retry succeeds:**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --retry-count 1 \
  [--entity-id ENTITY_ID] or [--entities "CSV"]
```

**If retry fails:**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE"
```

### Parameter Extraction Error

**Pattern**: Error contains `"Unable to extract parameters"`

**Cause**: Tool framework issue with parameter order

**Recovery**:
1. Reorder the parameters in your tool call (change the order you pass them)
2. Re-execute the operation with reordered parameters
3. Update per <UpdateOperationViaScript/> with retry count:

**If retry succeeds:**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --retry-count 1 \
  [--entity-id ENTITY_ID] or [--entities "CSV"]
```

**If retry fails:**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE"
```

### All Other Errors

No recovery - just mark `status: "FAIL"`, record `error` per <UpdateOperationViaScript/>, and continue to next operation.

```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```

**CRITICAL**: Never stop execution due to errors. Always complete all operations and update results per <UpdateOperationViaScript/>.
</ErrorRecovery>

## Complete Operation Flow Example

This section shows the complete flow for executing operations with updates per <UpdateOperationViaScript/> after each step.

**For each operation in the test plan:**

1. **Execute MCP tool** with parameters from operation
2. **Update operation per <UpdateOperationViaScript/>** with:
   - Operation's `operation_id` from JSON
   - Status: `SUCCESS` or `FAIL`
   - Result data: `--entity-id`, `--entities`, or neither
   - Error message: `--error` (if failed)
   - Retry count: `--retry-count` (if retried)
3. **Move to next operation**

**Why update after each operation:**
- Ensures processing can read partial results even if subagent crashes
- Provides incremental progress tracking
- Allows debugging of specific operation failures
- Atomic updates prevent JSON corruption

**Example sequence for a test with 3 operations:**
1. Read test plan file → parse JSON → note operation_id for each operation
2. Execute spawn operation (operation_id: 0) → update per <UpdateOperationViaScript/> with --status SUCCESS --entity-id 8589934477
3. Execute query operation (operation_id: 1) → update per <UpdateOperationViaScript/> with --status SUCCESS --entities "4294967200,8589934477"
4. Execute mutate operation (operation_id: 2) → update per <UpdateOperationViaScript/> with --status SUCCESS
5. Finish execution (no final output needed)

## Summary - The Only Way to Update Test Plans

**The ONLY acceptable workflow**:
1. Read tool (once) → parse JSON → note operation_id for each operation
2. MCP tool → get result
3. Update per <UpdateOperationViaScript/> with operation_id and result
4. Repeat steps 2-3 for each operation

**NEVER**:
- Use Write tool to update test plans
- Manually manipulate JSON yourself
- Use curl or make direct HTTP requests
- Use jq, sed, awk, or other JSON manipulation tools
- Write custom Python/Bash code or heredoc patterns for updates

**You MUST follow <UpdateOperationViaScript/> exactly for all test plan updates.**
