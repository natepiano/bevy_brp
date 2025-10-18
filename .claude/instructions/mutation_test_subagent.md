# Mutation Test Subagent Instructions

**CRITICAL**: Execute the workflow defined in <SubagentExecutionFlow/>

<SubagentExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 0:** Execute <InitializeLogging/> - Initialize progress log file
    **STEP 1:** Read and internalize <ErrorRecoveryProtocol/>
    **STEP 2:** Read <SubagentContext/> to understand your assignment
    **STEP 3:** Execute <FetchAssignment/> → ON ERROR: go to Step 7 with assignment error
    **STEP 4:** Execute <ParseAssignmentData/> → ON ERROR: go to Step 7 with parsing error
    **STEP 5:** Execute <TestAllTypes/> → ON ERROR: go to Step 7 with partial results
    **STEP 6:** Execute <PreFailureCheck/> before reporting any failures
    **STEP 7:** **[UNCONDITIONAL - ALWAYS EXECUTE]** Execute <ReturnResults/> with JSON output
    **STEP 8:** **[UNCONDITIONAL - ALWAYS EXECUTE]** Execute <FinalValidation/> before sending response

**CRITICAL**: Steps 7-8 execute whether previous steps succeed or fail. Step 7 returns results (success/failure/partial).

**LOGGING REQUIREMENT**:
- Log workflow steps (0, 3, 5, 7, 8) using `.claude/scripts/mutation_test_subagent_log.sh`
- Log errors only
- This creates a diagnostic trail if subagent fails silently
</SubagentExecutionFlow>

**OUTPUT FORMAT**: Return ONLY the JSON array result from <ReturnResults/>. No explanations, no commentary.

<InitializeLogging>
**Initialize progress logging for diagnostics:**

```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} init
```

This creates `$TMPDIR/mutation_test_subagent_${PORT}_progress.log` for tracking workflow progress.

**Log at these points**:
- At workflow steps 0, 3, 5, 7, 8
- When catching errors (critical for debugging)
</InitializeLogging>

<ErrorRecoveryProtocol>
**CRITICAL - READ THIS SECTION FIRST**

IF any mutation error contains `"invalid type: string"`:
1. This means YOU stringified the value (your bug, not BRP)
2. Immediately retry with proper JSON type (unquoted)
3. Increment `retry_count`
4. Only report failure if retry also fails with DIFFERENT error

**Example (Lightmap .uv_rect.max failure from testing)**:
- Sent: `{"value": "[1.0, 2.0]"}` (string)
- Error: `invalid type: string "[1.0, 2.0]", expected a sequence of 2 f32 values`
- Fix: Retry with `{"value": [1.0, 2.0]}` (array)
- Result: If retry succeeds → mark PASSED; if retry fails with different error → mark FAILED

 **Example (Sprite .image_mode.stretch_value failure from testing)**:
  - Sent: `{"value": "3.1415927410125732"}` (string)
  - Error: `invalid type: string "3.1415927410125732", expected f32`
  - Fix: Retry with `{"value": 3.1415927410125732}` (number, no quotes)
  - Result: If retry succeeds → mark PASSED; if retry fails with different error → mark
   FAILED

**IF any mutation error contains `"UUID parsing failed"` AND `"found \`\"\` at"`**:
1. This means YOU double-quoted the UUID string (your bug, not BRP)
2. The UUID value is already a JSON string - don't stringify it further
3. Immediately retry with the UUID as a plain JSON string value
4. Increment `retry_count`
5. Only report failure if retry also fails with DIFFERENT error

**Example (UUID parsing failure from testing)**:
- Type guide shows: `"example": "550e8400-e29b-41d4-a716-446655440000"`
- Sent: `{"value": "550e8400-e29b-41d4-a716-446655440000"}` (appears correct)
- Error: `UUID parsing failed: invalid character: expected an optional prefix of 'urn:uuid:' followed by [0-9a-fA-F-], found '"' at 1`
- **Root cause**: Value was double-serialized/quoted during JSON construction
- Fix: Ensure the UUID string from type guide is used AS-IS without additional stringification
- Result: If retry succeeds → mark PASSED; if retry fails with different error → mark FAILED

**Error patterns → Fix**:
- `expected a sequence` or `expected reflected list` → Unquoted array
- `expected a boolean` → Unquoted boolean
- `expected f32/i32/u32` → Unquoted number
- `expected reflected struct` → Unquoted object

**Tracking**: Keep `retry_count` variable starting at 0, increment for each "invalid type: string" or UUID parsing error you retry. Include in final output.

**IF YOU GET STUCK IN A LOOP CALLING TOOLS WITH EMPTY PARAMETERS**:
Reorder parameters in your tool call - parameter order doesn't matter, but reordering breaks mental loops.
</ErrorRecoveryProtocol>

<ContextWindowMonitoring>
**DETECT APPROACHING CONTEXT LIMIT**:

**Automatic Check Point**: At the start of testing each type in <TestAllTypes/>
- Extract REPORTED token count from system warnings after tool calls
- **CRITICAL**: System warnings exclude MCP tools and autocompact buffer
- Reported count includes: messages, memory files, system prompt, system tools
- Hidden overhead: 73,600 tokens (MCP tool definitions ~28.6K + autocompact buffer ~45K)
- **CALCULATED total = 73,600 + reported**
- **IF CALCULATED total >= 180,000 tokens (90% of 200K)**: IMMEDIATELY bail out to Step 7
- Use the CALCULATED total for the bailout check, NOT the reported number

**Manual Check Point**: After EVERY mutation operation
- IF you sense context running low OR have difficulty continuing:
  * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Context limit approaching - returning partial results"`
  * **GO TO STEP 7 IMMEDIATELY**
  * Return results for completed types + partial result for current type
  * Use standard FAIL result format with context-limit error message

**Bailout Result Format**:
```json
{
  "type": "bevy_pbr::light::PointLight",
  "tested_type": "bevy_pbr::light::PointLight",
  "status": "FAIL",
  "entity_id": 12345,
  "retry_count": 0,
  "operations_completed": {
    "spawn_insert": true,
    "entity_query": true,
    "mutations_passed": [".intensity", ".color", ".range"],
    "total_mutations_attempted": 3
  },
  "failure_details": {
    "failed_operation": "mutation",
    "failed_mutation_path": ".shadows",
    "error_message": "Context window approaching limit - stopped testing after 3 mutations on bevy_pbr::light::PointLight"
  }
}
```

Where:
- `failed_operation`: The operation type when bailout occurred (spawn|insert|query|mutation)
- `failed_mutation_path`: The specific path being tested (or "" if not in mutation phase)
- `error_message`: Always start with "Context window approaching limit"
- **No `request_sent` or `response_received`** - there's no BRP error, just context exhaustion

**Purpose**: Ensure you reach Steps 7-8 before complete context exhaustion.
</ContextWindowMonitoring>

<EmergencyBailout>
**IF ANY BRP tool fails with connection/timeout error:**

1. **EXTRACT PORT FROM ERROR**: Parse the error message for URL pattern `http://127.0.0.1:XXXXX/jsonrpc`
   - Extract the port number (XXXXX) that was actually attempted
   - If port cannot be extracted, assume genuine app crash and skip to step 5

2. **COMPARE PORTS**:
   - Compare extracted port against your assigned ${PORT}
   - Log both: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Connection failed: attempted port [EXTRACTED], assigned port ${PORT}"`

3. **IF PORT MISMATCH DETECTED** (extracted port ≠ ${PORT}):
   - This is YOUR bug - you failed to pass `port` parameter
   - **RETRY IMMEDIATELY** with same operation parameters PLUS `port=${PORT}` explicitly
   - Increment `retry_count`
   - Log retry: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Port mismatch detected - retrying with correct port ${PORT}"`
   - **IF RETRY SUCCEEDS**: Continue testing (no failure to report)
   - **IF RETRY FAILS**: Report FAIL with both attempts in failure_details

4. **IF PORT MATCH** (extracted port == ${PORT}):
   - Genuine app crash on correct port
   - Proceed to step 5

5. **STOP TESTING**:
   - Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "BRP connection/timeout error - app crashed"`
   - **GO TO STEP 7 IMMEDIATELY** - return partial results array with FAIL status for current type
   - For any completed types: include their results with actual status (PASS/FAIL)
   - For the type being tested when error occurred: set status="FAIL", failed_operation=last operation attempted, include complete error in failure_details
   - **DO NOT** continue testing remaining types or mutations
</EmergencyBailout>

<JsonPrimitiveRules>
**CRITICAL JSON VALUE REQUIREMENTS**:

**PRIMITIVES (numbers and booleans):**
- ALL numeric values MUST be JSON numbers, NOT strings
- NEVER quote numbers: ❌ "3.1415927410125732" → ✅ 3.1415927410125732
- This includes f32, f64, u32, i32, ALL numeric types
- High-precision floats like 3.1415927410125732 are STILL JSON numbers
- ALL boolean values MUST be JSON booleans, NOT strings
- NEVER quote booleans: ❌ "true" → ✅ true, ❌ "false" → ✅ false

**ARRAYS AND LISTS:**
- ALL arrays MUST be JSON arrays, NOT strings
- NEVER quote arrays: ❌ "[1, 2, 3]" → ✅ [1, 2, 3]
- NEVER quote array syntax: ❌ "[4294967297]" → ✅ [4294967297]
- This applies to Vec, lists, and all array-like structures

**OBJECTS AND STRUCTS:**
- ALL objects MUST be JSON objects, NOT strings
- NEVER quote objects: ❌ "{\"key\": \"value\"}" → ✅ {"key": "value"}
- NEVER quote struct syntax: ❌ "{\"x\": 1.0, \"y\": 2.0}" → ✅ {"x": 1.0, "y": 2.0}
- This applies to structs, maps, and all object-like structures

**COMMON MISTAKES THAT CAUSE STRING CONVERSION**:
❌ Converting example to string: `str(example)` or `f"{example}"`
❌ String interpolation in values: treating complex types as text
❌ Copy-pasting example values as strings instead of raw values
❌ Using string formatting functions on any values
❌ JSON.stringify or similar that wraps in quotes

✅ CORRECT: Use the example value DIRECTLY from the type guide without any string conversion
✅ When constructing mutation params: assign the value AS-IS from the example
✅ Keep ALL types in their native JSON form throughout your code

**MANDATORY PRE-SEND VERIFICATION**:
Before EVERY mutation request:
1. **CHECK**: Look at the value you're about to send in `params["value"]`
2. **VERIFY TYPE**:
   - Number like `42`? → Must be NUMBER 42, not STRING "42"
   - Boolean like `true`? → Must be BOOLEAN true, not STRING "true"
   - Array like `[1, 2, 3]`? → Must be ARRAY [1, 2, 3], not STRING "[1, 2, 3]"
   - Object like `{"x": 1}`? → Must be OBJECT {"x": 1}, not STRING "{\"x\": 1}"
3. **TEST**: In your JSON structure:
   - `"value": 42` NOT `"value": "42"`
   - `"value": [1, 2]` NOT `"value": "[1, 2]"`
   - `"value": {"x": 1}` NOT `"value": "{\"x\": 1}"`
4. **CONFIRM**: No quotes around the entire value structure

**VERIFICATION EXAMPLES**:

**Primitives:**
- ❌ WRONG: `{"value": "42"}` - This is a STRING "42"
- ✅ CORRECT: `{"value": 42}` - This is a NUMBER 42
- ❌ WRONG: `{"value": "true"}` - This is a STRING "true"
- ✅ CORRECT: `{"value": true}` - This is a BOOLEAN true

**Arrays:**
- ❌ WRONG: `{"value": "[4294967297]"}` - This is a STRING "[4294967297]"
- ✅ CORRECT: `{"value": [4294967297]}` - This is an ARRAY [4294967297]
- ❌ WRONG: `{"value": "[1.0, 2.0, 3.0]"}` - This is a STRING
- ✅ CORRECT: `{"value": [1.0, 2.0, 3.0]}` - This is an ARRAY

**Objects:**
- ❌ WRONG: `{"value": "{\"x\": 1.0, \"y\": 2.0}"}` - This is a STRING
- ✅ CORRECT: `{"value": {"x": 1.0, "y": 2.0}}` - This is an OBJECT

**ERROR RECOVERY**: If error contains `"invalid type: string"`, follow <ErrorRecoveryProtocol/> immediately - retry with unquoted value before reporting failure.
</JsonPrimitiveRules>

<SubagentContext>
You are subagent with index ${SUBAGENT_INDEX} (0-based) assigned to port ${PORT}.

**YOUR ASSIGNED PORT**: ${PORT}
**YOUR BATCH**: ${BATCH_NUMBER}
**YOUR SUBAGENT INDEX**: ${SUBAGENT_INDEX} (0-based)
**MAX SUBAGENTS**: ${MAX_SUBAGENTS}
**TYPES PER SUBAGENT**: ${TYPES_PER_SUBAGENT}

**DO NOT**:
- Launch any apps (use EXISTING app on your port)
- Update JSON files
- Provide explanations or commentary
- Test any type other than those provided in your assignment data
- Make up or substitute different types
- Use your Bevy knowledge to "fix" or "improve" type names
- Test related types (like bundles when given components)
- MODIFY TYPE NAMES IN ANY WAY - use the exact strings provided
- Call `brp_all_type_guides` tool - you already have the data you need
- Use `jq` command - parse JSON directly from tool outputs

**CRITICAL CONSTRAINT**: You MUST test ONLY the exact types provided in your assignment data. The test system controls type names completely.

**Follow <TypeNameValidation/> requirements exactly** before testing each type.
</SubagentContext>

<FetchAssignment>
**Execute the assignment script to get your assigned type names**:

```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 3: Fetching assignment"
```

```bash
python3 ./.claude/scripts/mutation_test_get_subagent_assignments.py \
    --batch ${BATCH_NUMBER} \
    --max-subagents ${MAX_SUBAGENTS} \
    --types-per-subagent ${TYPES_PER_SUBAGENT} \
    --subagent-index ${SUBAGENT_INDEX}
```

**ERROR HANDLING**:
- **IF** script fails to execute or returns non-zero exit code:
  - Log the error: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Assignment fetch failed: [error message]"`
  - Go directly to Step 7 (<ReturnResults/>)
  - Return single error result: `[{"type": "ASSIGNMENT_FETCH_FAILED", "tested_type": "ASSIGNMENT_FETCH_FAILED", "status": "FAIL", "entity_id": null, "retry_count": 0, "operations_completed": {"spawn_insert": false, "entity_query": false, "mutations_passed": [], "total_mutations_attempted": 0}, "failure_details": {"failed_operation": "assignment_fetch", "error_message": "[script error message]"}}]`
- **IF** script returns invalid JSON or missing `type_names` field:
  - Log the error: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Assignment JSON invalid"`
  - Go directly to Step 7 (<ReturnResults/>)
  - Return single error result per above format with error_message="Invalid JSON in assignment response"

**SUCCESS CASE - This returns a JSON object with a `type_names` array containing ONLY the literal type name strings**:
```json
{
  "batch_number": 1,
  "subagent_index": 3,
  "subagent_number": 4,
  "port": 30004,
  "type_names": [
    "bevy_pbr::cluster::ClusterConfig",
    "bevy_input::gamepad::GamepadSettings"
  ]
}
```

**CRITICAL**:
- ❌ NEVER use `> /tmp/file.json && cat /tmp/file.json`
- ❌ NEVER redirect to a file
- ❌ NEVER create any Python script (inline or otherwise)
- ❌ NEVER pipe to `python3 -c` with parsing scripts
- ✅ Parse JSON directly from the Bash tool result stdout
- The script prints JSON to stdout - it's already in the tool result

**IMMEDIATELY AFTER FETCHING ASSIGNMENT**:
Log the assigned types for diagnostics:

```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Assigned types: [comma-separated list of type names from assignment]"
```

Example: If assigned `["bevy_input::gamepad::Gamepad", "bevy_light::AmbientLight"]`, log:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Assigned types: bevy_input::gamepad::Gamepad, bevy_light::AmbientLight"
```
</FetchAssignment>

<ParseAssignmentData>
**Log the step**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 4: Parsing assignment data"
```

**Parse the type_names array from assignment and fetch each type guide:**

The assignment gives you a simple array of type names. For EACH type name, you MUST:

1. **Extract the literal type name string** from the `type_names` array
2. **Fetch its type guide** using INDIVIDUAL bash calls (one per type)

**CRITICAL - NO LOOPS ALLOWED:**
- ❌ NEVER use bash `for` loops - they require user approval
- ❌ NEVER use `while` loops or any iteration constructs
- ✅ Make INDIVIDUAL sequential Bash tool calls, one per type
- ✅ Call `./.claude/scripts/get_type_guide.sh <TYPE_NAME> --file .claude/transient/all_types.json` for each type

**CRITICAL - TYPE NAME HANDLING:**
- The type name from `type_names` is a LITERAL STRING
- COPY it EXACTLY when calling get_type_guide.sh
- NEVER retype or reconstruct the type name from memory
- Use the SAME EXACT STRING in all BRP operations

**Example workflow for 2 types:**
```
# Assignment returned: {"type_names": ["bevy_pbr::cluster::ClusterConfig", "bevy_input::gamepad::GamepadSettings"]}

# Call 1 - Fetch first type guide:
Bash: ./.claude/scripts/get_type_guide.sh bevy_pbr::cluster::ClusterConfig --file .claude/transient/all_types.json

# Call 2 - Fetch second type guide:
Bash: ./.claude/scripts/get_type_guide.sh bevy_input::gamepad::GamepadSettings --file .claude/transient/all_types.json
```

**VALIDATION**: You should make exactly as many Bash calls as there are entries in `type_names` array

**The get_type_guide.sh script returns:**
```json
{
  "status": "found",
  "type_name": "bevy_pbr::cluster::ClusterConfig",
  "guide": {
    "spawn_format": null,
    "mutation_paths": {...},
    "supported_operations": ["query", "get", "mutate"],
    ...
  }
}
```

**FIELD USAGE - How to use the type guide:**

- **`type_name`** (from script output): The AUTHORITATIVE type identifier
  - Use EXACTLY as-is in all BRP tool calls
  - This MUST match the string from your assignment

- **`mutation_type`** (from assignment data): Type category - "Component" or "Resource"
  - Use this to determine which testing protocol to follow
  - "Component" → Use <ComponentTestingProtocol/>
  - "Resource" → Use <ResourceTestingProtocol/>

- **`guide.spawn_format`**: Example value for entity creation (may be `null`)
  - If not `null` AND "spawn" in `supported_operations`: Use in spawn/insert
  - If `null` OR "spawn" not supported: Skip spawn/insert testing

- **`guide.mutation_paths`**: Dictionary of testable mutation paths
  - Keys are path strings (e.g., `""`, `".field"`, `".nested.value"`)
  - Each path has an `example` value to use in mutations
  - Check `path_info.mutability` before testing (skip if `"not_mutable"`)

- **`guide.supported_operations`**: Which BRP methods work with this type
  - Check before calling: If "spawn" not in list, don't call world_spawn_entity
  - If "mutate" not in list, don't call world_mutate_components
</ParseAssignmentData>

<TestAllTypes>
**Log the step**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 5: Testing all assigned types"
```

**Testing Protocol**:

For each type name string in your `type_names` array:
   1. **FETCH TYPE GUIDE**: Call `get_type_guide.sh <type_name> --file .claude/transient/all_types.json`
   2. **EXTRACT TYPE NAME**: Get the `type_name` field from the script output - this is your AUTHORITATIVE string
   3. **CALCULATE AND LOG TOKEN USAGE** (CRITICAL - Read carefully):
      - After the tool call, check for `<system_warning>` in the response
      - Extract the REPORTED token count from pattern: `Token usage: X/200,000`
      - **CRITICAL**: The reported count X is INCOMPLETE - it excludes MCP tools and autocompact buffer
      - Hidden overhead = 73,600 tokens (MCP tool definitions ~28.6K + autocompact buffer ~45K)
      - **CALCULATED total usage = 73,600 + X** (THIS is your actual total)
      - Percentage = (CALCULATED total / 200,000) * 100
      - Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Testing type: [type_name] - usage (hidden: 73.6K, reported: XK, total: YK, Z%)"`
      - **BAILOUT CHECK**: Compare CALCULATED total (not reported X) against threshold:
        * **IF CALCULATED total >= 180,000 tokens (90% of 200K)**:
          - Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Context limit reached - calculated total YK >= 180K (Z%) - returning partial results"`
          - **GO TO STEP 7 IMMEDIATELY**
          - Return results for all completed types (with their actual PASS/FAIL status)
          - Do NOT start testing this type
   4. **TEST THE TYPE**:

   **UNDERSTANDING spawn_format NULL**:
   - `spawn_format: null` means root path is `partially_mutable`
   - This is NOT an error or skip condition
   - Mutation paths ARE still testable
   - Components: Query for existing entities instead of spawning
   - Resources: Skip insert, mutate existing resource directly

   a. **COMPONENT_NOT_FOUND VALIDATION**:
      - **IF** entity query returns 0 entities for a type:
        1. **STOP IMMEDIATELY** - do NOT report COMPONENT_NOT_FOUND yet
        2. **RE-FETCH** your assignment using the assignment script again
        3. **COMPARE** the type name you tested against the assignment's `type_names` array
        4. **VERIFY** you used the EXACT string from the array (character-by-character match)
        5. **IF MISMATCH DETECTED**:
           - ERROR: You modified the type name - this is a CRITICAL BUG
           - In your result JSON:
             * Set `type` to the correct type_name from assignment
             * Set `tested_type` to the wrong type you actually used
             * This will expose the hallucination to the main agent
        6. **ONLY IF EXACT MATCH**:
           - Report status as COMPONENT_NOT_FOUND
           - Set both `type` and `tested_type` to the assignment's type_name
   b. **TYPE CATEGORY DETERMINATION - CRITICAL DECISION POINT**:
      - **CHECK `mutation_type` field from assignment data**
      - **RESOURCE DETECTION**:
        * IF `mutation_type == "Resource"` → **THIS IS A RESOURCE**
        * Execute <ResourceTestingProtocol/> ONLY
        * **SKIP** all component testing steps completely
        * **NEVER** use `world_spawn_entity` or `world_mutate_components` for resources
      - **COMPONENT DETECTION**:
        * IF `mutation_type == "Component"` → **THIS IS A COMPONENT**
        * Execute <ComponentTestingProtocol/> ONLY
        * **SKIP** all resource testing steps completely
        * **NEVER** use `world_insert_resources` or `world_mutate_resources` for components
      - **ERROR CASE**: If `mutation_type` is neither "Resource" nor "Component", report error in failure_details

   c. **ROOT EXAMPLE SETUP FOR VARIANT-DEPENDENT PATHS**:
      - **BEFORE testing each mutation path**, check if `path_info.root_example` exists
      - **IF `root_example` EXISTS**:
        1. **First** mutate the root path (`""`) to set up the correct variant structure:
           - **FOR COMPONENTS**: Use `world_mutate_components` with entity ID
           - **FOR RESOURCES**: Use `world_mutate_resources` (no entity ID)
           - Set `path: ""`
           - Set `value` to the `root_example` value from `path_info`
        2. **Then** proceed with mutating the specific path
      - **IF `root_example` DOES NOT EXIST**: Proceed directly with mutating the path
      - **PURPOSE**: Ensures enum variants are correctly set before accessing variant-specific fields
      - **EXAMPLE**: For `.middle_struct.nested_enum.name` with applicable_variants `["BottomEnum::VariantB"]` and this `root_example`:
        ```json
        {
          "WithMiddleStruct": {
            "middle_struct": {
              "nested_enum": {"VariantB": {"name": "Hello, World!", "value": 3.14}},
              "some_field": "Hello, World!",
              "some_value": 3.14
            }
          }
        }
        ```
        First mutate path `""` with this complete value, THEN mutate `.middle_struct.nested_enum.name`
   f. **MUTATION TESTING**: For each path in mutation_paths, validate THEN test
      - **FOR EACH path in mutation_paths object:**
        1. **CHECK mutability FIRST**: If `path_info.mutability == "not_mutable"` → SKIP path (don't count in total)
        2. **CHECK for example**: If no `example` or `examples` field exists → SKIP path (cannot test without value)
        3. **IF partially_mutable**: SKIP unless `example` or `examples` exists
        4. **ONLY if checks pass**: Proceed to mutation
      - **CHOOSE MUTATION METHOD** based on type category:
        * **FOR COMPONENTS**: Use `world_mutate_components` with entity ID
        * **FOR RESOURCES**: Use `world_mutate_resources` (no entity ID needed)
      - Apply Entity ID substitution BEFORE sending any mutation request (components only)
      - If a mutation uses Entity IDs and you don't have real ones, query for them first
      - **CRITICAL VALUE HANDLING**: Extract the `example` value from mutation_paths and follow <JsonPrimitiveRules/> when using it
      - **ON FAILURE ONLY**: Log error with `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Mutation [path] failed: [error]"`
      - **ENUM TESTING REQUIREMENT**: When a mutation path contains an "examples" array (indicating enum variants), you MUST test each example individually:
        * For each entry in the "examples" array, perform a separate mutation using that specific "example" value
        * Example: If `.depth_load_op` has examples `[{"example": {"Clear": 3.14}}, {"example": "Load"}]`, test BOTH:
          1. Mutate `.depth_load_op` with `{"Clear": 3.14}`
          2. Mutate `.depth_load_op` with `"Load"`
        * Count each example test as a separate mutation attempt in your totals
      - **IMPORTANT**: Only count actually attempted mutations in `total_mutations_attempted`
3. **CAPTURE ALL ERROR DETAILS**: When ANY operation fails, record the COMPLETE request and response
4. NEVER test types not provided in your assignment data

**AFTER EACH MUTATION**:
- If error contains "invalid type: string" or UUID parsing error, follow <ErrorRecoveryProtocol/> immediately.
- Check <ContextWindowMonitoring/> - if context limit approaching, bail out to Step 7 immediately.

**AFTER COMPLETING TESTING FOR CURRENT TYPE**:
- Add the result (PASS/FAIL/COMPONENT_NOT_FOUND) to your results collection
- **IF there are more types remaining in your `type_names` array**: Return to step 1 above and test the next type
- **IF there are no more types to test**: You have completed Step 5. Proceed to Step 6 (<PreFailureCheck/>)
</TestAllTypes>

<ResourceTestingProtocol>
**RESOURCE TESTING PROTOCOL - Use ONLY for types with "Resource" in schema_info.reflect_types**

**CRITICAL**: Do NOT use component methods (`world_spawn_entity`, `world_mutate_components`) - these will CRASH the app

1. **INSERT CHECK**: If `guide.spawn_format` is NOT null:
   - **FIRST**: Check spawn_format for Entity ID placeholders and apply <EntityIdSubstitution/> if needed
   - Use `world_insert_resources` tool
   - Pass `resource` parameter with exact type name from type guide
   - Pass `value` parameter with VALIDATED spawn_format data (after Entity ID substitution)
   - Then verify insertion with `world_get_resources`
   - **ON FAILURE**: Log error with `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Resource insert failed: [error]"`
   - Set `spawn_insert: true` in operations_completed

2. **IF spawn_format IS null**:
   - Skip insert step (root is `partially_mutable`)
   - Resource must already exist in the running app
   - Set `spawn_insert: false` in operations_completed
   - Proceed directly to mutation testing

3. **MUTATION TESTING** (ALWAYS execute if mutation paths exist):
   - **BEFORE each mutation**: Check mutation example for Entity ID placeholders and apply <EntityIdSubstitution/> if needed
   - Use `world_mutate_resources` tool (NOT `world_mutate_components`)
   - Pass `resource` parameter with exact type name
   - Pass `path` parameter with mutation path from type guide
   - Pass `value` parameter with VALIDATED example from type guide (after Entity ID substitution)
   - Follow <JsonPrimitiveRules/> for value formatting
   - **NO entity ID parameter** - resources don't have entities

4. **ENTITY QUERY**:
   - Resources are NOT attached to entities
   - Do NOT query for entities with `world_query`
   - Set `entity_query: false` in operations_completed
   - Set `entity_id: null` in result
</ResourceTestingProtocol>

<ComponentTestingProtocol>
**COMPONENT TESTING PROTOCOL - Use ONLY for types with "Component" in schema_info.reflect_types**

**CRITICAL**: Do NOT use resource methods (`world_insert_resources`, `world_mutate_resources`)

1. **SPAWN/INSERT CHECK**: If `guide.spawn_format` is NOT null:
   - **FIRST**: Check spawn_format for Entity ID placeholders and apply <EntityIdSubstitution/> if needed
   - **THEN**: Use `world_spawn_entity` tool
   - Pass `components` parameter with type name as key and VALIDATED spawn_format as value
   - **ON FAILURE**: Log error with `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Spawn failed: [error]"`
   - Set `spawn_insert: true` in operations_completed
   - Proceed to query for entity

2. **IF spawn_format IS null**:
   - Skip spawn step (root is `partially_mutable`)
   - Component must already exist on entities in the running app
   - Set `spawn_insert: false` in operations_completed
   - Proceed to query for EXISTING entities with this component

3. **QUERY FOR ENTITY** (ALWAYS execute):
   - Use `world_query` tool
   - Pass `filter: {"with": ["EXACT_TYPE_NAME"]}` to find entities
   - Pass `data: {}` to get entity IDs only
   - Store entity ID for mutation testing
   - If 0 entities found → Report COMPONENT_NOT_FOUND status
   - If query fails with error → Follow <EmergencyBailout> and return FAIL with error details
   - Set `entity_query: true` in operations_completed

4. **MUTATION TESTING** (ALWAYS execute if entity found and mutation paths exist):
   - **BEFORE each mutation**: Check mutation example for Entity ID placeholders and apply <EntityIdSubstitution/> if needed
   - Use `world_mutate_components` tool (NOT `world_mutate_resources`)
   - Pass `entity` parameter with entity ID from query
   - Pass `component` parameter with exact type name
   - Pass `path` parameter with mutation path from type guide
   - Pass `value` parameter with VALIDATED example from type guide (after Entity ID substitution)
   - Follow <JsonPrimitiveRules/> for value formatting
</ComponentTestingProtocol>

<PreFailureCheck>
**BEFORE REPORTING ANY FAILURE**:

1. Count "invalid type: string" and UUID parsing errors received: _____
2. Does this match your `retry_count`? (must be equal)
3. Are you reporting ANY failures that had "invalid type: string" or UUID parsing errors?
   - If YES → Protocol violation - those must be retried first per <ErrorRecoveryProtocol/>

If any check fails, go back and follow <ErrorRecoveryProtocol/>.
</PreFailureCheck>

<SubagentOutputFormat>
**Return EXACTLY this format (nothing else)**:
```json
[{
  "type": "[type_name from assignment script - REQUIRED]",
  "tested_type": "[actual type used in queries - MUST match 'type']",
  "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
  "entity_id": 123,  // Entity ID if created, null otherwise
  "retry_count": 0,  // How many "invalid type: string" errors you retried
  "operations_completed": {
    "spawn_insert": true|false,  // Did spawn/insert succeed?
    "entity_query": true|false,   // Did query find entity?
    "mutations_passed": [".path1", ".path2"],  // Which mutations succeeded
    "total_mutations_attempted": 5  // How many mutation paths were tested
  },
  "failure_details": {
    // ONLY PRESENT IF status is FAIL or COMPONENT_NOT_FOUND
    "failed_operation": "spawn|insert|query|mutation",
    "failed_mutation_path": ".specific.path.that.failed",
    "error_message": "Complete error message from BRP",
    "request_sent": {
      // EXACT parameters sent that caused the failure
      "method": "world.mutate_components",
      "params": {
        "entity": 123,
        "component": "full::type::name",
        "path": ".failed.path",
        "value": 3.14159  // ⚠️ MUST be JSON primitive (number/boolean), NOT string
      }
    },
    "response_received": {
      // COMPLETE response from BRP including error details
      "error": "Full error response",
      "code": -32000,
      "data": "any additional error data"
    }
  },
  "query_details": {
    // ONLY PRESENT IF status is COMPONENT_NOT_FOUND
    "filter": {"with": ["exact::type::used"]},
    "data": {"components": []},
    "entities_found": 0
  }
}]
```
</SubagentOutputFormat>

<ReturnResults>
**CRITICAL - Log reaching Step 7**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 7: Returning results"
```

**CRITICAL FIELD REQUIREMENTS**:
- `type`: Extract from the `type_name` field returned by `get_type_guide.sh` - this is the AUTHORITATIVE type name
- `tested_type`: The exact type name string you passed to BRP queries - MUST be identical to `type`
- `retry_count`: Number of "invalid type: string" errors you retried (required for validation)
- Purpose: Detects if you hallucinated or modified a type name (CRITICAL BUG if they differ)
- **BOTH MUST MATCH**: The string from assignment's `type_names` array = type guide's `type_name` = what you used in BRP calls

**IF YOU CANNOT COMPLETE TESTING** (app crash, connection lost, assignment failure):
- Return partial results for types tested so far
- Mark incomplete type as FAIL with error details
- DO NOT return empty/null - ALWAYS return valid JSON array
- **IF assignment fetch failed**: Return single error result with type="ASSIGNMENT_FETCH_FAILED" and failure_details
- **IF no types were assigned**: Return empty array `[]` only if assignment explicitly returned zero types
- **IF testing crashed mid-batch**: Return results for completed types + FAIL result for type being tested when crash occurred

<SubagentOutputFormat/>

**PRE-OUTPUT VALIDATION**: Before generating your final JSON, follow <JsonPrimitiveRules/> and pay special attention to `"value"` fields in failure_details.
</ReturnResults>

<FinalValidation>
**Log validation step**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 8: Final validation before output"
```

**VERIFY BEFORE OUTPUT**:
- `retry_count` matches number of "invalid type: string" and UUID parsing errors received
- No failures reported with "invalid type: string" or UUID parsing errors in error_message
- All failure values are proper JSON types (not strings)

If any fail: Review <ErrorRecoveryProtocol/> and fix before output.

**Log completion**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Subagent workflow complete - returning JSON"
```
</FinalValidation>

<TypeNameValidation>
═══════════════════════════════════════════════════════════════════════════════
🚨 TYPE NAME VALIDATION - CRITICAL REQUIREMENTS 🚨
═══════════════════════════════════════════════════════════════════════════════

**CRITICAL TYPE NAME REQUIREMENTS**:
- **NEVER modify type names** - use EXACT strings from assignment data
- **NEVER substitute** types based on Bevy knowledge or assumptions
- **NEVER "fix"** type paths you think are incorrect
- **FAIL IMMEDIATELY** if you detect yourself modifying any type name

**VALIDATION EXAMPLES**:
```
Assignment script says: "bevy_core_pipeline::tonemapping::ColorGrading"
✅ CORRECT: Use exactly "bevy_core_pipeline::tonemapping::ColorGrading"
❌ WRONG: Change to "bevy_render::view::ColorGrading" because you "know better"

Assignment script says: "bevy_ecs::hierarchy::Children"
✅ CORRECT: Use exactly "bevy_ecs::hierarchy::Children"
❌ WRONG: Change to "bevy_hierarchy::components::children::Children"
```

**ENFORCEMENT**: The test system controls type names completely. Use the EXACT `type_name` field from assignment data without any modifications.

═══════════════════════════════════════════════════════════════════════════════
</TypeNameValidation>

<EntityIdSubstitution>
**ENTITY ID SUBSTITUTION - For types with Entity fields**:

- **CRITICAL**: If any spawn_format or mutation example contains the value `8589934670`, this is a PLACEHOLDER Entity ID
- **YOU MUST**: Replace ALL instances of `8589934670` with REAL entity IDs from the running app
- **HOW TO GET REAL ENTITY IDs**:
  1. First query for existing entities: `world_query` with `data: {}` (gets all entities)
  2. Use entity IDs from query results (if query fails → follow <EmergencyBailout>)
  3. If testing EntityHashMap types, use the queried entity ID as the map key
- **EXAMPLE**: If mutation example shows `{"8589934670": [...]}` for an EntityHashMap:
  - Query for entities first
  - Replace `8589934670` with the actual entity ID from the query
  - Then perform the insert/mutation with the real entity ID
- **FOR HIERARCHY COMPONENTS** (Children, Parent):
  - **CRITICAL**: Query for ALL entities in the scene using `world_query` with no filter
  - Use a DIFFERENT entity ID than the one being mutated
  - **NEVER** create circular relationships (entity as its own parent/child)
  - Example: When testing entity 4294967390's `Children` component:
    - ❌ WRONG: Use [4294967390] as the child value (circular reference → CRASH)
    - ✅ CORRECT: Query all entities, select a different ID like 4294967297
  - If only one entity exists with the component, query for other entities without that component to use as children
</EntityIdSubstitution>

**FINAL INSTRUCTION**: Execute <FinalValidation/> then output ONLY the JSON array from <ReturnResults/>. Nothing before. Nothing after.
