# Type Schema Comprehensive Validation Test

## Usage
`/type_validation` - Validates ALL BRP component types by testing spawn/insert and mutation operations.

## Execution Instructions

1. **Check App Status**: Use `brp_status` to check if extras_plugin is running on port 20116
   - If running with BRP responding: Skip to step 5
   - If not running or BRP not responding: Continue with step 2
2. **Launch App**: `mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=20116)`
3. **Verify Launch**: Use `brp_status` to confirm BRP connectivity on port 20116
4. **Set Window Title**: `mcp__brp__brp_extras_set_window_title(port=20116, title="type_validation test - port 20116")`
5. **Execute Test**: 
   - Execute all test procedures below
   - Use parallel subagents for type testing within batches
   - Continue testing ALL types sequentially
   - Only stop on failure or user interruption
6. **Cleanup**: Shutdown app using `mcp__brp__brp_shutdown(app_name="extras_plugin", port=20116)` after completion or failure

## Configuration Constants
```
BATCH_SIZE = 30              # Types per batch
MAX_SUBAGENTS = 10           # Parallel subagents per batch  
TYPES_PER_SUBAGENT = 3       # Types each subagent tests
```

## Objective
Systematically validate ALL BRP component types by testing spawn/insert and mutation operations. This test tracks progress in `test-app/tests/type_validation.json` with simple pass/fail status for each type.

## Critical Execution Requirements

1. **ALWAYS reassign batch numbers** - Clear and reassign every run
2. **ALWAYS use parallel subagents** - Launch MAX_SUBAGENTS in parallel per batch
3. **Main agent orchestrates, subagents test** - Main agent never tests directly
4. **STOP ON ANY FAILURE** - If ANY type fails in a batch, STOP IMMEDIATELY. Do not continue to next batch.
5. **Simple pass/fail per type** - One overall result per type

**FAILURE MEANS STOP**: When any subagent reports a FAIL status, the entire test suite MUST stop. Save progress and report the failure to the user. DO NOT CONTINUE testing subsequent batches.

## Execution Architecture

- **Main Agent**: Manages batches, launches subagents, collects results, performs atomic JSON updates, handles app restarts
- **Subagents**: Test assigned types, return structured results (NO JSON updates)
- **Parallelism**: Up to MAX_SUBAGENTS run simultaneously per batch

## Schema and Progress Tracking

- **Type schemas**: Retrieved via `mcp__brp__brp_type_schema` 
- **Progress file**: `test-app/tests/type_validation.json`

Each type entry structure:
```json
{
  "type": "bevy_transform::components::transform::Transform",
  "spawn_support": "supported",  
  "mutation_paths": [".translation.x", ".rotation", ".scale"],  
  "test_status": "untested",  
  "batch_number": 1,
  "fail_reason": ""  
}
```

## Test Execution Steps

### 1. Load Progress and Reassign Batch Numbers (MANDATORY)

Always clear and reassign batch numbers using the renumbering script:

```bash
./test-app/tests/renumber_batches.sh
```

This script will:
- Clear all existing batch numbers
- Assign new batch numbers to untested/failed types (BATCH_SIZE=30 per batch)
- Display statistics about types to be tested

### 2. Test Types Using Parallel Subagents

Process each batch sequentially, with parallel subagents within each batch:

1. Identify all types in current batch (up to BATCH_SIZE types)
2. Divide types into groups of TYPES_PER_SUBAGENT each
3. **CRITICAL**: Create one Task tool call for EACH group
   - Number of Tasks = ceil(types_in_batch / TYPES_PER_SUBAGENT)
   - Each Task receives exactly TYPES_PER_SUBAGENT types (except possibly the last)
   - **ALL Tasks MUST be sent in a SINGLE message** to run in parallel
4. Wait for ALL subagents to complete before proceeding
5. Process results and update JSON atomically

**EXECUTION REQUIREMENT**: Never send Tasks one at a time. Always batch ALL Task calls for the entire batch into a single message with multiple tool invocations to ensure parallel execution.

#### Parallel Subagent Template

```python
Task(
    description="Test [concatenate all short type names with ' + '] (batch [X], subagent [Y]/[MAX_SUBAGENTS])",
    subagent_type="general-purpose", 
    prompt="""Test these types: 
[List all assigned full::qualified::type::names here, one per line]

[Include ENTIRE TestInstructions section]

Return structured JSON array with results for ALL assigned types."""
)
```

### 3. Individual Type Testing (Subagent Instructions)

<TestInstructions>
**Your Task**: Test ALL assigned component types with simple pass/fail results. Return structured results array to main agent.

**Port**: Use port 20116 for ALL BRP operations.

**CRITICAL**: 
- Do NOT update any JSON files
- Test spawn only if `spawn_support` is "supported"
- Test ALL mutation paths in the `mutation_paths` array
- Stop testing a type on first failure

**For EACH assigned type**:

1. **Get Type Schema** - Call `mcp__brp__brp_type_schema`
2. **Test Spawn** (if supported) - Use spawn_format from schema
3. **Prepare Mutations** - Query for entity with component
4. **Test Mutations** - Test each path from mutation_paths array
5. **Return Results** - Structured JSON for all types

**Return Format**:
```json
[
  {
    "type": "[full::qualified::type::name]",
    "status": "PASS|FAIL|COMPONENT_NOT_FOUND",
    "fail_reason": "Error message or empty string"
  }
  // ... one entry per assigned type
]
```

**Value Conversion Rules**:
- Integer types: Use reasonable integers (5, 10, 100)
- Float types: Use numeric values (3.14, not "3.14")
- Strings: Use quoted strings
- Enums: Use variant names from examples
- Skip paths with `path_kind: "NotMutatable"`

**CRITICAL TYPE HANDLING - NUMBERS MUST BE NUMBERS**:
When you get examples from `brp_type_schema`, pay EXTREME attention to the type:
- If the example is a number (like `20` or `3.14`), you MUST pass it as a JSON number
- If the example is a string (like `"example"`), pass it as a JSON string
- **NEVER** convert numbers to strings - this will cause "invalid type: string \"20\", expected u32" errors
- For numeric types (u32, usize, f32, etc.), the value in your mutation call MUST be:
  - CORRECT: `"value": 20` (raw number in JSON)
  - WRONG: `"value": "20"` (string representation - THIS WILL FAIL)
- When the schema shows `"example": 20`, this means use the number 20, NOT the string "20"

**CRITICAL Parameter Formatting**:
- **Empty paths**: For empty paths, use `""` (empty string), NEVER `"\"\""` (quoted string)
- **Parameter ordering**: If you encounter repeated "Unable to extract parameters" errors when calling `mcp__brp__bevy_mutate_component`, try reordering the parameters. The recommended order is: entity, component, path, value, port (with port last)

**Example of CORRECT mutation calls**:
```json
// For a u32 field - use raw number
{
  "entity": 123,
  "component": "bevy_core_pipeline::core_3d::camera_3d::Camera3d",
  "path": ".depth_texture_usages",
  "value": 20,  // ← NUMBER, not "20" string!
  "port": 20116
}

// For a string field - use quoted string
{
  "entity": 123,
  "component": "bevy_ecs::name::Name",
  "path": "",
  "value": "Entity Name",  // ← STRING is correct here
  "port": 20116
}
```
</TestInstructions>

### 4. Batch Result Processing (Main Agent)

After each batch completes:
1. Collect all subagent results
2. Update JSON atomically using Read/Write tools (save passed types first)
3. **CRITICAL FAILURE HANDLING**:
   - `COMPONENT_NOT_FOUND`: Add component to extras_plugin.rs, restart app, retry batch
   - `FAIL`: **STOP IMMEDIATELY** - Do NOT continue to next batch. Report failures and exit.
   - `PASS` (ALL types): Only if ALL types pass, continue to next batch

**NO EXCEPTIONS**: If even ONE type fails, STOP the entire test. Do not rationalize continuing.

## Component Not Found Handling

When subagent returns `COMPONENT_NOT_FOUND`:
1. Stop testing
2. Add missing component to `test-app/examples/extras_plugin.rs`
3. Shutdown app: `mcp__brp__brp_shutdown(app_name="extras_plugin", port=20116)`
4. Relaunch: `mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=20116)`
5. Reset title: `mcp__brp__brp_extras_set_window_title(port=20116, title="type_validation test - port 20116")`
6. Retry SAME batch

## Application Crash Handling

If app crashes during testing:
1. Run `mcp__brp__brp_shutdown` (safety cleanup)
2. Restart application
3. Mark type as failed with reason "App crashed during [operation]"
4. Continue with next type
5. Stop if same type crashes 2+ times

## Success/Failure Criteria

**Success**: ALL types in ALL batches pass their tests (spawn if supported, all mutations)

**Failure**: ANY single type fails = IMMEDIATE STOP
- Save progress for the passed types
- Report which types failed and why
- **DO NOT CONTINUE TO NEXT BATCH**
- Test can be resumed later after fixing issues

## Key Principles

- **No exceptions**: Test ALL mutation-capable components
- **Atomic updates**: Only main agent updates JSON, once per batch
- **Continuous execution**: Don't stop unless actual failure
- **Resume capability**: Can restart from any saved state