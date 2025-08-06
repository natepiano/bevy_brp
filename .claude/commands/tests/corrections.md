# Format Discovery and Correction Tests

## Objective
Validate BRP format discovery system including:
1. Transform format correction (object {x,y,z} → array [x,y,z])
2. Registry behavior with components lacking Serialize/Deserialize traits
3. Proper error handling for non-transformable input

**CRITICAL** You must include the specified {{PORT}} in the call to the tool or it will default to 15702 and FAIL!

## Test Steps

### 1. Transform Format Correction - Transformable Input
**STEP 1**: Query for entities with Transform:
- Tool: mcp__brp__bevy_query
- Use data: {"components": []}
- Use filter: {"with": ["bevy_transform::components::transform::Transform"]}

**STEP 2**: Test object-to-array transformation WITH THESE EXACT PARAMETERS:
```
mcp__brp__bevy_insert with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "components": {
    "bevy_transform::components::transform::Transform": {
      "translation": {"x": 10.0, "y": 20.0, "z": 30.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
      "scale": {"x": 2.0, "y": 2.0, "z": 2.0}
    }
  },
  "port": {{PORT}}
}
```

**Expected Result**:
- ✅ Success with format_corrected: "succeeded"
- ✅ Warning field in metadata when corrections applied
- ✅ Message: "Operation succeeded with 1 format correction(s) applied. See format_corrections field for details."
- ✅ Object format {x,y,z} automatically converted to array [x,y,z]

### 2. Transform Format Correction - Non-Transformable Input
**STEP 1**: Test malformed input that cannot be transformed:
```
mcp__brp__bevy_insert with parameters:
{
  "entity": [USE_ENTITY_ID_FROM_QUERY],
  "components": {
    "bevy_transform::components::transform::Transform": {
      "translation": 123,
      "rotation": 456,
      "scale": 789
    }
  },
  "port": {{PORT}}
}
```

**Expected Result**:
- ❌ Error with error_info containing format_corrected: "not_attempted" or "attempted_but_failed"
- ❌ Guidance message with correct format example
- ❌ No invented values - system should not guess what user intended

### 3. Mutation Should Work (Even Without Serialize/Deserialize)
**STEP 1**: Query for entities with Visibility:
- Tool: mcp__brp__bevy_query
- Use data: {"components": ["bevy_render::view::visibility::Visibility"]}
- Use filter: {"with": ["bevy_render::view::visibility::Visibility"]}

**STEP 2**: Execute this EXACT tool call:

Execute this EXACT tool call:
- Tool: mcp__brp__bevy_mutate_component  
- entity: [ENTITY_ID_FROM_QUERY]
- component: bevy_render::view::visibility::Visibility
- path: (empty string - pass exactly: "")
- value: Visible  
- port: {{PORT}}

**CRITICAL PATH PARAMETER**: The path must be an empty string "". This is NOT:
- null
- undefined  
- missing entirely
- the word "empty"
It is literally two quotation marks with nothing between them: ""

**MANDATORY: You MUST type out this exact verification before making the tool call:**
"PARAMETER VERIFICATION:
- entity: [TYPE THE ACTUAL NUMBER HERE] ✓ 
- component: bevy_render::view::visibility::Visibility ✓
- path: \"\" ✓ 
- value: \"Visible\" ✓
- port: {{PORT}} ✓"

**CRITICAL**: If you cannot type this exact verification with actual values, STOP IMMEDIATELY and report "SUB-AGENT FAILED TO VALIDATE PARAMETERS - I am unable to execute the required tool call correctly"

**AFTER the tool call, you MUST verify the response parameters section matches your verification above. If not, report "SUB-AGENT FAILED POST-CALL VERIFICATION - I passed incorrect parameters to the tool"**

**STEP 3**: After success, test other variants using the SAME parameter structure:
- Call again with "value": "Hidden" (keep all other parameters the same)
- Call again with "value": "Inherited" (keep all other parameters the same)

### 4. Transform Format Correction - Spawn Test
**STEP 1**: Test spawn with transformable object format:
```
mcp__brp__bevy_spawn with parameters:
{
  "components": {
    "bevy_transform::components::transform::Transform": {
      "translation": {"x": 5.0, "y": 15.0, "z": 25.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
      "scale": {"x": 1.5, "y": 1.5, "z": 1.5}
    }
  },
  "port": {{PORT}}
}
```

**Expected Result**:
- ✅ Success with format_corrected: "succeeded"
- ✅ Warning field in metadata when corrections applied
- ✅ Message: "Operation succeeded with 1 format correction(s) applied. See format_corrections field for details."
- ✅ Returns new entity ID

### 5. Component Without Serialize/Deserialize - Spawn Test
- Execute `mcp__brp__bevy_spawn` with Visibility component
- Verify spawn fails with registry diagnostic
- Check error mentions "lacks Serialize and Deserialize traits"
- Confirm error includes BRP registration requirements guidance

### 6. Component Without Serialize/Deserialize - Insert Test
- Spawn entity with basic Transform
- Execute `mcp__brp__bevy_insert` with Aabb component
- Verify insert fails with appropriate registry error
- Check error message is helpful and actionable

### 7. Registry Requirements Validation
- Execute `mcp__brp__bevy_list` to see registered components
- Verify all reflection-registered components appear
- Check that Transform, Name appear (have Serialize/Deserialize traits)
- Confirm Visibility, Aabb appear in list (registered but missing Serialize/Deserialize traits)

### 8. Error Message Quality Check
- Verify all registry errors include:
  - Clear problem description
  - Specific missing traits (Serialize, Deserialize)
  - Guidance on BRP registration requirements
  - Helpful suggestions for resolution

### 9. Enum Mutation Error Message Test
- Execute `mcp__brp__bevy_mutate_component` with INCORRECT syntax:
  - Path: ".Visible"
  - Value: {}
- Verify error response includes:
  - Error message mentioning variant access issue
  - Error info with:
    - "hint" field explaining empty path requirement
    - "valid_values" array listing all variants
    - "examples" showing correct usage
  - Hint should indicate proper usage like "Use empty path with variant name as value"
- This ensures users get helpful guidance when making this common mistake

## Expected Results

### Transform Format Correction
- ✅ **Transformable input succeeds**: Object format {x,y,z} → array [x,y,z] with format_corrected: "succeeded"
- ✅ **Non-transformable input fails**: Integers/invalid data returns error with guidance, format_corrected: "not_attempted" or "attempted_but_failed"
- ✅ **No value invention**: System never creates fake data when transformation fails
- ✅ **Clear success messaging**: "Operation succeeded with N format correction(s) applied. See format_corrections field for details." when corrected
- ✅ **Spawn and insert both work**: Format correction applies to both operations

### Registry Behavior
- ✅ Spawn fails appropriately for components lacking Serialize/Deserialize
- ✅ Insert fails appropriately for components lacking Serialize/Deserialize
- ✅ Mutation works for reflection-registered components (even without Serialize/Deserialize)
- ✅ Component listing shows all reflection-registered types (regardless of Serialize/Deserialize)
- ✅ Error messages are clear and actionable
- ✅ Registration requirements are well explained

## Failure Criteria
STOP if:
- Transform correction invents values instead of failing gracefully
- format_corrected field is missing or incorrect
- Registry errors are unclear
- Mutation fails for registered components
- Error guidance is insufficient
- Success message doesn't indicate when format correction occurred
