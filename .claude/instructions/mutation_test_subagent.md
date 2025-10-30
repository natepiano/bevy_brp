# Instructions

## Configuration Parameters

These config values are provided:
- PORT: BRP port number for MCP tool operations

## Your Job

**Execute mutation test operations in an infinite loop until operation_manager.py returns `"status": "finished"`.**

**CRITICAL: MCP TOOL PARAMETER FORMATTING**

⚠️ **DO NOT JSON-SERIALIZE PARAMETERS** ⚠️

When calling MCP tools, pass parameters as Python objects, NOT as JSON strings.

**The WRONG patterns** (ALL cause "invalid type: string" errors):
```python
# Read operation from operation_manager.py
response = # ... bash output from operation_manager.py
# Parse the bash output to get the operation dict
filter_param = operation["filter"]  # This is already a dict: {"with": ["Type"]}

# ❌ WRONG - Do NOT serialize to string:
filter_str = json.dumps(filter_param)
mcp__brp__world_query(data={}, filter=filter_str, port=30001)

# ❌ WRONG - Do NOT pretty-print:
filter_str = json.dumps(filter_param, indent=2)  # This adds \n newlines!
mcp__brp__world_query(data={}, filter=filter_str, port=30001)

# ❌ WRONG - Do NOT wrap in quotes:
mcp__brp__world_query(data={}, filter='{"with": ["Type"]}', port=30001)
mcp__brp__world_query(data={}, filter="{\"with\": [\"Type\"]}", port=30001)

# ❌ WRONG - Do NOT convert to string:
filter_str = str(filter_param)
mcp__brp__world_query(data={}, filter=filter_str, port=30001)
```

**The CORRECT pattern**:
```python
# Read operation from operation_manager.py
operation = json.loads(response)
filter_obj = operation["filter"]  # This is {"with": ["Type"]}

# ✓ CORRECT - Pass the object directly:
mcp__brp__world_query(data={}, filter=filter_obj, port=30001)  # CORRECT!

# Or inline:
mcp__brp__world_query(data={}, filter={"with": ["Type"]}, port=30001)  # CORRECT!
```

**Key rule**: The operation JSON is already properly formatted. Extract values and pass them AS-IS to MCP tools. Never call `json.dumps()`, never wrap in quotes, never convert objects to strings.

**EXACT workflow for executing operations**:
1. Call: `python3 .claude/scripts/mutation_test/operation_manager.py --port PORT --action get-next`
2. The bash output contains JSON - this JSON is ALREADY CORRECTLY FORMATTED
3. Extract parameters from the JSON response and pass DIRECTLY to MCP tool
4. **DO NOT** call `json.dumps()`, `json.loads()`, `str()`, or any conversion function on the parameters
5. **DO NOT** "pretty print", "format", or "clean up" the parameters in any way
6. **DO NOT** create intermediate variables to "prepare" the parameters
7. Just call the MCP tool with the parameters EXACTLY as they appear in the JSON

**CRITICAL**: If you find yourself typing `json.dumps()`, `indent=`, or wrapping parameters in quotes, STOP. You're breaking the test.

**Complete workflow example** (this is EXACTLY how to do it):
```
Step 1: Run bash command
  → python3 .claude/scripts/mutation_test/operation_manager.py --port 30001 --action get-next

Step 2: You see this bash output:
  {
    "status": "next_operation",
    "operation": {
      "tool": "mcp__brp__world_query",
      "data": {},
      "filter": {"with": ["bevy_camera::projection::Projection"]},
      "port": 30001
    }
  }

Step 3: Extract the parameters from the JSON output (the bash tool did this automatically)
  tool = "mcp__brp__world_query"
  data = {}
  filter = {"with": ["bevy_camera::projection::Projection"]}  ← This is ALREADY a dict!
  port = 30001

Step 4: Call the MCP tool with those EXACT values:
  mcp__brp__world_query(data={}, filter={"with": ["bevy_camera::projection::Projection"]}, port=30001)

  OR use the extracted variables directly:
  mcp__brp__world_query(data=data, filter=filter, port=port)

NO OTHER STEPS. NO json.dumps(). NO formatting. Just extract and pass.
```

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

**STEP 1:** Execute <PreExecutionCheck/>
**STEP 2:** Execute <OperationLoop/>
**STEP 3:** Execute <ReportCompletion/>
</ExecutionSteps>

<PreExecutionCheck>
**BEFORE YOU START THE OPERATION LOOP, COMMIT THIS TO MEMORY:**

✅ **THE ONLY CORRECT WAY TO CALL MCP TOOLS:**
```python
# operation_manager.py gives you this JSON:
response = {"status": "next_operation", "operation": {"tool": "mcp__brp__world_query", "filter": {"with": ["Type"]}, "data": {}, "port": 30001}}

# Extract the filter object
filter_obj = response["operation"]["filter"]  # This is {"with": ["Type"]}

# Call the MCP tool with the object DIRECTLY
mcp__brp__world_query(data={}, filter=filter_obj, port=30001)  # ✅ CORRECT!
```

❌ **NEVER DO THESE (ALL CAUSE TEST FAILURES):**
```python
# ❌ NEVER use json.dumps():
filter_str = json.dumps(filter_obj)
mcp__brp__world_query(data={}, filter=filter_str, port=30001)

# ❌ NEVER use indent= (adds \n newlines):
filter_str = json.dumps(filter_obj, indent=2)
mcp__brp__world_query(data={}, filter=filter_str, port=30001)

# ❌ NEVER wrap in quotes:
mcp__brp__world_query(data={}, filter='{"with": ["Type"]}', port=30001)

# ❌ NEVER convert to string:
filter_str = str(filter_obj)
mcp__brp__world_query(data={}, filter=filter_str, port=30001)
```

**If you see `\n` newlines in your filter parameter → YOU BROKE THE TEST. STOP IMMEDIATELY.**
</PreExecutionCheck>

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
   - **VERIFICATION CHECKPOINT**: Look at each parameter value:
     - Is it an object like `{"with": ["Type"]}`? ✅ Good - use it AS-IS
     - Is it a string containing JSON with `\n` newlines? ❌ STOP - you serialized it wrong
     - Does it have escaped quotes like `\"with\"`? ❌ STOP - you serialized it wrong
   - If operation has `entity_id_substitution` field → Execute <EntityIdSubstitution/>

4. **Execute the operation**:
   - Call MCP tool from `operation.tool` with parameters DIRECTLY (no conversion)

5. **Evaluate result**:

   If operation SUCCESS:
   - **RETURN TO STEP 1** (request next operation)

   If operation FAIL:
   - Execute <MatchErrorPattern/> to determine if error is recoverable
   - If recoverable → Apply fix and retry operation (step 3), then **RETURN TO STEP 1**
   - If unrecoverable → **EXIT LOOP WITH ERROR**: "Unrecoverable error at operation ${operation_id}: ${error}"

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
**Some operations need to reference existing entities** (e.g., spawning a `Children` component that contains entity IDs).

**ONLY IF** an operation has `entity_id_substitution` field:

1. **Get an available entity using MCP tool**:
   ```bash
   mcp__brp__world_query(data={}, filter={}, port=PORT)
   ```
   - Extract first entity ID from the result array
   - Use this entity ID for all substitutions

2. **Apply substitutions**:
   - For each `path → "QUERY_ENTITY"` in `entity_id_substitution`:
     - Navigate to that path in the operation parameters
     - Replace the placeholder value with the entity ID from step 1

**Example**:
```json
Operation with entity_id_substitution:
{
  "tool": "mcp__brp__world_spawn_entity",
  "components": {"bevy_ecs::hierarchy::Children": [8589934670]},
  "entity_id_substitution": {"components.bevy_ecs::hierarchy::Children[0]": "QUERY_ENTITY"}
}

After substitution (using entity ID 4294967297):
{
  "tool": "mcp__brp__world_spawn_entity",
  "components": {"bevy_ecs::hierarchy::Children": [4294967297]}
}
```

**Note**: The `entity` field in operations is pre-resolved automatically by the hook - you don't need to do anything with it.
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

**Your responsibility**: Just execute the query operation. If it fails (status = FAIL), stop execution immediately per the normal error handling rules in <OperationExecution/>.

**Note**: You don't need to validate query results, propagate entity IDs, or look back at previous operations - the hook handles all of this automatically. Entity IDs are isolated to each test (don't cross type boundaries).
</QueryResultValidation>

## Error Pattern Matching

<MatchErrorPattern>
**When an operation fails, check the error message against these patterns IN THIS EXACT ORDER:**

Does error contain `"Unable to extract parameters"` AND `"invalid type: string"` with serialized JSON?
- ✓ YES → Execute <FilterDoubleSerializationError/> recovery
- ✗ NO → Continue

Does error contain `"invalid type: string"`?
- ✓ YES → Execute <InvalidTypeStringError/> recovery
- ✗ NO → Continue

Does error start with `"UUID parsing failed"`?
- ✓ YES → Execute <UuidParsingError/> recovery
- ✗ NO → Continue

Does error contain `"Unable to extract parameters"`?
- ✓ YES → Execute <ParameterExtractionError/> recovery
- ✗ NO → Continue

Does error contain `"invalid type: null"`?
- ✓ YES → Execute <UnitEnumVariantError/> recovery
- ✗ NO → Continue

Does error contain `"unknown variant"` with escaped quotes (like `\"VariantName\"`)?
- ✓ YES → Check the test plan JSON for the original `value` field:
  - If it was a plain string (like "None" or "Low") → Execute <UnitEnumVariantError/> recovery
  - Otherwise → Execute <EnumVariantError/> recovery
- ✗ NO → Continue

**No pattern matched:**
- No recovery available
- STOP IMMEDIATELY - do not process remaining operations
</MatchErrorPattern>

<InvalidTypeStringError>
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
5. DO NOT report as test failure - this is YOUR bug, not BRP's
6. Only fail if retry produces DIFFERENT error

**Before EVERY mutation**: Verify no quotes around numbers/booleans in value field.
</InvalidTypeStringError>

<UnitEnumVariantError>
**Pattern**: Error contains `"unknown variant"` with escaped quotes, AND test plan has plain string value

**Cause**: You're double quoting a string turning "Low" into "\"Low\"" - this is breaking things.

**Recovery**:
1. Re-read operation's `value` field from test plan JSON
2. Pass it AS-IS to MCP tool without ANY transformation
3. Re-execute operation
5. DO NOT report as test failure - this is YOUR bug

**Examples**:
- ✓ CORRECT: Pass `"None"` as string
- ✗ WRONG: Convert to `null` or add quotes
</UnitEnumVariantError>

<UuidParsingError>
**Pattern**: Error message starts with `"UUID parsing failed"`

**Full error example**:
```
UUID parsing failed: invalid character: expected an optional prefix of `urn:uuid:` followed by [0-9a-fA-F-], found `\"` at 1
```

**Cause**: You double-quoted a UUID string

**Recovery**:
1. Find UUID value in operation params
2. Remove extra quotes: `"\"550e8400-e29b-41d4-a716-446655440000\""` → `"550e8400-e29b-41d4-a716-446655440000"`
3. Re-execute operation
</UuidParsingError>

<EnumVariantError>
**Pattern**: Error contains `"unknown variant"` with escaped quotes like `\"VariantName\"`

**Cause**: You double-quoted an enum variant

**Recovery**:
1. Remove extra quotes: `"\"Low\""` → `"Low"`
2. Re-execute operation
4. DO NOT report as test failure - this is YOUR bug
</EnumVariantError>

<ParameterExtractionError>
**Pattern**: Error contains `"Unable to extract parameters"`

**Cause**: Tool framework issue with parameter order

**Recovery**:
1. Reorder parameters in your tool call (change the order you pass them)
2. Re-execute operation with reordered parameters
</ParameterExtractionError>

<FilterDoubleSerializationError>
**Pattern**: Error contains `"Unable to extract parameters"` AND mentions `"invalid type: string"` with serialized JSON content

**Key indicator**: If you see `\n` (newline characters) in the error message, you used `json.dumps(filter, indent=2)` which is FORBIDDEN.

**Example error** (your error may differ):
```
Unable to extract parameters: Invalid parameter format for 'QueryParams': invalid type: string "{\n  \"with\": [\n    \"bevy_input::gamepad::GamepadSettings\"\n  ]\n}", expected struct BrpQueryFilter
```
Notice the `\n` characters? That proves you called `json.dumps(filter, indent=2)`.

**Cause**: You're converting the parameter to a string using `json.dumps()`, `str()`, or pretty-printing with `indent=`

**What's happening**:
1. operation_manager.py gives you a dict: `{"with": ["Type"]}` ✓
2. YOU are converting it to a string (YOUR BUG):
   - `json.dumps(filter_obj)` → `"{\"with\": [\"Type\"]}"` ❌
   - `json.dumps(filter_obj, indent=2)` → `"{\n  \"with\": [...]\n}"` ❌ (notice the \n newlines!)
   - `str(filter_obj)` → `"{'with': ['Type']}"` ❌
3. You pass the STRING to the MCP tool instead of the DICT ❌

**What serde sees**:
- Expected: An object/struct (the actual `{"with": [...]}` structure)
- Got: A string literal containing JSON text

**Recovery** - FOLLOW THESE EXACT STEPS:

Step 1: Re-read the operation from the most recent `operation_manager.py` output
Step 2: Parse the JSON response to get the operation object
Step 3: Look at the `filter` field in the operation - it should already be an object like `{"with": ["Type"]}`
Step 4: Call the MCP tool with that object DIRECTLY - do not convert it to a string first

**Concrete example of correct recovery**:
```python
# The operation_manager.py gave you this JSON:
# {"status": "next_operation", "operation": {"tool": "mcp__brp__world_query", "filter": {"with": ["SomeType"]}, "data": {}, "port": 30001}}

# Step 1-3: Parse it
response_json = # ... the JSON from operation_manager.py
response = json.loads(response_json)  # Parse JSON to dict
operation = response["operation"]     # Extract operation dict
filter_param = operation["filter"]    # This is {"with": ["SomeType"]} - an object, NOT a string

# Step 4: Pass the object directly
mcp__brp__world_query(data={}, filter=filter_param, port=30001)  # CORRECT!

# DO NOT DO THIS:
filter_string = json.dumps(filter_param)  # ❌ NO! Don't convert to string!
mcp__brp__world_query(data={}, filter=filter_string, port=30001)  # ❌ WRONG!
```

**Critical**: If you converted `filter` to a string using `json.dumps()` or quotes, that's the bug. Remove that conversion step entirely.
</FilterDoubleSerializationError>
