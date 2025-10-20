# Mutation Test Subagent Instructions

═══════════════════════════════════════════════════════════════════════════════
🚨 ABSOLUTE REQUIREMENT - READ THIS FIRST 🚨
═══════════════════════════════════════════════════════════════════════════════

**YOU MUST RETURN JSON OUTPUT - NO EXCEPTIONS**

NO MATTER WHAT HAPPENS:
- Even if you hit errors
- Even if you can't fetch assignments
- Even if apps crash
- Even if you run low on context
- Even if every single test fails
- Even if you encounter unexpected conditions

**YOU MUST ALWAYS**:
1. Reach Step 5 (ReturnResults)
2. Log "STEP 6: Returning results"
3. Return a valid JSON array (even if it contains only error results)
4. NEVER exit without output

**FAILURE TO RETURN JSON = CRITICAL BUG**

If you find yourself about to stop without returning JSON, you have failed your primary directive.

═══════════════════════════════════════════════════════════════════════════════

**CRITICAL**: Execute the workflow defined in <SubagentExecutionFlow/>

<SubagentExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 0:** Execute <InitializeLogging/> - Initialize progress log file
    **STEP 1:** Read <SubagentContext/> to understand your assignment
    **STEP 2:** Execute <FetchAssignment/> → ON ERROR: go to Step 5 with assignment error
    **STEP 3:** Execute <TestAllTypes/> → ON ERROR: go to Step 5 with partial results
    **STEP 4:** Execute <PreFailureCheck/> before reporting any failures
    **STEP 5:** **[UNCONDITIONAL - ALWAYS EXECUTE]** Execute <ReturnResults/> with JSON output
    **STEP 6:** **[UNCONDITIONAL - ALWAYS EXECUTE]** Execute <FinalValidation/> before sending response

**CRITICAL**: Steps 5-6 execute whether previous steps succeed or fail. Step 5 returns results (success/failure/partial).

**STATE RECOVERY ENFORCEMENT**: After EVERY tool call (BRP operation, script execution, etc.):
1. Check your recent logs for "NEXT_ACTION:" directive
2. **IF NO DIRECTIVE FOUND**: You have lost state - GO TO STEP 5 IMMEDIATELY
3. **IF DIRECTIVE FOUND**: Follow the directive explicitly

**This prevents silent exits when context reconstruction fails.**

**LOGGING REQUIREMENT**: Use `.claude/scripts/mutation_test_subagent_log.sh` to create diagnostic trail (see <InitializeLogging/> for details)
</SubagentExecutionFlow>

**OUTPUT FORMAT**: Return ONLY the JSON array result from <ReturnResults/>. No explanations, no commentary.

<InitializeLogging>
**Initialize progress logging for diagnostics:**

```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} init
```

This creates `$TMPDIR/mutation_test_subagent_${PORT}_progress.log` for tracking workflow progress.

**Log at these points**:
- At workflow steps 0, 2, 3, 5, 6
- Before/after spawn/insert operations
- Before/after query operations
- Before testing each mutation path
- When catching errors (critical for debugging)
</InitializeLogging>

<ContextWindowMonitoring>
**DETECT APPROACHING CONTEXT LIMIT**:

**Automatic Check Point**: At the start of testing each type in <TestAllTypes/>
- Extract REPORTED token count from system warnings after tool calls
- **CRITICAL**: System warnings exclude MCP tools and autocompact buffer
- Reported count includes: messages, memory files, system prompt, system tools
- Hidden overhead: 73,600 tokens (MCP tool definitions ~28.6K + autocompact buffer ~45K)
- **CALCULATED total = 73,600 + reported**
- **IF CALCULATED total >= 180,000 tokens (90% of 200K)**: IMMEDIATELY bail out to Step 6
- Use the CALCULATED total for the bailout check, NOT the reported number

**Manual Check Point**: After EVERY mutation operation
- IF you sense context running low OR have difficulty continuing:
  * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Context limit approaching - returning partial results"`
  * **GO TO STEP 6 IMMEDIATELY**
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
   - **GO TO STEP 6 IMMEDIATELY** - return partial results array with FAIL status for current type
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
- Call `brp_all_type_guides` tool - you already have the data you need
- Use `jq` command - parse JSON directly from tool outputs

**CRITICAL**: Follow <TypeNameValidation/> requirements exactly - NEVER modify type names in any way.
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
  - Go directly to Step 6 (<ReturnResults/>)
  - Return single error result: `[{"type": "ASSIGNMENT_FETCH_FAILED", "tested_type": "ASSIGNMENT_FETCH_FAILED", "status": "FAIL", "entity_id": null, "retry_count": 0, "operations_completed": {"spawn_insert": false, "entity_query": false, "mutations_passed": [], "total_mutations_attempted": 0}, "failure_details": {"failed_operation": "assignment_fetch", "error_message": "[script error message]"}}]`
- **IF** script returns invalid JSON or missing `type_names` field:
  - Log the error: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Assignment JSON invalid"`
  - Go directly to Step 6 (<ReturnResults/>)
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

<TestAllTypes>
**Log the step**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 4: Testing all assigned types"
```

**CRITICAL - NO LOOPS ALLOWED:**
- ❌ NEVER use bash `for` loops - they require user approval
- ❌ NEVER use `while` loops or any iteration constructs
- ✅ Make INDIVIDUAL sequential tool calls, one per type
- ✅ Follow <TypeNameValidation/> - use EXACT strings from `type_names` array

**Testing Protocol** - For each type name string in your `type_names` array:

   1. **LOG TYPE START**:
      ```bash
      .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "TYPE_START: Testing type [index]/[total] - [type_name]"
      ```

   2. **FETCH TYPE GUIDE**: Call `get_type_guide.sh <type_name> --file .claude/transient/all_types.json`
      - Returns: `{"status": "found", "type_name": "...", "guide": {...}}`

   3. **EXTRACT TYPE NAME**: Get the `type_name` field from script output - this is your AUTHORITATIVE string

   4. **TOKEN CHECKPOINT**: Execute <CalculateAndCheckTokens context="Testing type: [type_name]"/>

   5. **LOG NEXT ACTION**:
      ```bash
      # Determine if this is the last type
      IF this is the last type in type_names array:
        .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "NEXT_ACTION: complete_all_types_after_testing"
      ELSE:
        .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "NEXT_ACTION: test_next_type_after_completion"
      ```

   6. **TEST THE TYPE**:

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

3. **CAPTURE ALL ERROR DETAILS**: When ANY operation fails, record the COMPLETE request and response
4. NEVER test types not provided in your assignment data

**AFTER COMPLETING TESTING FOR CURRENT TYPE**:
1. **Log type completion**:
   ```bash
   .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "TYPE_COMPLETE: Finished testing [type_name] with status [PASS/FAIL/COMPONENT_NOT_FOUND]"
   ```

2. Add the result (PASS/FAIL/COMPONENT_NOT_FOUND) to your results collection

3. **Check NEXT_ACTION from step 5 above**:
   - **IF last TYPE_START log said "NEXT_ACTION: test_next_type_after_completion"**:
     * Return to step 1 above and test the next type
   - **IF last TYPE_START log said "NEXT_ACTION: complete_all_types_after_testing"**:
     * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "ALL_TYPES_COMPLETE: Tested [count] types, proceeding to Step 5"`
     * You have completed Step 4. Proceed to Step 5 (<PreFailureCheck/>)
   - **ELSE (no clear directive)**:
     * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "STATE_LOST: Cannot determine if more types remain - proceeding to Step 5"`
     * Proceed to Step 5 with results collected so far
</TestAllTypes>

<ResourceTestingProtocol>
**RESOURCE TESTING PROTOCOL - Use ONLY for types with "Resource" in schema_info.reflect_types**

**CRITICAL**: Do NOT use component methods (`world_spawn_entity`, `world_mutate_components`) - these will CRASH the app

1. **INSERT CHECK**: If `guide.spawn_format` is NOT null:
   - **FIRST**: Check spawn_format for Entity ID placeholders and apply <EntityIdSubstitution/> if needed
   - **LOG**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Inserting resource"`
   - Use `world_insert_resources` tool
   - Pass `resource` parameter with exact type name from type guide
   - Pass `value` parameter with VALIDATED spawn_format data (after Entity ID substitution)
   - Then verify insertion with `world_get_resources`
   - **ON FAILURE**: Apply <LogOperationFailure operation="Resource insert"/>
   - **ON SUCCESS**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Resource insert successful"`
   - Apply <MarkSpawnInsertSuccess/>

2. **IF spawn_format IS null**: Apply <NullSpawnFormatHandling/>

3. **MUTATION TESTING** (ALWAYS execute if mutation paths exist):
   - Execute <MutationTestingLoop/> using `world_mutate_resources` tool
   - **NO entity ID parameter** - resources are global, not entity-attached

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
   - **LOG**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Spawning entity with component"`
   - **THEN**: Use `world_spawn_entity` tool
   - Pass `components` parameter with type name as key and VALIDATED spawn_format as value
   - **ON FAILURE**: Apply <LogOperationFailure operation="Spawn"/>
   - **ON SUCCESS**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Spawn successful - entity ID: [id]"`
   - Apply <MarkSpawnInsertSuccess/>
   - Proceed to query for entity

2. **IF spawn_format IS null**: Apply <NullSpawnFormatHandling/>

3. **QUERY FOR ENTITY** (ALWAYS execute):
   - **LOG**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Querying for entities with component"`
   - Use `world_query` tool
   - Pass `filter: {"with": ["EXACT_TYPE_NAME"]}` to find entities
   - Pass `data: {}` to get entity IDs only
   - Store entity ID for mutation testing
   - **ON SUCCESS**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "Query found [count] entities"`
   - If 0 entities found → Report COMPONENT_NOT_FOUND status
   - If query fails with error → Follow <EmergencyBailout> and return FAIL with error details
   - Set `entity_query: true` in operations_completed

4. **MUTATION TESTING** (ALWAYS execute if entity found and mutation paths exist):
   - Execute <MutationTestingLoop/> using `world_mutate_components` tool
   - **REQUIRED**: Pass `entity` parameter with entity ID from query
   - Pass `component` parameter with exact type name
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
**CRITICAL - Log reaching Step 6**:
```bash
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 6: Returning results"
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
.claude/scripts/mutation_test_subagent_log.sh ${PORT} step "STEP 7: Final validation before output"
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

<NullSpawnFormatHandling>
- Skip spawn/insert step
- Set `spawn_insert: false`
- **Components**:
  - **LOG**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "spawn_format is null - querying for existing entities"`
  - Query for EXISTING entities
- **Resources**:
  - **LOG**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "spawn_format is null - proceeding to mutation testing"`
  - Proceed to mutation testing
</NullSpawnFormatHandling>

<LogOperationFailure>
`.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "[operation] failed: [error]"`
</LogOperationFailure>

<LogMutationAttempt>
Log the mutation path AND the full arguments being passed to the tool:

For Components:
`.claude/scripts/mutation_test_subagent_log.sh ${PORT} tool "world_mutate_components(entity=[entity_id], component=\"[component_type]\", path=\"[path]\", value=[json_value])"`

For Resources:
`.claude/scripts/mutation_test_subagent_log.sh ${PORT} tool "world_mutate_resources(resource=\"[resource_type]\", path=\"[path]\", value=[json_value])"`

Replace placeholders with actual values. Include the full JSON value being passed.
</LogMutationAttempt>

<MarkSpawnInsertSuccess>
Set `spawn_insert: true` in operations_completed
</MarkSpawnInsertSuccess>

<MutationTestingLoop>
**CRITICAL STATE MANAGEMENT**: This loop must be stateless - every decision determinable from logs alone.

**BEFORE ENTERING MUTATION LOOP**:
1. Execute <CalculateAndCheckTokens context="Pre-mutation token check"/>
2. Build complete mutation queue from type guide's mutation_paths
3. Log loop initialization:
   ```bash
   .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "LOOP_INIT: Starting mutations for [type_name], total_paths=[count]"
   ```
4. Set tracking variables: `mutation_index = 0`, `last_root_mutated = null`

**STATE RECOVERY PROTOCOL** (execute at start of EVERY iteration):
```bash
# Check for NEXT_ACTION directive in recent logs
IF no "NEXT_ACTION:" found in last 5 log entries:
  - Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "STATE_LOST: Cannot determine next action - returning partial results"`
  - GO TO STEP 5 IMMEDIATELY with partial results for current type
```

**FOR EACH path in mutation_paths**:

**STEP A - PRE-MUTATION STATE LOGGING**:
1. Check mutability: If `path_info.mutability == "not_mutable"` → SKIP to next path
2. Check example: If no `example` or `examples` → SKIP to next path
3. Log current state:
   ```bash
   .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "MUTATION_START: path=[index]/[total] name=[path] type=[type_name]"
   ```
4. Determine if this is last mutation:
   ```bash
   IF mutation_index == (total_paths - 1):
     .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "NEXT_ACTION: complete_type_after_mutation"
   ELSE:
     .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "NEXT_ACTION: test_next_mutation_path"
   ```

**STEP B - ROOT MUTATION HANDLING** (if needed):
- If `path_info.root_example` exists AND differs from `last_root_mutated`:
  1. Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "ROOT_MUTATION: Setting root to enable nested path testing"`
  2. Mutate root path (`""`) with `root_example`
  3. Set `last_root_mutated = path_info.root_example`
  4. Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "NEXT_ACTION: test_nested_path_after_root"`
  5. Then proceed to mutate the specific nested path
- Else: mutate path directly (no root mutation needed)

**STEP C - EXECUTE MUTATION**:
- Apply <EntityIdSubstitution/> if type has Entity fields
- Apply <LogMutationAttempt/>
- Components: `world_mutate_components` with entity, component, path, value
- Resources: `world_mutate_resources` with resource, path, value
- Follow <JsonPrimitiveRules/>

**STEP D - POST-MUTATION STATE LOGGING**:
1. **Log the result**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} result "Mutation result: status=[success/error], path=[path]"`

2. **IF status == "success"**:
   - Track in `mutations_passed`
   - Increment `total_mutations_attempted`
   - Increment `mutation_index`
   - Log completion:
     ```bash
     .claude/scripts/mutation_test_subagent_log.sh ${PORT} log "MUTATION_COMPLETE: [path] succeeded, tested=[mutation_index]/[total]"
     ```
   - **CRITICAL - Check NEXT_ACTION from Step A**:
     * IF last log before tool call said "NEXT_ACTION: complete_type_after_mutation":
       - Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "TYPE_COMPLETE: All mutations passed for [type_name]"`
       - EXIT mutation loop
       - Return to <TestAllTypes/> to process next type
     * IF last log before tool call said "NEXT_ACTION: test_next_mutation_path":
       - Continue to next iteration of mutation loop
     * IF last log before tool call said "NEXT_ACTION: test_nested_path_after_root":
       - Continue to test the actual nested path
     * ELSE (no clear directive):
       - Apply <StateRecoveryProtocol/>

3. **IF status == "error"**:
   - Increment `total_mutations_attempted`
   - **MATCH error message and dispatch recovery:**
     * IF contains `"Unable to extract parameters"` → Apply <ParameterExtractionRecovery/>
     * IF contains `"invalid type: string"` → Apply <InvalidTypeStringRecovery/>
     * IF contains `"UUID parsing failed"` AND `"found \`\"\` at"` → Apply <UuidParsingRecovery/>
     * ELSE → Apply <LogOperationFailure operation="Mutation [path]"/>, mark as failed, increment mutation_index, continue to next path
   - After error handling, check NEXT_ACTION same as success case above

**ENUM VARIANTS HANDLING**:
- If path has `examples` array (enum with multiple variants):
  1. Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "ENUM_VARIANTS: Testing [count] variants for [path]"`
  2. For each variant in examples array:
     - Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "ENUM_VARIANT: [variant_index]/[total_variants] for [path]"`
     - Test variant separately
     - Count as separate mutation in totals
  3. After all variants tested, continue to next path

**CONTEXT LIMIT CHECK**:
- After EVERY mutation, execute <CalculateAndCheckTokens context="Post-mutation [path]"/>
- If bailout triggered: LOG and GO TO STEP 5 with partial results
</MutationTestingLoop>

<StateRecoveryProtocol>
**When NEXT_ACTION cannot be determined after successful mutation:**

1. Log the problem:
   ```bash
   .claude/scripts/mutation_test_subagent_log.sh ${PORT} error "STATE_AMBIGUOUS: Cannot determine next action after [path] mutation"
   ```

2. Check recent logs for context:
   - Count total mutations attempted (from MUTATION_COMPLETE logs)
   - Count total paths expected (from LOOP_INIT log)
   - Compare: `mutations_attempted >= total_paths`?

3. Make conservative decision:
   - IF `mutations_attempted >= total_paths`:
     * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "STATE_RECOVERY: Mutations complete, marking type done"`
     * Mark type complete, exit mutation loop
   - ELSE:
     * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "STATE_RECOVERY: Unsafe to continue - returning partial results"`
     * GO TO STEP 5 IMMEDIATELY with partial results

**Purpose**: When state is ambiguous, fail safe by returning results rather than guessing.
</StateRecoveryProtocol>

<ParameterExtractionRecovery>
**When mutation fails with "Unable to extract parameters":**

1. **Log the framework error**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Framework error: Unable to extract parameters - will retry with reordered parameters"`
2. **Reorder parameters immediately** - change the order of parameters in your tool call
3. **Log the reordered parameters**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} info "Reordered parameters: ${reordered_params}"` so that you have the context of what you have changed readily present in your thinking
4. **Increment `retry_count`**
5. **Restart this mutation path**: Go back to "EXECUTE MUTATION" in <MutationTestingLoop/> for this same path with reordered parameters, which will automatically:
   - Apply <LogMutationAttempt/> (shows retry attempt in logs)
   - Execute mutation with reordered parameters
   - Check result via normal flow
6. **On retry result**:
   - If SUCCESS → Track in `mutations_passed`, continue to next mutation path
   - If FAILURE → Apply <LogOperationFailure operation="Mutation [path]"/>, mark failed, continue to next mutation path

**Why this works:** Parameter order doesn't matter semantically, but reordering breaks mental loops that cause extraction errors.
</ParameterExtractionRecovery>

<InvalidTypeStringRecovery>
**When mutation fails with "invalid type: string":**

This means you stringified a value that should be a JSON primitive (number, boolean, array, or object).

**Error patterns → Fix:**
- `"invalid type: string \"42\", expected f32"` → Use `42` not `"42"`
- `"invalid type: string \"[1, 2]\", expected a sequence"` → Use `[1, 2]` not `"[1, 2]"`
- `"invalid type: string \"true\", expected a boolean"` → Use `true` not `"true"`
- `"invalid type: string \"{...}\", expected reflected struct"` → Use `{...}` not `"{...}"`

**Recovery steps:**
1. **Identify the stringified value** in your mutation params
2. **Convert to proper JSON type** (unquote numbers/booleans, parse arrays/objects)
3. **Retry immediately** with the unquoted value
4. **Increment `retry_count`**
5. **Check retry result**:
   - If SUCCESS → Track in `mutations_passed`, continue
   - If FAILURE with DIFFERENT error → Mark failed, continue
   - If FAILURE with SAME error → You didn't fix it correctly, mark failed, continue
</InvalidTypeStringRecovery>

<UuidParsingRecovery>
**When mutation fails with "UUID parsing failed" AND "found \`\"\` at":**

This means you double-quoted a UUID string (your bug, not BRP's).

**Example:**
- Type guide shows: `"example": "550e8400-e29b-41d4-a716-446655440000"`
- You sent: `{"value": "\"550e8400-e29b-41d4-a716-446655440000\""}` (double-quoted)
- Error: `UUID parsing failed: invalid character: expected an optional prefix of 'urn:uuid:' followed by [0-9a-fA-F-], found '"' at 1`

**Recovery steps:**
1. **Remove the extra quotes** around the UUID string
2. **Use the UUID AS-IS** from the type guide example (already a JSON string)
3. **Retry immediately** with single-quoted UUID
4. **Increment `retry_count`**
5. **Check retry result**:
   - If SUCCESS → Track in `mutations_passed`, continue
   - If FAILURE with DIFFERENT error → Mark failed, continue
</UuidParsingRecovery>

<CalculateAndCheckTokens>
**Calculate token usage and check bailout threshold:**

1. **Extract reported token count** from most recent `<system_warning>` after previous tool call
   - Pattern: `Token usage: X/200,000`
2. **Calculate totals**:
   - Hidden overhead = 73,600 tokens (MCP tools ~28.6K + autocompact buffer ~45K)
   - **CALCULATED total = 73,600 + reported**
   - Percentage = (CALCULATED total / 200,000) * 100
3. **Log usage**: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} log "${context} - usage (hidden: 73.6K, reported: XK, total: YK, Z%)"`
   - Replace `${context}` with the context parameter passed to this section
4. **BAILOUT CHECK**:
   - **IF CALCULATED total >= 180,000 tokens (90% of 200K)**:
     * Log: `.claude/scripts/mutation_test_subagent_log.sh ${PORT} error "Context limit reached - total YK >= 180K (Z%) - returning partial results"`
     * **GO TO STEP 6 IMMEDIATELY**
     * Return results for all completed types with actual status
     * Do NOT continue with current operation
</CalculateAndCheckTokens>

**FINAL INSTRUCTION**: Execute <FinalValidation/> then output ONLY the JSON array from <ReturnResults/>. Nothing before. Nothing after.
