# Instructions

## Configuration Parameters

These config values are provided:
- TEST_PLAN_FILE: Path to the JSON test plan file to execute
- PORT: BRP port number for MCP tool operations

## Your Job

**Execute the test plan and update results after each operation.**

## Execution Steps

Loop until finished:

1. **Get next assignment**:
   ```bash
   python3 .claude/scripts/mutation_test/operation_manager.py --port PORT --action get-next
   ```
   Parse the JSON response

2. **Check response status**:
   - If `"status": "finished"` → All operations complete, EXIT with success message
   - If `"status": "next_operation"` → Continue to step 3

3. **Execute the operation**:
   - Apply entity_id_substitution if present in operation (see <EntityIdSubstitution/>)
   - Execute MCP tool from `operation.tool` with parameters from `operation` object
   - Hook automatically updates operation status (SUCCESS or FAIL)

4. **Handle result**:
   - If SUCCESS → Loop back to step 1 (get next operation)
   - If FAIL → Execute <MatchErrorPattern/>:
     - If recoverable → Fix parameters and retry from step 3 (operation still marked FAIL, next_assignment returns same operation)
     - If unrecoverable → EXIT with error message (stop execution)

5. **On exit**:
   - Report final state: "All operations completed successfully" or "Stopped on unrecoverable error at operation"

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
