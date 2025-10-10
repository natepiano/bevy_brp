# Mutation Test Subagent Instructions

**CRITICAL**: Execute the workflow defined in <SubagentExecutionFlow/>

<SubagentExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Read and internalize <ErrorRecoveryProtocol/>
    **STEP 2:** Read <SubagentContext/> to understand your assignment
    **STEP 3:** Execute <FetchAssignment/>
    **STEP 4:** Execute <ParseAssignmentData/>
    **STEP 5:** Execute <TestAllTypes/>
    **STEP 6:** Execute <PreFailureCheck/> before reporting any failures
    **STEP 7:** Execute <ReturnResults/> with JSON output
    **STEP 8:** Execute <FinalValidation/> before sending response
</SubagentExecutionFlow>

**CRITICAL RESPONSE LIMIT**: Return ONLY the JSON array result. NO explanations, NO commentary, NO test steps, NO summaries.

<ErrorRecoveryProtocol>
═══════════════════════════════════════════════════════════════════════════════
🚨 MANDATORY ERROR RECOVERY PROTOCOL - READ THIS FIRST 🚨
═══════════════════════════════════════════════════════════════════════════════

**BEFORE YOU REPORT ANY FAILURE, YOU MUST CHECK IF IT'S A PRIMITIVE QUOTING ERROR**

**IF YOU SEE THIS ERROR**: `invalid type: string "X", expected [number/boolean type]`

**THIS MEANS**:
- ❌ YOU sent "X" as a quoted string (YOUR BUG, not BRP bug)
- ❌ This is NOT a test failure - it's YOUR serialization error
- ✅ You MUST retry immediately with unquoted primitive
- ✅ ONLY report as failure if retry fails with DIFFERENT error

**CONCRETE EXAMPLE - FOLLOW THESE EXACT STEPS**:

1. **You send**: `{"method": "world_mutate_components", "params": {"value": "true"}}`
   - ⚠️ ERROR: You quoted the boolean!

2. **You receive**: `invalid type: string "true", expected a boolean`
   - 🔍 RECOGNIZE: This error means YOU sent "true" (string) instead of true (boolean)

3. **YOU MUST DO THIS IMMEDIATELY**:
   - **STEP 1:** DO NOT report this as a test failure
   - **STEP 2:** Verify your value: true is a BOOLEAN, not a STRING
   - **STEP 3:** Retry the SAME mutation with: {"value": true}  (no quotes!)
   - **STEP 4:** If retry succeeds → mark mutation as PASSED
   - **STEP 5:** If retry fails with DIFFERENT error → then report as failure

4. **ONLY REPORT FAILURE IF**:
   - The retry ALSO fails AND the error is NOT about string quoting

**VERIFICATION CHECKLIST - COMPLETE BEFORE EVERY MUTATION**:
□ My value is a number (like 42)? → Ensure params shows `"value": 42` NOT `"value": "42"`
□ My value is a boolean (like true)? → Ensure params shows `"value": true` NOT `"value": "true"`
□ I see quotes around my number/boolean? → STOP! Remove the quotes!
□ I received "invalid type: string" error? → Follow ERROR RECOVERY PROTOCOL above!

═══════════════════════════════════════════════════════════════════════════════

**IF YOU GET STUCK IN A LOOP CALLING TOOLS WITH EMPTY PARAMETERS**:

**SYMPTOM**: You find yourself repeatedly calling MCP tools (like `mcp__brp__world_mutate_components`) with empty or incomplete parameter lists, unable to construct the full parameters.

**SOLUTION**: **REORDER THE PARAMETERS** in your tool call and try again. This will break you out of the mental loop.
 Parameter order doesn't matter for the MCP tool - reordering just helps you break the loop

**EXAMPLE**:
- ❌ STUCK: Calling `mcp__brp__world_mutate_components(entity=123, component="type", path=".field", value=42)`
- ✅ UNSTUCK: Reorder to `mcp__brp__world_mutate_components(component="type", entity=123, value=42, path=".field")`

</ErrorRecoveryProtocol>

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

**ERROR RECOVERY PROTOCOL**:
If you receive error containing: `invalid type: string "X", expected [any type]`:
1. **RECOGNIZE**: This means you DEFINITELY sent the value as a quoted string
2. **DO NOT** report this as a test failure - this is YOUR bug, not a BRP bug
3. **IDENTIFY THE TYPE**:
   - "expected reflected list value" → You stringified an array
   - "expected a boolean" → You stringified a boolean
   - "expected f32" → You stringified a number
   - "expected reflected struct" → You stringified an object
4. **FIX IMMEDIATELY**: Retry the SAME mutation with the value in proper JSON form:
   - Arrays: Remove outer quotes, send as native JSON array
   - Objects: Remove outer quotes, send as native JSON object
   - Primitives: Remove quotes, send as native JSON number/boolean
5. **VERIFY**: Before retry, inspect your params structure - ensure NO outer quotes
6. **ONLY FAIL**: If the retry also fails with a DIFFERENT error message

**VALIDATION**: Before sending ANY mutation, verify the entire value is in native JSON form (not a string representation)
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

**CRITICAL CONSTRAINT**: You MUST test ONLY the exact types provided in your assignment data. The test system controls type names completely.

**Follow <TypeNameValidation/> requirements exactly** before testing each type.
</SubagentContext>

<FetchAssignment>
**Execute the assignment script to get your assigned type names**:

```bash
python3 ./.claude/scripts/mutation_test_get_subagent_assignments.py \
    --batch ${BATCH_NUMBER} \
    --max-subagents ${MAX_SUBAGENTS} \
    --types-per-subagent ${TYPES_PER_SUBAGENT} \
    --subagent-index ${SUBAGENT_INDEX}
```

**This returns a JSON object with a `type_names` array containing ONLY the literal type name strings**:
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
</FetchAssignment>

<ParseAssignmentData>
**Parse the type_names array from assignment and fetch each type guide:**

The assignment gives you a simple array of type names. For EACH type name, you MUST:

1. **Extract the literal type name string** from the `type_names` array
2. **Fetch its type guide** using the script:
   ```bash
   ./.claude/scripts/get_type_guide.sh <EXACT_TYPE_NAME> --file .claude/transient/all_types.json
   ```

**CRITICAL - TYPE NAME HANDLING:**
- The type name from `type_names` is a LITERAL STRING
- COPY it EXACTLY when calling get_type_guide.sh
- NEVER retype or reconstruct the type name from memory
- Use the SAME EXACT STRING in all BRP operations

**Example workflow:**
```bash
# Assignment returned: {"type_names": ["bevy_pbr::cluster::ClusterConfig"]}

# For each type name in the array:
./.claude/scripts/get_type_guide.sh bevy_pbr::cluster::ClusterConfig --file .claude/transient/all_types.json
```

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

- **`guide.spawn_format`**: Example value for entity creation (may be `null`)
  - If not `null` AND "spawn" in `supported_operations`: Use in spawn/insert
  - If `null` OR "spawn" not supported: Skip spawn/insert testing

- **`guide.mutation_paths`**: Dictionary of testable mutation paths
  - Keys are path strings (e.g., `""`, `".field"`, `".nested.value"`)
  - Each path has an `example` value to use in mutations
  - Check `path_info.mutation_status` before testing (skip if `"not_mutable"`)

- **`guide.supported_operations`**: Which BRP methods work with this type
  - Check before calling: If "spawn" not in list, don't call world_spawn_entity
  - If "mutate" not in list, don't call world_mutate_components
</ParseAssignmentData>

<TestAllTypes>
**Testing Protocol**:

For each type name string in your `type_names` array:
   1. **FETCH TYPE GUIDE**: Call `get_type_guide.sh <type_name> --file .claude/transient/all_types.json`
   2. **EXTRACT TYPE NAME**: Get the `type_name` field from the script output - this is your AUTHORITATIVE string
   3. **TEST THE TYPE**:

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
      - **CHECK `schema_info.reflect_types` to determine type category**
      - **RESOURCE DETECTION**:
        * IF "Resource" in schema_info.reflect_types → **THIS IS A RESOURCE**
        * Execute <ResourceTestingProtocol/> ONLY
        * **SKIP** all component testing steps completely
        * **NEVER** use `world_spawn_entity` or `world_mutate_components` for resources
      - **COMPONENT DETECTION**:
        * IF "Component" in schema_info.reflect_types → **THIS IS A COMPONENT**
        * Execute <ComponentTestingProtocol/> ONLY
        * **SKIP** all resource testing steps completely
        * **NEVER** use `world_insert_resources` or `world_mutate_resources` for components
      - **ERROR CASE**: If neither "Resource" nor "Component" found in reflect_types, report error in failure_details

   e. **ENTITY ID SUBSTITUTION FOR MUTATIONS**:
      - **CRITICAL**: If any mutation example contains the value `8589934670`, this is a PLACEHOLDER Entity ID
      - **YOU MUST**: Replace ALL instances of `8589934670` with REAL entity IDs from the running app
      - **HOW TO GET REAL ENTITY IDs**:
        1. First query for existing entities: `world_query` with appropriate filter
        2. Use the entity IDs from query results
        3. If testing EntityHashMap types, use the queried entity ID as the map key
      - **EXAMPLE**: If mutation example shows `{"8589934670": [...]}` for an EntityHashMap:
        - Query for an entity with the component first
        - Replace `8589934670` with the actual entity ID from the query
        - Then perform the mutation with the real entity ID
      - **FOR HIERARCHY COMPONENTS** (Children, Parent):
        - **CRITICAL**: Query for ALL entities in the scene using `world_query` with no filter
        - Use a DIFFERENT entity ID than the one being mutated
        - **NEVER** create circular relationships (entity as its own parent/child)
        - Example: When testing entity 4294967390's `Children` component:
          - ❌ WRONG: Use [4294967390] as the child value (circular reference → CRASH)
          - ✅ CORRECT: Query all entities, select a different ID like 4294967297
        - If only one entity exists with the component, query for other entities without that component to use as children
   e. **ROOT EXAMPLE SETUP FOR VARIANT-DEPENDENT PATHS**:
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
   f. **MUTATION TESTING**: Test ONLY mutable paths from the mutation_paths object
      - **CHOOSE MUTATION METHOD** based on type category:
        * **FOR COMPONENTS**: Use `world_mutate_components` with entity ID
        * **FOR RESOURCES**: Use `world_mutate_resources` (no entity ID needed)
      - **SKIP NON-MUTABLE PATHS**: Check `path_info.mutation_status` before attempting ANY mutation:
        * `"not_mutable"` → SKIP (don't count in total)
        * `"partially_mutable"` → SKIP unless `example` or `examples` exists
        * `"mutable"` or missing → TEST normally
      - Apply Entity ID substitution BEFORE sending any mutation request (components only)
      - If a mutation uses Entity IDs and you don't have real ones, query for them first
      - **CRITICAL VALUE HANDLING**: Extract the `example` value from mutation_paths and follow <JsonPrimitiveRules/> when using it
      - **ENUM TESTING REQUIREMENT**: When a mutation path contains an "examples" array (indicating enum variants), you MUST test each example individually:
        * For each entry in the "examples" array, perform a separate mutation using that specific "example" value
        * Example: If `.depth_load_op` has examples `[{"example": {"Clear": 3.14}}, {"example": "Load"}]`, test BOTH:
          1. Mutate `.depth_load_op` with `{"Clear": 3.14}`
          2. Mutate `.depth_load_op` with `"Load"`
        * Count each example test as a separate mutation attempt in your totals
      - **IMPORTANT**: Only count actually attempted mutations in `total_mutations_attempted`
3. **CAPTURE ALL ERROR DETAILS**: When ANY operation fails, record the COMPLETE request and response
4. NEVER test types not provided in your assignment data

**IMPORTANT**: Follow <JsonPrimitiveRules/> - validate every `"value"` field before sending mutations.
**ERROR SIGNAL**: "invalid type: string" means you quoted a primitive.
**IF YOU GET THIS ERROR**: Follow the ERROR RECOVERY PROTOCOL in <JsonPrimitiveRules/> - retry immediately with the unquoted value, do NOT report as test failure unless retry fails with a different error.
</TestAllTypes>

<ResourceTestingProtocol>
**RESOURCE TESTING PROTOCOL - Use ONLY for types with "Resource" in schema_info.reflect_types**

**CRITICAL**: Do NOT use component methods (`world_spawn_entity`, `world_mutate_components`) - these will CRASH the app

1. **SKIP CHECK**: If `guide.spawn_format` is `null`, SKIP all insert/mutation testing for this resource

2. **INSERT RESOURCE**:
   - Use `world_insert_resources` tool
   - Pass `resource` parameter with exact type name from type guide
   - Pass `value` parameter with `spawn_format` data
   - **NEVER** use `world_spawn_entity` - resources are NOT spawned as entities

3. **VERIFY INSERTION**:
   - Use `world_get_resources` tool
   - Pass `resource` parameter with exact type name
   - Confirms the resource exists in the world

4. **MUTATION TESTING**:
   - Use `world_mutate_resources` tool (NOT `world_mutate_components`)
   - Pass `resource` parameter with exact type name
   - Pass `path` parameter with mutation path from type guide
   - Pass `value` parameter with example from type guide
   - Follow <JsonPrimitiveRules/> for value formatting
   - **NO entity ID parameter** - resources don't have entities

5. **SKIP ENTITY QUERY**:
   - Resources are NOT attached to entities
   - Do NOT query for entities with `world_query`
   - Set `entity_query: false` in operations_completed
   - Set `entity_id: null` in result
</ResourceTestingProtocol>

<ComponentTestingProtocol>
**COMPONENT TESTING PROTOCOL - Use ONLY for types with "spawn" in supported_operations**

**CRITICAL**: Do NOT use resource methods (`world_insert_resources`, `world_mutate_resources`)

1. **SKIP CHECK**: If `guide.spawn_format` is `null`, SKIP all spawn/insert testing for this component

2. **SPAWN ENTITY**:
   - Use `world_spawn_entity` tool
   - Pass `components` parameter with type name as key and `spawn_format` as value
   - **NEVER** use `world_insert_resources` - components are spawned with entities

3. **QUERY FOR ENTITY**:
   - Use `world_query` tool
   - Pass `filter: {"with": ["EXACT_TYPE_NAME"]}` to find entities
   - Pass `data: {}` to get entity IDs only
   - Store entity ID for mutation testing

4. **MUTATION TESTING**:
   - Use `world_mutate_components` tool (NOT `world_mutate_resources`)
   - Pass `entity` parameter with entity ID from query
   - Pass `component` parameter with exact type name
   - Pass `path` parameter with mutation path from type guide
   - Pass `value` parameter with example from type guide
   - Follow <JsonPrimitiveRules/> for value formatting
   - Apply Entity ID substitution per <TestAllTypes/> section e
</ComponentTestingProtocol>

<PreFailureCheck>
═══════════════════════════════════════════════════════════════════════════════
🛑 MANDATORY PRE-FAILURE-REPORT CHECK 🛑
═══════════════════════════════════════════════════════════════════════════════

**BEFORE YOU REPORT status: "FAIL" FOR ANY TYPE, ANSWER THESE QUESTIONS**:

1. ❓ Did ANY mutation fail with error: `invalid type: string "X", expected [type]`?
   - ✅ YES → Did you retry with unquoted primitive? If NO, you MUST retry now!
   - ✅ NO → Proceed to report failure

2. ❓ After retrying with unquoted primitive, did the mutation succeed?
   - ✅ YES → Mark mutation as PASSED, DO NOT report as failure
   - ✅ NO → Proceed to report failure (only if retry also failed)

3. ❓ Are you 100% certain this is NOT a primitive quoting error on your part?
   - ✅ YES → Proceed to report failure
   - ✅ NO → Review ERROR RECOVERY PROTOCOL at top of this prompt

**IF YOU SKIP THESE CHECKS, YOUR RESULTS WILL BE INVALID**

═══════════════════════════════════════════════════════════════════════════════
</PreFailureCheck>

<ReturnResults>
**CRITICAL FIELD REQUIREMENTS**:
- `type`: Extract from the `type_name` field returned by `get_type_guide.sh` - this is the AUTHORITATIVE type name
- `tested_type`: The exact type name string you passed to BRP queries - MUST be identical to `type`
- Purpose: Detects if you hallucinated or modified a type name (CRITICAL BUG if they differ)
- **BOTH MUST MATCH**: The string from assignment's `type_names` array = type guide's `type_name` = what you used in BRP calls

**Return EXACTLY this format (nothing else)**:
```json
[{
  "type": "[type_name from assignment script - REQUIRED]",
  "tested_type": "[actual type used in queries - MUST match 'type']",
  "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
  "entity_id": 123,  // Entity ID if created, null otherwise
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

**PRE-OUTPUT VALIDATION**: Before generating your final JSON, follow <JsonPrimitiveRules/> and pay special attention to `"value"` fields in failure_details.
</ReturnResults>

<FinalValidation>
═══════════════════════════════════════════════════════════════════════════════
🚨 FINAL VALIDATION BEFORE OUTPUT 🚨
═══════════════════════════════════════════════════════════════════════════════

**STOP! Before you output your JSON, answer YES/NO to each**:

1. ❓ Did I follow ERROR RECOVERY PROTOCOL for ANY "invalid type: string" errors?
   - If you got this error and did NOT retry with unquoted primitive, YOUR RESULTS ARE INVALID

2. ❓ Did I complete the MANDATORY PRE-FAILURE-REPORT CHECK above?
   - If you reported ANY failure without completing the check, YOUR RESULTS ARE INVALID

3. ❓ Are ALL my failure reports legitimate (not primitive quoting errors)?
   - If ANY failure is actually a primitive quoting error, YOUR RESULTS ARE INVALID

**IF YOU CANNOT ANSWER YES TO ALL THREE, DO NOT OUTPUT YOUR JSON - GO BACK AND FIX IT**

═══════════════════════════════════════════════════════════════════════════════
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

**FINAL INSTRUCTION**: Execute <FinalValidation/> then output ONLY the JSON array from <ReturnResults/>. Nothing before. Nothing after.
