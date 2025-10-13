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

**Error patterns → Fix**:
- `expected a sequence` or `expected reflected list` → Unquoted array
- `expected a boolean` → Unquoted boolean
- `expected f32/i32/u32` → Unquoted number
- `expected reflected struct` → Unquoted object

**Tracking**: Keep `retry_count` variable starting at 0, increment for each "invalid type: string" error you retry. Include in final output.

**IF YOU GET STUCK IN A LOOP CALLING TOOLS WITH EMPTY PARAMETERS**:
Reorder parameters in your tool call - parameter order doesn't matter, but reordering breaks mental loops.
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
   f. **MUTATION TESTING**: For each path in mutation_paths, validate THEN test
      - **FOR EACH path in mutation_paths object:**
        1. **CHECK mutation_status FIRST**: If `path_info.mutation_status == "not_mutable"` → SKIP path (don't count in total)
        2. **CHECK for example**: If no `example` or `examples` field exists → SKIP path (cannot test without value)
        3. **IF partially_mutable**: SKIP unless `example` or `examples` exists
        4. **ONLY if checks pass**: Proceed to mutation
      - **CHOOSE MUTATION METHOD** based on type category:
        * **FOR COMPONENTS**: Use `world_mutate_components` with entity ID
        * **FOR RESOURCES**: Use `world_mutate_resources` (no entity ID needed)
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

**AFTER EACH MUTATION**: If error contains "invalid type: string", follow <ErrorRecoveryProtocol/> immediately.
</TestAllTypes>

<ResourceTestingProtocol>
**RESOURCE TESTING PROTOCOL - Use ONLY for types with "Resource" in schema_info.reflect_types**

**CRITICAL**: Do NOT use component methods (`world_spawn_entity`, `world_mutate_components`) - these will CRASH the app

1. **INSERT CHECK**: If `guide.spawn_format` is NOT null:
   - Use `world_insert_resources` tool
   - Pass `resource` parameter with exact type name from type guide
   - Pass `value` parameter with `spawn_format` data
   - Then verify insertion with `world_get_resources`
   - Set `spawn_insert: true` in operations_completed

2. **IF spawn_format IS null**:
   - Skip insert step (root is `partially_mutable`)
   - Resource must already exist in the running app
   - Set `spawn_insert: false` in operations_completed
   - Proceed directly to mutation testing

3. **MUTATION TESTING** (ALWAYS execute if mutation paths exist):
   - Use `world_mutate_resources` tool (NOT `world_mutate_components`)
   - Pass `resource` parameter with exact type name
   - Pass `path` parameter with mutation path from type guide
   - Pass `value` parameter with example from type guide
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
   - Use `world_spawn_entity` tool
   - Pass `components` parameter with type name as key and `spawn_format` as value
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
   - Set `entity_query: true` in operations_completed

4. **MUTATION TESTING** (ALWAYS execute if entity found and mutation paths exist):
   - Use `world_mutate_components` tool (NOT `world_mutate_resources`)
   - Pass `entity` parameter with entity ID from query
   - Pass `component` parameter with exact type name
   - Pass `path` parameter with mutation path from type guide
   - Pass `value` parameter with example from type guide
   - Follow <JsonPrimitiveRules/> for value formatting
   - Apply Entity ID substitution per <TestAllTypes/> section e
</ComponentTestingProtocol>

<PreFailureCheck>
**BEFORE REPORTING ANY FAILURE**:

1. Count "invalid type: string" errors received: _____
2. Does this match your `retry_count`? (must be equal)
3. Are you reporting ANY failures that had "invalid type: string"?
   - If YES → Protocol violation - those must be retried first per <ErrorRecoveryProtocol/>

If any check fails, go back and follow <ErrorRecoveryProtocol/>.
</PreFailureCheck>

<ReturnResults>
**CRITICAL FIELD REQUIREMENTS**:
- `type`: Extract from the `type_name` field returned by `get_type_guide.sh` - this is the AUTHORITATIVE type name
- `tested_type`: The exact type name string you passed to BRP queries - MUST be identical to `type`
- `retry_count`: Number of "invalid type: string" errors you retried (required for validation)
- Purpose: Detects if you hallucinated or modified a type name (CRITICAL BUG if they differ)
- **BOTH MUST MATCH**: The string from assignment's `type_names` array = type guide's `type_name` = what you used in BRP calls

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

**PRE-OUTPUT VALIDATION**: Before generating your final JSON, follow <JsonPrimitiveRules/> and pay special attention to `"value"` fields in failure_details.
</ReturnResults>

<FinalValidation>
**VERIFY BEFORE OUTPUT**:
- `retry_count` matches number of "invalid type: string" errors received
- No failures reported with "invalid type: string" in error_message
- All failure values are proper JSON types (not strings)

If any fail: Review <ErrorRecoveryProtocol/> and fix before output.
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
