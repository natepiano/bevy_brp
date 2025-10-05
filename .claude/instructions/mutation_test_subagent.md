# Mutation Test Subagent Instructions

**CRITICAL**: Execute the workflow defined in <SubagentExecutionFlow/>

<SubagentExecutionFlow>
    **EXECUTE THESE STEPS IN ORDER:**

    **STEP 1:** Read and internalize <ErrorRecoveryProtocol/>
    **STEP 2:** Read <SubagentContext/> to understand your assignment
    **STEP 3:** Execute <FetchAssignment/>
    **STEP 4:** Execute <TestAllTypes/>
    **STEP 5:** Execute <PreFailureCheck/> before reporting any failures
    **STEP 6:** Execute <ReturnResults/> with JSON output
    **STEP 7:** Execute <FinalValidation/> before sending response
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

1. **You send**: `{"method": "bevy/mutate_component", "params": {"value": "true"}}`
   - ⚠️ ERROR: You quoted the boolean!

2. **You receive**: `invalid type: string "true", expected a boolean`
   - 🔍 RECOGNIZE: This error means YOU sent "true" (string) instead of true (boolean)

3. **YOU MUST DO THIS IMMEDIATELY**:
   ```
   Step 1: DO NOT report this as a test failure
   Step 2: Verify your value: true is a BOOLEAN, not a STRING
   Step 3: Retry the SAME mutation with: {"value": true}  (no quotes!)
   Step 4: If retry succeeds → mark mutation as PASSED
   Step 5: If retry fails with DIFFERENT error → then report as failure
   ```

4. **ONLY REPORT FAILURE IF**:
   - The retry ALSO fails AND the error is NOT about string quoting

**VERIFICATION CHECKLIST - COMPLETE BEFORE EVERY MUTATION**:
□ My value is a number (like 42)? → Ensure params shows `"value": 42` NOT `"value": "42"`
□ My value is a boolean (like true)? → Ensure params shows `"value": true` NOT `"value": "true"`
□ I see quotes around my number/boolean? → STOP! Remove the quotes!
□ I received "invalid type: string" error? → Follow ERROR RECOVERY PROTOCOL above!

═══════════════════════════════════════════════════════════════════════════════
</ErrorRecoveryProtocol>

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
**Fetch your assignment data as your FIRST action**:
```bash
python3 ./.claude/scripts/mutation_test_get_subagent_assignments.py \
    --batch ${BATCH_NUMBER} \
    --max-subagents ${MAX_SUBAGENTS} \
    --types-per-subagent ${TYPES_PER_SUBAGENT} \
    --subagent-index ${SUBAGENT_INDEX}
```

This returns your specific assignment with complete type data.
</FetchAssignment>

<TestAllTypes>
**Testing Protocol**:
1. FIRST: Fetch your assignment using the script with --subagent-index parameter
2. VALIDATE: Ensure you received exactly ${TYPES_PER_SUBAGENT} types
3. For each type in your fetched assignment:
   a. **COMPONENT_NOT_FOUND VALIDATION**:
      - **IF** entity query returns 0 entities for a type:
        1. **STOP IMMEDIATELY** - do NOT report COMPONENT_NOT_FOUND yet
        2. **RE-FETCH** your assignment using the script again
        3. **COMPARE** the type name you tested against the assignment data
        4. **VERIFY** you used the EXACT type_name from the assignment (character-by-character match)
        5. **IF MISMATCH DETECTED**:
           - ERROR: You modified the type name - this is a CRITICAL BUG
           - Report the mismatch in your failure details
           - Show: Expected (from assignment) vs Actual (what you tested)
        6. **ONLY IF EXACT MATCH**: Report status as COMPONENT_NOT_FOUND
   b. **SPAWN/INSERT TESTING**:
      - **CHECK FIRST**: If `spawn_format` is `null` OR `supported_operations` does NOT include "spawn" or "insert", SKIP spawn/insert testing entirely
      - **ONLY IF** spawn_format exists AND supported_operations includes "spawn"/"insert": attempt spawn/insert operations
      - **NEVER** attempt spawn/insert on types that don't support it - this will cause massive error responses
   c. **ENTITY QUERY**: Query for entities with component using EXACT syntax:
   ```json
   {
     "filter": {"with": ["EXACT_TYPE_NAME_FROM_GUIDE"]},
     "data": {"components": []}
   }
   ```
   CRITICAL: Follow <TypeNameValidation/> - use the exact `type_name` field from the guide
   d. **ENTITY ID SUBSTITUTION FOR MUTATIONS**:
      - **CRITICAL**: If any mutation example contains the value `8589934670`, this is a PLACEHOLDER Entity ID
      - **YOU MUST**: Replace ALL instances of `8589934670` with REAL entity IDs from the running app
      - **HOW TO GET REAL ENTITY IDs**:
        1. First query for existing entities: `bevy_query` with appropriate filter
        2. Use the entity IDs from query results
        3. If testing EntityHashMap types, use the queried entity ID as the map key
      - **EXAMPLE**: If mutation example shows `{"8589934670": [...]}` for an EntityHashMap:
        - Query for an entity with the component first
        - Replace `8589934670` with the actual entity ID from the query
        - Then perform the mutation with the real entity ID
   e. **ROOT EXAMPLE SETUP FOR VARIANT-DEPENDENT PATHS**:
      - **BEFORE testing each mutation path**, check if `path_info.root_example` exists
      - **IF `root_example` EXISTS**:
        1. **First** mutate the root path (`""`) to set up the correct variant structure:
           - Use `bevy/mutate_component` or `bevy/mutate_resource`
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
      - **SKIP NON-MUTABLE PATHS**: Check `path_info.mutation_status` before attempting ANY mutation:
        * `"not_mutable"` → SKIP (don't count in total)
        * `"partially_mutable"` → SKIP unless `example` or `examples` exists
        * `"mutable"` or missing → TEST normally
      - Apply Entity ID substitution BEFORE sending any mutation request
      - If a mutation uses Entity IDs and you don't have real ones, query for them first
      - **CRITICAL VALUE HANDLING**:
        * Extract the `example` value from mutation_paths
        * Use it DIRECTLY - do NOT convert to string with str() or f-strings
        * If the example is a number like `42`, keep it as the NUMBER 42
        * If the example is a boolean like `true`, keep it as the BOOLEAN true
        * When building the mutation request, assign: `value = example` (not `value = str(example)`)
        * Verify your JSON shows `"value": 42` NOT `"value": "42"` before sending
      - **IMPORTANT**: Follow <JsonPrimitiveRules/> before EVERY mutation request
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
**Return EXACTLY this format (nothing else)**:
```json
[{
  "type": "[full::qualified::type::name]",
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
      "method": "bevy/mutate_component",
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
