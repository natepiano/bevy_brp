# Type Schema Comprehensive Validation Test

## Objective
Systematically validate ALL BRP component types by testing spawn/insert and mutation operations using individual type schema files. This test tracks progress in `test-app/examples/type_validation.json` to avoid retesting passed types.

**CRITICAL**: Test ALL types in sequence without stopping unless there's an actual failure. NEVER stop for ANY reason including progress updates, successful completions, user checkpoints, summaries, explanations, or demonstrations. The user will manually interrupt if needed. ANY stopping for communication is a test execution failure.

**NOTE**: The extras_plugin app is already running on the specified port - focus on comprehensive type validation.

**IMPORTANT**: For types that only support mutations (no Serialize/Deserialize) and don't exist in the running app:
1. Modify `test-app/examples/extras_plugin.rs` to add the component to an entity
2. Restart the app (auto-builds and launches with the new component)
3. Then test mutations on the newly available component
4. **CONTINUE TO NEXT TYPE** - Do not stop after adding components unless there's a build or test failure

## Schema Source
- **Type schemas**: Retrieved dynamically via `mcp__brp__brp_type_schema` tool
- **Progress tracking**: `test-app/examples/type_validation.json` (single file tracking all types)

## Progress Tracking

The test uses a single JSON file to track testing progress:
- `test-app/examples/type_validation.json`: Contains all types with their current test status

Each type entry has the following structure:

```json
{
  "type": "bevy_transform::components::transform::Transform",
  "spawn_test": "untested",  // "untested", "passed", "failed", "skipped" (if no Serialize/Deserialize)
  "mutation_tests": "untested",  // "untested", "passed", "failed", "n/a" (if no mutations)
  "mutation_paths": {},  // Object with path as key, status as value (e.g., {".translation.x": "passed", ".rotation": "failed"})
  "notes": ""  // Additional notes about failures or special cases
}
```

**IMPORTANT**: When testing mutations, track each individual path result in the `mutation_paths` object. The overall `mutation_tests` field should reflect the aggregate status (all passed = "passed", any failed = "failed", no mutations = "n/a").

When a type is tested, update its status in place:
- **FIRST**: When getting the type schema, immediately update `mutation_paths` with ALL discovered paths set to "untested" (e.g., `{".translation.x": "untested", ".rotation": "untested"}`)
- Change `spawn_test` from "untested" to "passed", "failed", or "skipped"
- Update each path in `mutation_paths` from "untested" to "passed" or "failed" as you test them
- Change `mutation_tests` from "untested" to "passed", "failed", or "n/a" based on aggregate results
- Add any relevant notes about failures or special handling

## Test Strategy

1. **Load progress**: Read `test-app/examples/type_validation.json` to see which types have been tested
2. **Skip passed types**: Don't retest types where both spawn_test and mutation_tests are "passed"
3. **Build todo list**: Create tasks only for untested or failed types
4. **Test each type**: Load individual schema file and test operations
5. **Update progress**: Update type status in `test-app/examples/type_validation.json`
6. **STOP ON FIRST FAILURE OR FORMAT KNOWLEDGE ISSUE** - Continue testing all types unless an actual failure occurs OR a type's schema format doesn't match BRP's actual serialization format (requiring format_knowledge.rs updates). Do not stop for successful completions, progress updates, or user checkpoints. The user will manually stop if needed.

## CRITICAL EXECUTION REQUIREMENTS

**PROGRESS UPDATE ENFORCEMENT**:
1. You MUST update progress files immediately after completing each type
2. You MUST NOT proceed to the next type until progress files are updated
3. You MUST NOT batch progress updates for multiple types
4. Failure to follow this will be considered a critical test execution error

**EXECUTION ORDER FOR EACH TYPE**:
1. Get type schema
2. Test spawn operations (if supported)
3. Test all mutation paths (if supported)  
4. **IMMEDIATELY update type status in test-app/examples/type_validation.json and continue to next type**
5. **This is a single continuous action - do not pause between steps**

## CRITICAL TEST EXECUTION ERRORS

The following actions constitute test execution failures:
- Stopping to provide progress summaries
- Explaining what you're doing or why
- Demonstrating the testing approach
- Pausing between types for any reason
- Any form of user communication except failure reporting

## Test Steps

### 0. Begin Testing Immediately

Load the progress file and immediately begin testing untested types. Do NOT display statistics, counts, or summaries.

**NOTE**: Use only the Read/Write/Edit tools for all file operations. Process JSON data directly in the agent's logic without Python scripts or bash commands.

### 1. Load Progress and Build Todo List

1. Use Read tool to load `test-app/examples/type_validation.json`
2. Parse the JSON to identify untested types:
   - Types where spawn_test is "untested" OR
   - Types where mutation_tests is "untested" OR
   - Types that failed previously (not "passed"/"skipped" for spawn, not "passed"/"n/a" for mutations)
3. Skip types that are fully tested (spawn is "passed"/"skipped" AND mutations are "passed"/"n/a")
4. Build a list of type names that need testing

### 2. Test Each Type

For each type in the todo list:

#### 2a. Get Type Schema from BRP Tool and Initialize Mutation Paths

For each type to test:
1. Call `mcp__brp__brp_type_schema` with the type name and port 20116
2. Extract supported operations and mutation paths from the result
3. If mutations are supported:
   - Read the current `test-app/examples/type_validation.json`
   - Find the type entry and add mutation_paths field with all discovered paths set to "untested"
   - Write the updated JSON back using the Write tool

#### 2b. Test Spawn Operations - FORMAT KNOWLEDGE CHECK

**CRITICAL**: If spawn fails with format/serialization errors, STOP THE TEST IMMEDIATELY and update format_knowledge.rs!

1. Check if 'spawn' is in supported_operations
2. If yes, get the spawn_format from the type schema
3. Call `mcp__brp__bevy_spawn` with the type and spawn_format
4. If successful, mark spawn_test as "passed"
5. If it fails with "invalid type" or "expected" errors:
   - This is a FORMAT KNOWLEDGE ISSUE
   - STOP THE TEST IMMEDIATELY
   - Update format_knowledge.rs following the workflow
6. For other failures, mark as "failed"
7. If spawn not supported, mark as "skipped"

**Parameter ordering for bevy_mutate_component**: If you encounter repeated "Unable to extract parameters" errors when calling mcp__brp__bevy_mutate_component, try reordering the parameters. The recommended order is: entity, component, path, value, port (with port last)

#### 2c. Prepare Entity for Mutation Testing

For types that only support mutation (no spawn/insert):

1. Use `mcp__brp__bevy_query` to check if any entities with this component exist
2. If entity_count is 0:
   - **CRITICAL**: NO EXCEPTIONS - Add ANY component that supports mutations
   - Edit extras_plugin.rs to add the component
        # Determine where to add based on component type:
        # - Camera-related (Bloom, Camera3d, etc): Add to camera spawn
        # - UI-related: Add to UI entity spawns
        # - Render/visibility: Add to visible entities
        # - Transform-related: Add to existing entity with Transform
        # Default: Create new test entity in setup_test_entities()
        
        # 2. Shutdown current app
        mcp__brp__brp_shutdown(app_name="extras_plugin", port=20116)
        
        # 3. Relaunch with updated code (auto-builds before launching)
        mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=20116)
        
        # 4. Set window title
        mcp__brp__brp_extras_set_window_title(
            port=20116, 
            title="type_validation test - port 20116"
        )
        
        # 5. Query again to get entity ID
        query_result = mcp__brp__bevy_query(
            port=20116,
            filter={"with": [type_name]},
            data={"components": [type_name]}
        )
        
        # 6. CRITICAL: Check if component still doesn't exist after restart
        if query_result["metadata"]["entity_count"] == 0:
            # Component addition caused build or runtime error
            # Check the log for errors
            # Common issues:
            # - Missing required components/bundles
            # - Incompatible component combinations
            # - Component requires specific setup/initialization
            # - System pipeline conflicts (e.g., ViewCasPipeline errors)
            
            # Mark as failed and STOP testing
            update_progress(type_name, "skipped", "failed", 
                          "Component causes errors when added - stopping to investigate")
            print(f"ERROR: {type_name} still not found after adding to extras_plugin.rs")
            print("Check log for errors. Common issues:")
            print("- Missing required components")
            print("- Incompatible combinations")
            print("- Pipeline conflicts")
            return "STOP_FOR_COMPONENT_ERROR"
    
    # Get entity ID for mutation testing
    if query_result["metadata"]["entity_count"] > 0:
        entity_id = query_result["result"][0]["entity"]
```

#### 2d. Test All Mutation Paths

If mutations are supported:

1. Get mutation_paths from the type schema
2. For each mutation path:
   - Determine the test value from path_info (example, enum_variants[0], or example_some/none)
   - Call `mcp__brp__bevy_mutate_component` with entity, component, path, value, and port
   - **IMPORTANT**: If you get repeated "Unable to extract parameters" errors, try reordering parameters: entity, component, path, value, port
   - If successful:
     - Update the path status to "passed" in the JSON file
   - If failed:
     - Update the path status to "failed" in the JSON file
     - Stop testing this type and move to the next
3. After all paths tested:
   - If all passed, set mutation_tests to "passed"
   - If any failed, set mutation_tests to "failed"
   - If no paths exist, set mutation_tests to "n/a"

### 3. Update Progress - MANDATORY AFTER EACH TYPE

**CRITICAL**: IMMEDIATELY after completing ALL tests for a type (spawn + all mutations), you MUST update the progress file and seamlessly continue to the next type in one continuous flow. Do not pause or wait between these actions.

After testing each type:
1. **IMMEDIATELY update the type's status in test-app/examples/type_validation.json**
2. **IMMEDIATELY continue to the next type without pausing**

**FAILURE TO UPDATE PROGRESS IMMEDIATELY WILL BE CONSIDERED A TEST EXECUTION ERROR**

**IMPORTANT**: 
- Do NOT create backup files (.bak or similar) when updating these JSON files. The files are already under source control (git), which provides version history and backup functionality.
- Do NOT use bash commands like jq with redirects/pipes to edit JSON files - use Read/Write or Edit tools instead
- Always use proper file editing tools (Read/Write, Edit, MultiEdit) to update JSON files

### Progress Update Implementation

**CRITICAL**: Use the Read and Write tools, NOT bash commands, to update the JSON files. This ensures the test can run unattended without requiring user approval for file modifications.

For updating type status after testing:
1. Use Read tool to load the current JSON file
2. Parse the JSON data and find the entry to update
3. Modify the entry with new status/paths/notes
4. Use Write tool to save the updated JSON back to the file

Example workflow for updating a type's status:
1. Use Read tool to get the current content of `test-app/examples/type_validation.json`
2. Parse the JSON data internally
3. Find the entry matching the type name and update:
   - spawn_test: "passed", "failed", or "skipped"
   - mutation_tests: "passed", "failed", or "n/a"
   - mutation_paths: Object with path statuses
   - notes: Any relevant notes
4. Use Write tool to save the complete updated JSON back to the file

**NEVER use bash commands like:**
- `jq 'map(...)' file.json > /tmp/updated.json && mv /tmp/updated.json file.json`
- `jq ... file.json | tee file.json`
- Any command involving `>`, `>>`, or file redirection that requires approval

**ALWAYS use file editing tools that don't require approval:**
- Read tool to get file contents
- Write tool to save complete updated contents
- Edit tool for specific string replacements
- MultiEdit for multiple changes in one operation

### 4. Progress File Updates vs Progress Reporting

**MANDATORY PROGRESS FILE UPDATES**: After each type is completed, you MUST immediately update the JSON file:
- Update the type's status in `test-app/examples/type_validation.json`

**ABSOLUTE PROHIBITION ON USER COMMUNICATION**: Do NOT stop, pause, or communicate ANYTHING to the user including summaries, progress reports, status updates, explanations, demonstrations, or any other form of communication. Silent execution only. ANY communication is a critical test execution error.

**DISTINCTION**: 
- JSON file updates = REQUIRED as part of continuous flow to next type
- User progress reports = FORBIDDEN
- Pausing between types = FORBIDDEN


## Success Criteria

✅ Test passes when:
- All untested types are validated (test ALL types in sequence without stopping)
- Spawn/insert operations work for supported types
- All mutation paths work for supported types
- Progress is saved after each type (but continue to next type immediately)

**IMPORTANT**: The test executor should process ALL types in types-all.json sequentially without stopping unless there's an actual failure. User interruption is their choice, not the executor's responsibility.

## Failure Handling

**On failure**:

### Standard Failure (bugs, missing components, etc.)
1. Mark type as "failed" in test-app/examples/type_validation.json
2. Record failure details:
   - Operation that failed (spawn/insert/mutate)
   - Error message
   - Path (for mutations)
3. **STOP TESTING** - Only stop when there's an actual failure
4. Save progress so test can resume later if stopped due to failure

### Special Case: Invalid Example Values Causing Crashes

When a type's generated example value causes crashes (e.g., wgpu validation errors, invalid enum variants, etc.):

#### Step 1: Investigate Valid Values
1. Search the Bevy codebase (`/Users/natemccoy/rust/bevy/`) for:
   - How the type is defined
   - Valid enum variants or flag combinations
   - Default implementations
   - Usage examples in the codebase
2. For bitflags types, identify:
   - Individual flag values and their meanings
   - Which combinations are valid/invalid
   - Hardware or API restrictions (e.g., STORAGE_BINDING incompatible with multisampled textures)

#### Step 2: Update format_knowledge.rs
1. Add ONLY the problematic type to `mcp/src/brp_tools/brp_type_schema/format_knowledge.rs`
2. Provide a safe, valid example value that won't cause crashes
3. Document WHY this value is needed (what restriction it avoids)
4. Example:
```rust
// Camera3dDepthTextureUsage - wrapper around u32 texture usage flags
// Valid flags: COPY_SRC=1, COPY_DST=2, TEXTURE_BINDING=4, STORAGE_BINDING=8, RENDER_ATTACHMENT=16
// STORAGE_BINDING (8) causes crashes with multisampled textures!
// Safe combinations: 16 (RENDER_ATTACHMENT only), 20 (RENDER_ATTACHMENT | TEXTURE_BINDING)
map.insert(
    "bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthTextureUsage".into(),
    BrpFormatKnowledge {
        example_value:  json!(20), // RENDER_ATTACHMENT | TEXTURE_BINDING - safe combination
        subfield_paths: None,
    },
);
```

#### Step 3: Stop for MCP Tool Reinstall
1. Save the current test state in test-app/examples/type_validation.json with a note about the format_knowledge update
2. **STOP THE TEST** and inform the user:
   - Format knowledge has been updated for [TYPE_NAME]
   - User needs to exit and reinstall the MCP tool for changes to take effect
   - After reinstall, the type schema tool will automatically use the new format knowledge

#### Step 4: Resume After Reinstall (User re-runs test)
1. Detect that format_knowledge was updated for a type (check notes field in test-app/examples/type_validation.json)
2. The BRP type schema tool will now automatically provide the safe example value from format_knowledge
3. Resume testing from where it left off

## CRITICAL: No Component Exceptions

**There are NO exceptions for any component types.** If a component:
1. Supports mutations (`'mutate' in supported_ops`)
2. Has no existing entities (`entity_count == 0`)

Then it MUST be added to extras_plugin.rs regardless of assumptions about:
- Whether it's "computed" 
- Whether it's "system-managed"
- Whether it "should" be manually added
- Any other reasoning

**The protocol is absolute: Test ALL mutation-capable components.**

## Known Issues - STOPPING CONDITIONS

**CRITICAL**: Any of these issues require IMMEDIATE test stopping and format_knowledge.rs updates:

Types that require special handling and will cause **IMMEDIATE TEST STOPPING**:
1. **Schema format mismatch**: When BRP rejects a format that the schema tool generated (like GlobalTransform expecting flat array vs nested object)
2. **bevy_ecs::name::Name**: Schema shows struct but BRP expects string
3. **Option fields**: Some types use "None" string vs null
4. **Handle types**: May have complex serialization
5. **Math types**: May serialize as arrays vs objects (like Vec3, GlobalTransform)

**When you encounter ANY schema format error from BRP, STOP IMMEDIATELY and follow the format knowledge workflow.**

These should be marked with special handling in the test logic, but more importantly, **they require stopping the test to update format_knowledge.rs**.

## Resume Capability

The test can be resumed at any time:
1. Previously passed types are skipped
2. Failed types can be retried 
3. Untested types are processed in order

**IMPORTANT**: Resume capability exists for when tests are stopped due to failures or manual user intervention. The test executor should NOT proactively stop for checkpoints, progress reports, or successful completions. Process all types continuously unless an actual failure occurs.

This allows incremental testing and debugging of specific type issues when failures occur.