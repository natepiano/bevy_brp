# Type Schema Comprehensive Validation Test

## Objective
Systematically validate ALL BRP component types by testing spawn/insert and mutation operations using individual type schema files. This test tracks progress in `test-app/examples/type_validation.json` to avoid retesting passed types.

**CRITICAL**: Test ALL types in sequence without stopping unless there's an actual failure. Do not stop for progress updates, successful completions, or user checkpoints. The user will manually interrupt if needed.

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

## Test Steps

### 0. Display Progress Statistics

First, display the current test progress statistics using jq:

```bash
# Display test statistics
jq -r '
  def count_tested: 
    map(select(
      (.spawn_test == "passed" or .spawn_test == "skipped") and 
      (.mutation_tests == "passed" or .mutation_tests == "n/a")
    )) | length;
  
  def count_untested:
    map(select(
      (.spawn_test == "untested" or .mutation_tests == "untested") or
      ((.spawn_test != "passed" and .spawn_test != "skipped") or 
       (.mutation_tests != "passed" and .mutation_tests != "n/a"))
    )) | length;
  
  "Total types: \(length)",
  "Tested types: \(count_tested)",
  "Untested types: \(count_untested)"
' test-app/examples/type_validation.json
```

### 1. Load Progress and Build Todo List

```python
import json
import os

# Load current progress
with open('test-app/examples/type_validation.json', 'r') as f:
    all_types = json.load(f)

# Build todo list of untested types
todo_types = []
for type_entry in all_types:
    # Test if type is not fully passed
    if type_entry["spawn_test"] != "passed" or type_entry["mutation_tests"] != "passed":
        # Skip types that are n/a for mutations if spawn is passed/skipped
        if type_entry["spawn_test"] in ["passed", "skipped"] and type_entry["mutation_tests"] == "n/a":
            continue  # This type is fully tested
        todo_types.append(type_entry["type"])

print(f"Types to test: {len(todo_types)}")
print(f"Already passed: {len(all_types) - len(todo_types)}")
```

### 2. Test Each Type

For each type in the todo list:

#### 2a. Get Type Schema from BRP Tool and Initialize Mutation Paths
```python
type_name = todo_types[0]  # Process one at a time

# Get type schema using the BRP type schema tool
schema_result = mcp__brp__brp_type_schema(
    types=[type_name],
    port=20116  # Use the test port
)

# Extract the schema for this type
type_schema = schema_result['types'][type_name]
supported_ops = type_schema.get('supported_operations', [])

# IMMEDIATELY update the JSON file with discovered mutation paths set to "untested"
if 'mutate' in supported_ops:
    mutation_paths = type_schema.get('mutation_paths', {})
    
    # Load the progress file
    with open('test-app/examples/type_validation.json', 'r') as f:
        all_types = json.load(f)
    
    # Find and update the type entry with mutation paths
    for i, entry in enumerate(all_types):
        if entry["type"] == type_name:
            # Initialize mutation_paths with all paths set to "untested"
            all_types[i]["mutation_paths"] = {path: "untested" for path in mutation_paths.keys()}
            break
    
    # Write back immediately
    with open('test-app/examples/type_validation.json', 'w') as f:
        json.dump(all_types, f, indent=2)
```

#### 2b. Test Spawn Operations - FORMAT KNOWLEDGE CHECK

**CRITICAL**: If spawn fails with format/serialization errors, STOP THE TEST IMMEDIATELY and update format_knowledge.rs!

```python
test_result = {
    "type": type_name,
    "spawn_test": "Skipped",  # Default if no Serialize/Deserialize
    "mutation_paths": []
}

if 'spawn' in supported_ops:
    spawn_format = type_schema.get('spawn_format')
    try:
        # Execute mcp__brp__bevy_spawn with spawn_format
        result = mcp__brp__bevy_spawn(components={type_name: spawn_format})
        test_result["spawn_test"] = "Passed"
    except Exception as e:
        if "invalid type" in str(e) or "expected" in str(e):
            # FORMAT KNOWLEDGE ISSUE - STOP TEST IMMEDIATELY
            # Follow Step 2: Update format_knowledge.rs workflow
            return "STOP_FOR_FORMAT_KNOWLEDGE"
        else:
            test_result["spawn_test"] = "Failed"
```

**Parameter ordering for bevy_mutate_component**: If you encounter repeated "Unable to extract parameters" errors when calling mcp__brp__bevy_mutate_component, try reordering the parameters. The recommended order is: entity, component, path, value, port (with port last)

#### 2c. Prepare Entity for Mutation Testing
```python
# For types that only support mutation (no spawn/insert)
if 'mutate' in supported_ops and 'spawn' not in supported_ops:
    # Query to check if any entities with this component exist
    query_result = mcp__brp__bevy_query(
        port=20116,
        filter={"with": [type_name]},
        data={"components": [type_name]}
    )
    
    if query_result["metadata"]["entity_count"] == 0:
        # No entities exist - need to add to extras_plugin.rs
        # CRITICAL: NO EXCEPTIONS - Add ANY component that supports mutations
        # regardless of whether it appears to be:
        # - Computed by systems
        # - Auto-generated  
        # - "System-managed"
        # - "Not meant to be manually added"
        # The test protocol requires testing ALL mutation-capable components
        print(f"No entities with {type_name} found. Adding to extras_plugin.rs...")
        
        # 1. Edit extras_plugin.rs to add the component
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
```python
if 'mutate' in supported_ops:
    mutation_paths = type_schema.get('mutation_paths', {})
    
    # Create todo items for each mutation path
    mutation_todos = []
    for path in mutation_paths.keys():
        mutation_todos.append({
            "content": f"Test mutation path {path}",
            "status": "pending",
            "activeForm": f"Testing mutation path {path}"
        })
    
    # Add mutation paths to TodoWrite tool
    TodoWrite(todos=mutation_todos)
    
    # Track overall success for aggregate status
    all_mutations_passed = True
    
    # Now test each mutation path
    for i, path in enumerate(mutation_paths.keys()):
        path_info = mutation_paths[path]
        
        # Mark current mutation as in_progress in todo list
        mutation_todos[i]["status"] = "in_progress"
        TodoWrite(todos=mutation_todos)
        
        # Determine value to use
        if 'example' in path_info:
            value = path_info['example']
        elif 'enum_variants' in path_info:
            value = path_info['enum_variants'][0]
        elif 'example_some' in path_info:
            # Test both Some and None
            test_values = [path_info['example_some'], path_info['example_none']]
        
        # Execute mcp__brp__bevy_mutate_component
        # IMPORTANT: If you get repeated "Unable to extract parameters" errors,
        # try reordering the parameters. The recommended order is:
        # entity, component, path, value, port
        try:
            result = mcp__brp__bevy_mutate_component(
                entity=entity_id,
                component=type_name,
                path=path,
                value=value,
                port=port
            )
            # Update this specific path to "passed" in the JSON file
            update_mutation_path_status(type_name, path, "passed")
            mutation_todos[i]["status"] = "completed"
        except Exception as e:
            # Update this specific path to "failed" in the JSON file
            update_mutation_path_status(type_name, path, "failed")
            all_mutations_passed = False
            mutation_todos[i]["status"] = "completed"
            
            # Update todo list before stopping
            TodoWrite(todos=mutation_todos)
            
            # Stop on first failure
            print(f"FAILURE: Mutation path {path} failed with error: {e}")
            break
        
        # Update todo list after each successful test
        TodoWrite(todos=mutation_todos)
    
    # Update the aggregate mutation_tests status based on all path results
    if all_mutations_passed:
        update_aggregate_mutation_status(type_name, "passed")
    else:
        update_aggregate_mutation_status(type_name, "failed")
```

### 3. Update Progress - MANDATORY AFTER EACH TYPE

**CRITICAL**: IMMEDIATELY after completing ALL tests for a type (spawn + all mutations), you MUST update the progress file and seamlessly continue to the next type in one continuous flow. Do not pause or wait between these actions.

After testing each type:
1. **IMMEDIATELY update the type's status in test-app/examples/type_validation.json**
2. **IMMEDIATELY continue to the next type without pausing**

**FAILURE TO UPDATE PROGRESS IMMEDIATELY WILL BE CONSIDERED A TEST EXECUTION ERROR**

**IMPORTANT**: Do NOT create backup files (.bak or similar) when updating these JSON files. The files are already under source control (git), which provides version history and backup functionality.

```python
def update_mutation_path_status(type_name, path, status):
    """Update the status of a specific mutation path immediately after testing."""
    # Load the progress file
    with open('test-app/examples/type_validation.json', 'r') as f:
        all_types = json.load(f)
    
    # Find and update the specific path
    for i, entry in enumerate(all_types):
        if entry["type"] == type_name:
            if "mutation_paths" not in all_types[i]:
                all_types[i]["mutation_paths"] = {}
            all_types[i]["mutation_paths"][path] = status
            break
    
    # Write back immediately
    with open('test-app/examples/type_validation.json', 'w') as f:
        json.dump(all_types, f, indent=2)

def update_aggregate_mutation_status(type_name, status):
    """Update the aggregate mutation_tests status based on all path results."""
    # Load the progress file
    with open('test-app/examples/type_validation.json', 'r') as f:
        all_types = json.load(f)
    
    # Find and update the aggregate status
    for i, entry in enumerate(all_types):
        if entry["type"] == type_name:
            all_types[i]["mutation_tests"] = status
            break
    
    # Write back immediately
    with open('test-app/examples/type_validation.json', 'w') as f:
        json.dump(all_types, f, indent=2)

def update_progress(type_name, spawn_result, mutation_result, notes=""):
    # Load the progress file
    # DO NOT create backup files - git provides version control
    with open('.claude/commands/type_validation.json', 'r') as f:
        all_types = json.load(f)
    
    # Find and update the type entry
    for i, entry in enumerate(all_types):
        if entry["type"] == type_name:
            # Update spawn test status
            if spawn_result is not None:
                all_types[i]["spawn_test"] = spawn_result  # "passed", "failed", or "skipped"
            
            # Update mutation test status (this should already be set by update_aggregate_mutation_status)
            if mutation_result is not None:
                all_types[i]["mutation_tests"] = mutation_result  # "passed", "failed", or "n/a"
            
            # Add any notes
            if notes:
                all_types[i]["notes"] = notes
            
            break
    
    # Write the updated progress back
    with open('test-app/examples/type_validation.json', 'w') as f:
        json.dump(all_types, f, indent=2)
```

### 4. Progress File Updates vs Progress Reporting

**MANDATORY PROGRESS FILE UPDATES**: After each type is completed, you MUST immediately update the JSON file:
- Update the type's status in `test-app/examples/type_validation.json`

**NO PROGRESS REPORTING TO USER**: Do NOT stop to provide summaries, progress reports, or status updates to the user. The JSON file updates are mandatory, but user communication about progress is forbidden.

**DISTINCTION**: 
- JSON file updates = REQUIRED as part of continuous flow to next type
- User progress reports = FORBIDDEN
- Pausing between types = FORBIDDEN

```
Testing Progress:
- Total types: 101
- Passed: X
- Failed: Y  
- Remaining: Z

Current type: [TYPE_NAME]
- Spawn test: [PASS/FAIL/SKIP]
- Insert test: [PASS/FAIL/SKIP]
- Mutation paths: [X/Y passed]
```

**This format is ONLY for internal logging. NEVER stop the test to show this to the user.**

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