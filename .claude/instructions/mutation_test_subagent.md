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
Executing the script in a loop via a for command is not allowed
this form of executino example is **not allowed**:
```
 for i in {7..84}; do python3 /Users/natemccoy/rust/bevy_brp/.claude/scripts/mutation_test_operation_update.py --file   /var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/mutation_test_subagent_30001_plan.json --operation-id $i --status FAIL --error "Connection failed: BRP server unavailable"; done
```

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

## JSON Primitive Rules

<JsonPrimitiveRules>
**CRITICAL JSON PRIMITIVE REQUIREMENTS**:
- ALL numeric values MUST be JSON numbers, NOT strings
- NEVER quote numbers: ❌ "3.1415927410125732" → ✅ 3.1415927410125732
- This includes f32, f64, u32, i32, ALL numeric types
- High-precision floats like 3.1415927410125732 are STILL JSON numbers
- ALL boolean values MUST be JSON booleans, NOT strings
- NEVER quote booleans: ❌ "true" → ✅ true, ❌ "false" → ✅ false
- Numbers: ✅ 3.14, ✅ 42, ✅ 3.1415927410125732
- Booleans: ✅ true, ✅ false
- NEVER: ❌ "3.14", ❌ "42", ❌ "true", ❌ "false"
- If you get "invalid type: string" error, you quoted a number or boolean

**COMMON MISTAKES THAT CAUSE STRING CONVERSION**:
❌ Converting example to string: `str(example)` or `f"{example}"`
❌ String interpolation in values: treating numbers as text
❌ Copy-pasting example values as strings instead of raw values
❌ Using string formatting functions on numeric values

✅ CORRECT: Use the example value DIRECTLY from the type guide without any string conversion
✅ When constructing mutation params: assign the value AS-IS from the example
✅ Keep numeric types as numbers, boolean types as booleans throughout your code

**MANDATORY PRE-SEND VERIFICATION**:
Before EVERY mutation request with a numeric or boolean value:
1. **CHECK**: Look at the value you're about to send in `params["value"]`
2. **VERIFY**: If it's a number like `42`, ensure you're sending the NUMBER 42, not the STRING "42"
3. **TEST**: In your JSON structure, it should appear as `"value": 42` NOT `"value": "42"`
4. **CONFIRM**: No quotes around numbers or booleans in the actual value field

**VERIFICATION EXAMPLES**:
- ❌ WRONG: `{"value": "42"}` - This is a STRING "42"
- ✅ CORRECT: `{"value": 42}` - This is a NUMBER 42
- ❌ WRONG: `{"value": "true"}` - This is a STRING "true"
- ✅ CORRECT: `{"value": true}` - This is a BOOLEAN true
- ❌ WRONG: `{"value": "3.14"}` - This is a STRING "3.14"
- ✅ CORRECT: `{"value": 3.14}` - This is a NUMBER 3.14

**ERROR RECOVERY PROTOCOL**:
If you receive error: `invalid type: string "X", expected [numeric/boolean type]`:
1. **RECOGNIZE**: This means you DEFINITELY sent "X" as a quoted string
2. **DO NOT** report this as a test failure - this is YOUR bug, not a BRP bug
3. **FIX IMMEDIATELY**: Retry the SAME mutation with the value as an unquoted primitive
4. **VERIFY**: Before retry, confirm your value is a number/boolean, NOT a string
5. **ONLY FAIL**: If the retry also fails with a DIFFERENT error message

**VALIDATION**: Before sending ANY mutation, verify primitives are unquoted
</JsonPrimitiveRules>

## Error Recovery

<ErrorRecovery>
**When an operation fails, check the error message and apply recovery:**

### Invalid Type String Error

**Pattern**: Error contains `"invalid type: string"`

**Cause**: You sent a number/boolean as a string (YOUR bug, not BRP's)

**Recovery**: Follow the ERROR RECOVERY PROTOCOL in <JsonPrimitiveRules/>
1. Parse the error to identify which parameter has the wrong type
2. Convert the value to proper JSON type (remove quotes from primitives)
3. Re-execute the operation with corrected value
4. Update per <UpdateOperationViaScript/> with retry count:

**If retry succeeds**:
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --retry-count 1 \
  [--entity-id ENTITY_ID] or [--entities "CSV"]
```

**If retry fails**:
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

**If retry succeeds**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --retry-count 1 \
  [--entity-id ENTITY_ID] or [--entities "CSV"]
```

**If retry fails**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  i-file TEST_PLAN_FILE \
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

**If retry succeeds**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status SUCCESS \
  --retry-count 1 \
  [--entity-id ENTITY_ID] or [--entities "CSV"]
```

**If retry fails**
```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE"
```

### All Other Errors

No recovery - just mark `status: "FAIL"`, record `error` per <UpdateOperationViaScript/>, and **STOP IMMEDIATELY**.

```bash
python3 .claude/scripts/mutation_test_operation_update.py \
  --file TEST_PLAN_FILE \
  --operation-id OPERATION_ID_FROM_JSON \
  --status FAIL \
  --error "ERROR_MESSAGE_FROM_TOOL"
```

**CRITICAL**: Stop execution immediately on first failure. Do NOT process any remaining operations. Mark only the failed operation and return.
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
- Use curl or make direct HTTP requests
- Use jq, sed, awk, or other JSON manipulation tools
- Write custom Python/Bash code or heredoc patterns for updates

**You MUST follow <UpdateOperationViaScript/> exactly for all test plan updates.**
