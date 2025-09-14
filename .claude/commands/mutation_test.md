# Type Guide Comprehensive Validation Test

## Overview

**Command**: `/type_validation`

**Purpose**: Systematically validate ALL BRP component types by testing spawn/insert and mutation operations. Tracks progress in `{temp_dir}/all_types.json` with simple pass/fail status for each type. The file contains a `type_guide` array at the root with complete type information including mutation paths as a dict with examples.

**Process Summary**: Renumber batches → Launch/verify app → Test types in parallel batches → Process results → Cleanup

**Configuration**:
```
TYPES_PER_SUBAGENT = 1                                      # Types each subagent tests
MAX_SUBAGENTS = 10                                          # Parallel subagents per batch
BATCH_SIZE = MAX_SUBAGENTS * TYPES_PER_SUBAGENT            # Types per batch
BASE_PORT = 30001                                           # Starting port for subagents (30001-30010)
```

## Critical Execution Requirements

**CRITICAL PATH HANDLING**:
- **NEVER use `$TMPDIR` directly in Write tool file paths** - The Write tool does not expand environment variables
- **ALWAYS get the actual temp directory path first** using `echo $TMPDIR`
- **USE the expanded path** (e.g., `/var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/`) in all Write tool calls
- **This prevents creating literal `$TMPDIR` directories**

**Core Rules**:
1. **ALWAYS reassign batch numbers** - Clear and reassign every run using renumber script
2. **ALWAYS use parallel subagents** - Launch MAX_SUBAGENTS in parallel per batch
3. **Main agent orchestrates, subagents test** - Main agent never tests directly
4. **STOP ON ANY FAILURE** - If ANY type fails in a batch, STOP IMMEDIATELY. Do not continue to next batch
5. **Simple pass/fail per type** - One overall result per type

**Failure Handling**:
- **FAILURE MEANS STOP**: When any subagent reports a FAIL status, the entire test suite MUST stop
- Save progress and report the failure to the user
- **DO NOT CONTINUE** testing subsequent batches after any failure
- Test can be resumed later after fixing issues

**App Management**:
- **NEW MULTI-PORT ARCHITECTURE**: Each subagent gets its own dedicated port (30001-30010)
- **Main agent launches ALL apps**: Main agent launches 10 extras_plugin instances on ports 30001-30010
- **Subagents use their assigned port**: Subagent N uses port 30000+N (e.g., subagent 1 uses 30001)
- **Each subagent has isolated app**: No port conflicts, no shared state between tests
- **Main agent handles ALL app lifecycle**: Launch all 10 apps before testing, shutdown all after

**Common Failure Prevention**:
- **JSON Number Types**: ALL numeric primitives (u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64) MUST be JSON numbers, not strings
- **Critical Error**: `"invalid type: string"` means YOU serialized a number wrong - fix and retry before marking as FAIL
- **Large Numbers**: Even huge numbers like 18446744073709551615 are JSON numbers, not strings

## Complete Execution Procedure

### Step 1: Batch Setup and Renumbering

**MANDATORY FIRST STEP**: Always clear and reassign batch numbers using the renumbering script:

```bash
./.claude/commands/scripts/mutation_test_renumber_batches.sh BATCH_SIZE
```

This script will:
- Clear all existing batch numbers
- Assign new batch numbers to untested/failed types (BATCH_SIZE per batch)
- Display statistics about types to be tested

**CLEANUP PREVIOUS RUNS**: Remove any leftover batch result files from previous test runs:

```bash
rm -f $TMPDIR/batch_results_*.json
```

**NOTE**: When using the Write tool for creating files, you must use the actual expanded temp directory path (e.g., `/var/folders/.../T/`) rather than the `$TMPDIR` environment variable, as the Write tool does not expand environment variables.

This prevents interference from previous test runs and ensures clean batch result processing.

### Step 2: Application Management

**NEW: Launch ALL 10 apps on different ports (30001-30010)**:

1. **Shutdown any existing apps**: Clean slate for testing
   - Shutdown apps on ports 30001, 30002, 30003, 30004, 30005, 30006, 30007, 30008, 30009, 30010
   - Use `mcp__brp__brp_shutdown(app_name="extras_plugin", port=PORT)` for each

2. **Launch 10 parallel apps**: Launch all extras_plugin instances
   - Launch on ports 30001, 30002, 30003, 30004, 30005, 30006, 30007, 30008, 30009, 30010
   - Use `mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=PORT)` for each

3. **Verify ALL launches**: Confirm BRP connectivity on all ports
   - Check ports 30001, 30002, 30003, 30004, 30005, 30006, 30007, 30008, 30009, 30010
   - Use `mcp__brp__brp_status(app_name="extras_plugin", port=PORT)` for each

4. **Set Window Titles**: Main agent sets window titles based on type assignments
   - After determining which types go to which subagent
   - Set window title for each port based on the types that subagent will test
   - Port 30001: `mcp__brp__brp_extras_set_window_title(port=30001, title="[type names for subagent 1]")`
   - Port 30002: `mcp__brp__brp_extras_set_window_title(port=30002, title="[type names for subagent 2]")`
   - Continue for all 10 ports with their assigned type names

Process each batch sequentially, with parallel subagents within each batch:

1. **Identify batch types**: Get all types WITH FULL TYPE GUIDES in current batch (up to BATCH_SIZE types)
   ```bash
   # Get types for batch N - returns COMPLETE type guides with examples
   python3 ./.claude/commands/scripts/mutation_test_get_batch_types.py N
   ```
2. **Divide into groups**: Split types into groups of TYPES_PER_SUBAGENT each
3. **Launch parallel subagents**:
   - Create one Task tool call for EACH group
   - Number of Tasks = ceil(types_in_batch / TYPES_PER_SUBAGENT)
   - Each Task receives exactly TYPES_PER_SUBAGENT types (except possibly the last)
   - **ALL Tasks MUST be sent in a SINGLE message** to run in parallel
4. **Wait for completion**: Wait for ALL subagents to complete before proceeding
5. **Process results**: Collect results and execute merge script

**EXECUTION REQUIREMENT**: Never send Tasks one at a time. Always batch ALL Task calls for the entire batch into a single message with multiple tool invocations to ensure parallel execution.

**Subagent Template**:
```python
Task(
    description="Test [concatenate all short type names with ' + '] (batch [X], subagent [Y]/[MAX_SUBAGENTS])",
    subagent_type="general-purpose",
    prompt="""CRITICAL: You are subagent [Y] assigned to port [30000+Y].

YOUR ASSIGNED PORT: [30000+Y] (e.g., subagent 1 uses 30001, subagent 2 uses 30002, etc.)

DO NOT launch any apps! Use the EXISTING extras_plugin on YOUR assigned port [30000+Y].

Test these types WITH COMPLETE TYPE GUIDES (DO NOT call brp_type_guide - use these provided type guides):
[Include the FULL type guides from mutation_test_get_batch_types.py output - includes type_name, spawn_format, mutation_paths dict with examples, etc.]

[Include ENTIRE TestInstructions section below]

Return structured JSON array with results for ALL assigned types."""
)
```

### Step 4: Individual Type Testing (Subagent Instructions)

<TestInstructions>
⚠️ **CRITICAL - WHAT YOU MUST NOT DO** ⚠️
- **DO NOT launch any apps** - The main agent already launched YOUR extras_plugin on YOUR assigned port
- **DO NOT use brp_launch_bevy_app or brp_launch_bevy_example** - NEVER!
- **DO NOT restart or shutdown apps** - The main agent manages the app lifecycle
- **DO NOT modify test-app/examples/extras_plugin.rs** - Only the main agent does this
- **You are ONLY testing** - Use the EXISTING app on YOUR assigned port

**THE APP IS ALREADY RUNNING**: Your dedicated extras_plugin is running on YOUR assigned port. Use it for ALL tests. If you get connection errors, report them - DO NOT try to fix by launching apps!

⚠️ **WARNING - MOST COMMON FAILURE CAUSE** ⚠️
The #1 reason tests fail is passing numbers as strings in JSON!
- ❌ WRONG: `"value": "42"` or `"value": "3.14"` or `"value": "18446744073709551615"`
- ✅ RIGHT: `"value": 42` or `"value": 3.14` or `"value": 18446744073709551615`
ALL primitive number types (u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64) MUST be JSON numbers!
If you get "invalid type: string" errors, YOU serialized a number wrong. Fix it and retry!

**Your Task**: Test ALL assigned component types with simple pass/fail results. Return structured results array to main agent.

**Port**: Use YOUR ASSIGNED PORT for ALL BRP operations. Your dedicated extras_plugin app was already launched by the main agent on YOUR port. DO NOT launch your own app!

**BEFORE YOU START - CRITICAL CHECKLIST**:
□ I understand that ALL numeric types (f32, u32, i32, usize, etc.) must be JSON numbers, not strings
□ I understand that large numbers like 18446744073709551615 are STILL JSON numbers, not strings
□ I understand that if I get "invalid type: string" errors, it's MY mistake and I must retry with proper types
□ I will NOT mark a type as FAIL on first type error - I will fix my JSON and retry

**CRITICAL**:
- Do NOT update any JSON files
- Test spawn/insert only if `spawn_format` exists in the provided type guide
- Test ALL mutation paths in the `mutation_paths` dict (iterate over all keys)
- Stop testing a type on first failure

**For EACH assigned type**:

1. **Use Provided Type Guide** - DO NOT call `mcp__brp__brp_type_guide` - use the complete type guide provided in your instructions
2. **Test Spawn/Insert** (if supported) - When spawn_format exists in the provided type guide:
   - Test `bevy/spawn` using spawn_format from type guide (creates new entity)
   - Test `bevy/insert` using spawn_format on an existing entity (for validation)
3. **Prepare Mutations** - Query for entity with component by **substituting the actual component type name**:
   ```json
   {
     "filter": {"with": ["ACTUAL_COMPONENT_TYPE_NAME_HERE"]},
     "data": {"components": []}
   }
   ```

   **Example:** For component `bevy_ecs::name::Name`, use:
   ```json
   {
     "filter": {"with": ["bevy_ecs::name::Name"]},
     "data": {"components": []}
   }
   ```

   **CRITICAL:** Replace `ACTUAL_COMPONENT_TYPE_NAME_HERE` with the real component type from your assigned list. Do NOT use the placeholder text literally.
4. **Test Mutations** - Test each path from mutation_paths dict (iterate over the keys):
   - **Root path `""`** (empty string): Full component replacement using the example from mutation_paths[""]
   - **Field paths** (e.g., `.translation.x`): Individual field mutations using the example value for each path
   - Use the `example` value provided in each mutation path entry
   - Skip paths where `path_info.mutation_status` is "not_mutable" or "partially_mutable"

**CRITICAL - Missing Component Detection**:
When testing mutations, if you encounter a "Component not found" or "Unknown component type" error:
1. **Identify the context**: Record which type you were testing and which mutation path caused the error
2. **Extract the missing component**: The error message will indicate which component type is not registered
3. **Include full context in response**: Your result MUST include:
   - `outer_type`: The component you were testing (e.g., "bevy_transform::components::transform::Transform")
   - `mutation_path`: The specific path that failed (e.g., ".rotation" or ".scale.x")
   - `missing_type`: The component type that wasn't found (e.g., "glam::Quat" or "glam::Vec3")
4. **Mark as COMPONENT_NOT_FOUND**: Set status to "COMPONENT_NOT_FOUND" with complete context

5. **Return Results** - Structured JSON for all types

**REMEMBER**:
- You are a subagent - you ONLY test and return results
- The main agent handles ALL app management
- If BRP fails, return error - DO NOT try to fix it yourself

**Return Format**:
```json
[
  {
    "type": "[full::qualified::type::name]",
    "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
    "fail_reason": "Error message or empty string",
    "missing_component_context": {
      "outer_type": "[type being tested when component was missing]",
      "mutation_path": "[mutation path that referenced the missing component]",
      "missing_type": "[the component type that was not found]"
    }
  }
  // ... one entry per assigned type
]
```

**Value Conversion Rules**:
- Integer types: Use reasonable integers (5, 10, 100)
- Float types: Use numeric values (3.14, not "3.14")
- Strings: Use quoted strings
- Enums: Use variant names from examples

**CRITICAL TYPE HANDLING - NUMBERS MUST BE NUMBERS**:
When you get examples from the provided type guide, pay EXTREME attention to the type:
- If the example is a primitive number type (f32, u32, i32, usize, f64, u64, i64, etc.), you MUST pass it as a JSON number
- If the example is a string (like `"example"`), pass it as a JSON string
- **NEVER** convert numbers to strings - this will cause "invalid type: string \"20\", expected u32" errors
- For ALL numeric primitive types (u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64), the value in your mutation call MUST be:
  - CORRECT: `"value": 20` (raw number in JSON)
  - CORRECT: `"value": 3.14` (raw number in JSON)
  - CORRECT: `"value": 18446744073709551615` (raw number in JSON, even for huge numbers!)
  - WRONG: `"value": "20"` (string representation - THIS WILL FAIL)
  - WRONG: `"value": "3.14"` (string representation - THIS WILL FAIL)
  - WRONG: `"value": "18446744073709551615"` (string representation - THIS WILL FAIL)
- When the type guide shows `"example": 20`, this means use the number 20, NOT the string "20"
- **SPECIAL ATTENTION**: Large numbers like `usize::MAX` (18446744073709551615) are STILL numbers, not strings!

**TYPE ERROR RECOVERY**:
If you get "invalid type: string \"X\", expected TYPE" errors:
1. **STOP! DO NOT mark as FAIL yet** - this is YOUR serialization error, not a component issue!
2. You passed a number as a string. Fix it immediately:
   - Remove quotes from ALL numbers: `"3.14"` → `3.14`
   - Remove quotes from ALL booleans: `"false"` → `false`
   - Remove quotes from LARGE numbers: `"18446744073709551615"` → `18446744073709551615`
3. Retry the mutation with corrected JSON number type
4. Only mark FAIL if the retry with proper number types also fails
5. **REMEMBER**: Getting this error means YOU made a mistake, not the component

**MUTATION PATH USAGE**:
- **Root path `""`** (empty string): Replaces the ENTIRE component with a new value
  - **CRITICAL**: Use the EXACT SAME format as spawn_format from the type guide
  - This is essentially the same as spawn/insert but on an existing component
  - Example: For `bevy_ecs::name::Name`, use `{"value": "New Name"}` (the spawn_format structure)
- **Field paths** (e.g., `.translation.x`): Mutates individual fields within the component
  - Use specific values for the field type (numbers for numeric fields, strings for string fields)

**CRITICAL Parameter Formatting**:
- **Empty paths**: For empty paths, use `""` (empty string), NEVER `"\"\""` (quoted string)
- **Parameter ordering**: If you encounter repeated "Unable to extract parameters" errors when calling `mcp__brp__bevy_mutate_component`, try reordering the parameters. The recommended order is: entity, component, path, value, port (with port last)

**Example of CORRECT mutation calls**:
```json
// ROOT PATH ("") - Full component replacement using spawn_format
{
  "entity": 123,
  "component": "bevy_ecs::name::Name",
  "path": "",
  "value": "Entity Name",  // ← Use spawn_format structure from type guide
  "port": YOUR_ASSIGNED_PORT
}

// FIELD PATH - Individual field mutation
{
  "entity": 123,
  "component": "bevy_core_pipeline::core_3d::camera_3d::Camera3d",
  "path": ".depth_texture_usages",
  "value": 20,  // ← NUMBER, not "20" string!
  "port": YOUR_ASSIGNED_PORT
}

// ROOT PATH for complex component - Use full spawn_format structure
{
  "entity": 123,
  "component": "bevy_transform::components::transform::Transform",
  "path": "",
  "value": {
    "translation": {"x": 1.0, "y": 2.0, "z": 3.0},
    "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
    "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
  },  // ← ENTIRE spawn_format structure
  "port": YOUR_ASSIGNED_PORT
}
```
</TestInstructions>

### Step 5: Batch Result Processing

After each batch completes:

1. **Collect results**: Gather all subagent results into a single JSON array
2. **Get temp directory path**: First get the actual temp directory path: `echo $TMPDIR`
3. **Write to temp file**: **MANDATORY** - Use the Write tool to save results array to `{temp_dir}/batch_results_${batch_number}.json`
   - **NEVER use bash commands like `cat >` or `echo >` for writing JSON files**
   - **ALWAYS use the Write tool** - this prevents permission interruptions
   - **CRITICAL**: Use the actual expanded temp directory path, NOT the literal string `$TMPDIR`
4. **Execute merge script**: Run `./.claude/commands/scripts/mutation_test_merge_batch_results.sh {temp_dir}/batch_results_${batch_number}.json {temp_dir}/all_types.json`
5. **Cleanup temp file**: Remove the batch results file after merging: `rm -f {temp_dir}/batch_results_${batch_number}.json`
5. **Handle merge results**:
   - Script exit code 0: All passed, continue to next batch
   - Script exit code 2: Failures detected - **STOP IMMEDIATELY**
   - `COMPONENT_NOT_FOUND` in results: Handle missing component (see Step 6)

**Result Collection Format** (write this exact structure to temp file):
```json
[
  {"type": "full::type::name", "status": "PASS", "fail_reason": ""},
  {"type": "other::type", "status": "FAIL", "fail_reason": "Error message"},
  {
    "type": "third::type",
    "status": "COMPONENT_NOT_FOUND",
    "fail_reason": "Component not registered",
    "missing_component_context": {
      "outer_type": "third::type",
      "mutation_path": ".some_field",
      "missing_type": "missing::component::Type"
    }
  }
]
```

**NO EXCEPTIONS**: The merge script will detect failures and exit with code 2. If this happens, STOP the entire test.

### Step 6: Component Not Found Handling

When subagent returns `COMPONENT_NOT_FOUND` (this is a FAILURE):
1. **STOP IMMEDIATELY** - This is a failure condition
2. **Collect failure details**: Extract from all COMPONENT_NOT_FOUND results:
   - Which outer component types were being tested
   - Which mutation paths referenced missing components
   - The complete list of missing component types
3. **Save progress** for any types that passed before the failure
4. **Report failure to user** with details:
   - "Component `missing::type` not found when testing `outer::type` at mutation path `.field`"
5. **Shutdown ALL apps**:
   - Shutdown apps on ports 30001, 30002, 30003, 30004, 30005, 30006, 30007, 30008, 30009, 30010
6. **DO NOT CONTINUE** to next batch - test stops here

### Step 7: Cleanup

After completion or failure:
- Shutdown ALL apps on ports 30001-30010
- Use `mcp__brp__brp_shutdown(app_name="extras_plugin", port=PORT)` for each port

## Reference Information

### Progress Tracking Schema

**Type guides**: Stored in `{temp_dir}/all_types.json` with COMPLETE type guides including examples
**Progress file**: `{temp_dir}/all_types.json` (where `{temp_dir}` is the actual expanded temp directory path)

File structure contains `type_guide` array at root. Each type entry structure:
```json
{
  "type_name": "bevy_transform::components::transform::Transform",
  "spawn_format": {...},  // Complete spawn example if supported
  "mutation_paths": {
    "": {...},              // Root path with example
    ".translation.x": {...} // Field paths with examples
  },
  "test_status": "untested",
  "batch_number": 1,
  "fail_reason": ""
}
```

### Application Crash Handling

If app crashes during testing:
1. Run `mcp__brp__brp_shutdown` (safety cleanup)
2. Restart application
3. Mark type as failed with reason "App crashed during [operation]"
4. Continue with next type
5. Stop if same type crashes 2+ times

### Success/Failure Criteria

**Success**: ALL types in ALL batches pass their tests (spawn if supported, all mutations)

**Failure**: ANY single type fails = IMMEDIATE STOP
- Save progress for the passed types
- Report which types failed and why
- **DO NOT CONTINUE TO NEXT BATCH**
- Test can be resumed later after fixing issues

### Execution Architecture

- **Main Agent**: Manages batches, launches subagents, collects results, executes merge script, handles app restarts
- **Subagents**: Test assigned types, return structured results (NO JSON updates)
- **Parallelism**: Up to MAX_SUBAGENTS run simultaneously per batch
- **Fast Updates**: Pre-built shell script merges results in milliseconds
