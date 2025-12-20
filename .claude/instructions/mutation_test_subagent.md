# Instructions

## Configuration Parameters

These config values are provided:
- PORT: BRP port number for MCP tool operations

## Your Job

**Execute mutation test operations in an infinite loop until operation_manager.py returns `"status": "finished"`.**

**CRITICAL CONSTRAINTS**:
- The ONLY source of operations is operation_manager.py
- The ONLY exit conditions are: receiving `"status": "finished"` OR encountering an unrecoverable error
- **SCRIPT EXECUTION PROHIBITION**: You are ONLY allowed to execute this exact command:
  ```bash
  python3 .claude/scripts/mutation_test/operation_manager.py --port PORT --action get-next
  ```
- **NEVER call `--action update`** - the post-tool hook handles ALL status updates automatically. If you do call it this way, you will break the test.
- **DO NOT** try to report operation success/failure yourself - just execute operations and move to next
- Running ANY other script, Python file, or bash command is a **TEST FAILURE**
- If you think you need to run something else, you have misunderstood your job - STOP and report error

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <OperationLoop/>
**STEP 2:** Execute <ReportCompletion/>
</ExecutionSteps>

<OperationLoop>
**THIS IS AN INFINITE LOOP. DO NOT STOP UNTIL YOU RECEIVE `"status": "finished"` OR HIT AN UNRECOVERABLE ERROR.**

REPEAT these steps continuously:

1. **Request next operation**:
   ```bash
   python3 .claude/scripts/mutation_test/operation_manager.py --port PORT --action get-next
   ```

2. **Parse JSON response and check status**:

   If `"status": "finished"`:
   - **EXIT LOOP IMMEDIATELY**
   - Proceed to <ReportCompletion/>

   If `"status": "next_operation"`:
   - Extract `operation` object from response
   - Continue to step 3

   If any other status value:
   - **EXIT LOOP WITH ERROR**: "Invalid response from operation_manager.py: ${status}"

3. **Prepare operation parameters**:
   - Extract parameters from `operation` object
   - If operation has `entity_id_substitution` field → Execute <EntityIdSubstitution/>

4. **Execute the operation**:
   - Call MCP tool from `operation.tool` with parameters DIRECTLY (no conversion)

5. **Evaluate result**:

   If operation SUCCESS:
   - **RETURN TO STEP 1** (request next operation)

   If operation FAIL:
   - **EXIT LOOP WITH ERROR**: "Unrecoverable error at operation ${operation_id}: ${error}"

**END OF LOOP ITERATION - RETURN TO STEP 1**
</OperationLoop>

<ReportCompletion>
Report final state based on how loop exited:

- If exited via `"status": "finished"`: "✅ All operations completed successfully"
- If exited via unrecoverable error: "❌ Stopped on unrecoverable error at operation ${operation_id}: ${error_message}"
- If exited via invalid status: "❌ Invalid response from operation_manager.py: ${status_value}"
</ReportCompletion>

## Entity ID Substitution

<EntityIdSubstitution>
Some operations contain placeholder entity IDs that must be replaced with real entities before execution.

**ONLY IF** an operation has `entity_id_substitution` field (contains the placeholder entity ID):

1. **Query for all entities**:
   ```bash
   mcp__brp__world_query(data={}, filter={}, port=PORT)
   ```

2. **Get a real entity ID**:
   - Extract entity IDs from the result array
   - If operation has `entity` field: exclude that entity from the list
   - Use the first remaining entity ID

3. **Replace all placeholders**:
   - Recursively search through `operation["value"]` field
   - Replace ALL instances of the placeholder (e.g., `8589934670`) with the real entity ID
   - This handles both simple cases (`"value": 8589934670`) and nested cases (`{"position": {"Entity": 8589934670}}`)

**Example 1 - Simple:**
```json
Before: {"value": 8589934670, "entity_id_substitution": 8589934670}
After:  {"value": 4294967262}  // Queried entity, placeholder removed
```

**Example 2 - Nested:**
```json
Before: {
  "value": {"position": {"Entity": 8589934670}, "mode": {"Entity": 8589934670}},
  "entity_id_substitution": 8589934670
}
After: {
  "value": {"position": {"Entity": 4294967262}, "mode": {"Entity": 4294967262}}
}
```

**Note**: The `entity` field is already set by the operation manager - don't modify it.
</EntityIdSubstitution>

## Query Result Validation

<QueryResultValidation>
Query result validation and entity ID propagation are handled automatically by the mutation test infrastructure.

When you execute `mcp__brp__world_query`, the post-tool hook will:
- Extract entities from the query result
- If entities found:
  - Add `"entity"` field to the query operation in the test plan
  - **Propagate the entity ID to all subsequent operations in the same test** that have `"entity": "USE_QUERY_RESULT"`
- If no entities found: Mark the query as FAIL with error "Query returned 0 entities"

**Your responsibility**: Just execute the query operation. If it fails (status = FAIL), stop execution immediately.

**Note**: You don't need to validate query results, propagate entity IDs, or look back at previous operations - the hook handles all of this automatically. Entity IDs are isolated to each test (don't cross type boundaries).
</QueryResultValidation>

## Error Handling

**When an operation fails**: STOP IMMEDIATELY - do not process remaining operations.
