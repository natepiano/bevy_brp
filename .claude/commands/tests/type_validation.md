# Type Schema Comprehensive Validation Test

## Objective
Systematically validate all BRP component types by testing spawn/insert and mutation operations using individual type schema files. This test tracks progress in `types-all.json` to avoid retesting passed types.

**NOTE**: The extras_plugin app is already running on the specified port - focus on comprehensive type validation.

**IMPORTANT**: For types that only support mutations (no Serialize/Deserialize) and don't exist in the running app:
1. Modify `test-app/examples/extras_plugin.rs` to add the component to an entity
2. Build the modified example
3. Restart the app with the new component
4. Then test mutations on the newly available component

## Schema Files Location
- **Individual type schemas**: `.claude/commands/tests/type_schemas/{type_name}.json`
- **Progress tracking (all types)**: `types-all.json` (project root)
- **Progress tracking (passed)**: `types-passed.json` (project root)

## Progress Tracking

The test uses two files to track testing progress:
- `types-all.json`: Types still to be tested or with failures (project root)
- `types-passed.json`: Types that fully passed all tests (project root)

When testing a type, track detailed results:

```json
{
  "type": "bevy_transform::components::transform::Transform",
  "spawn_test": "Passed",  // or "Failed", "Skipped" (if no Serialize/Deserialize)
  "mutation_paths": [
    {"name": ".translation", "status": "Passed"},
    {"name": ".translation.x", "status": "Passed"},
    {"name": ".rotation", "status": "Failed"}
  ]
}
```

**IMPORTANT**: The mutation_paths array must be created upfront with ALL paths from the schema marked as "Untested" before testing begins. Each path is then tested and updated to "Passed" or "Failed". This ensures no mutation path is accidentally skipped.

When a type fully passes (spawn test passed/skipped AND all mutation paths passed):
- Remove it from `types-all.json`
- Add it to `types-passed.json` with its complete test results

## Test Strategy

1. **Load progress**: Read `types-all.json` to see which types have been tested
2. **Skip passed types**: Don't retest types marked as "Passed"
3. **Build todo list**: Create tasks only for untested or failed types
4. **Test each type**: Load individual schema file and test operations
5. **Update progress**: Mark types as "Passed" or "Failed" in `types-all.json`
6. **STOP ON FIRST FAILURE** for immediate issue identification

## Test Steps

### 1. Load Progress and Build Todo List

```python
import json
import os

# Load current progress
with open('types-all.json', 'r') as f:
    all_types = json.load(f)

# Convert to dict format if still an array
if isinstance(all_types, list) and all(isinstance(t, str) for t in all_types):
    all_types = [{"type": t} for t in all_types]

# Build todo list of untested types
todo_types = []
for type_entry in all_types:
    if isinstance(type_entry, dict):
        status = type_entry.get("status")
        if status != "Passed":  # Test if no status or Failed
            todo_types.append(type_entry["type"])
    else:  # Legacy format
        todo_types.append(type_entry)

print(f"Types to test: {len(todo_types)}")
print(f"Already passed: {len([t for t in all_types if isinstance(t, dict) and t.get('status') == 'Passed'])}")
```

### 2. Test Each Type

For each type in the todo list:

#### 2a. Load Type Schema
```python
type_name = todo_types[0]  # Process one at a time
schema_file = f'.claude/commands/tests/type_schemas/{type_name}.json'

with open(schema_file, 'r') as f:
    type_schema = json.load(f)

supported_ops = type_schema.get('supported_operations', [])
```

#### 2b. Test Spawn Operations
```python
test_result = {
    "type": type_name,
    "spawn_test": "Skipped",  # Default if no Serialize/Deserialize
    "mutation_paths": []
}

if 'spawn' in supported_ops:
    spawn_format = type_schema.get('spawn_format')
    # Execute mcp__brp__bevy_spawn with spawn_format
    # Record entity ID if successful
    test_result["spawn_test"] = "Passed"  # or "Failed" if error
```

**KNOWN ISSUES to handle**:
- `bevy_ecs::name::Name`: Use plain string instead of struct format
- Option fields: Use "None" string instead of null

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
        print(f"No entities with {type_name} found. Adding to extras_plugin.rs...")
        
        # 1. Edit extras_plugin.rs to add the component
        # Determine where to add based on component type:
        # - Camera-related (Bloom, Camera3d, etc): Add to camera spawn
        # - UI-related: Add to UI entity spawns
        # - Render/visibility: Add to visible entities
        # - Transform-related: Add to existing entity with Transform
        # Default: Create new test entity in setup_test_entities()
        
        # 2. Build the modified example
        result = bash("cargo build --example extras_plugin")
        if result.returncode != 0:
            test_result["spawn_test"] = "Failed"
            test_result["error"] = f"Failed to build after adding {type_name}"
            return test_result
        
        # 3. Shutdown current app
        mcp__brp__brp_shutdown(app_name="extras_plugin", port=20116)
        
        # 4. Relaunch with updated code
        mcp__brp__brp_launch_bevy_example(example_name="extras_plugin", port=20116)
        
        # 5. Set window title
        mcp__brp__brp_extras_set_window_title(
            port=20116, 
            title="type_validation test - port 20116"
        )
        
        # 6. Query again to get entity ID
        query_result = mcp__brp__bevy_query(
            port=20116,
            filter={"with": [type_name]},
            data={"components": [type_name]}
        )
    
    # Get entity ID for mutation testing
    if query_result["metadata"]["entity_count"] > 0:
        entity_id = query_result["result"][0]["entity"]
```

#### 2d. Test All Mutation Paths
```python
if 'mutate' in supported_ops:
    mutation_paths = type_schema.get('mutation_paths', {})
    
    # FIRST: Create all mutation_paths with "Untested" status
    for path in mutation_paths.keys():
        test_result["mutation_paths"].append({"name": path, "status": "Untested"})
    
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
        try:
            result = mcp__brp__bevy_mutate_component(
                port=port,
                entity=entity_id,
                component=type_name,
                path=path,
                value=value
            )
            # Update status to "Passed" if successful
            test_result["mutation_paths"][i]["status"] = "Passed"
            mutation_todos[i]["status"] = "completed"
        except Exception as e:
            # Update status to "Failed" on error
            test_result["mutation_paths"][i]["status"] = "Failed"
            test_result["mutation_paths"][i]["error"] = str(e)
            mutation_todos[i]["status"] = "completed"
            
            # Update todo list before stopping
            TodoWrite(todos=mutation_todos)
            
            # Stop on first failure
            print(f"FAILURE: Mutation path {path} failed with error: {e}")
            break
        
        # Update todo list after each successful test
        TodoWrite(todos=mutation_todos)
```

### 3. Update Progress

After testing each type:

```python
def update_progress(test_result):
    type_name = test_result["type"]
    
    # Check if fully passed
    spawn_ok = test_result["spawn_test"] in ["Passed", "Skipped"]
    mutations_ok = all(m["status"] == "Passed" for m in test_result["mutation_paths"])
    
    if spawn_ok and mutations_ok:
        # Move to types-passed.json
        # Load or create types-passed.json
        try:
            with open('types-passed.json', 'r') as f:
                passed_types = json.load(f)
        except FileNotFoundError:
            passed_types = []
        
        passed_types.append(test_result)
        
        with open('types-passed.json', 'w') as f:
            json.dump(passed_types, f, indent=2)
        
        # Remove from types-all.json
        with open('types-all.json', 'r') as f:
            all_types = json.load(f)
        
        all_types = [t for t in all_types if t.get("type") != type_name]
        
        with open('types-all.json', 'w') as f:
            json.dump(all_types, f, indent=2)
    else:
        # Keep in types-all.json with failure details
        with open('types-all.json', 'r') as f:
            all_types = json.load(f)
        
        # Update or add the test result
        found = False
        for i, entry in enumerate(all_types):
            if entry.get("type") == type_name:
                all_types[i] = test_result
                found = True
                break
        
        if not found:
            all_types.append(test_result)
        
        with open('types-all.json', 'w') as f:
            json.dump(all_types, f, indent=2)
```

### 4. Progress Reporting

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

## Success Criteria

✅ Test passes when:
- All untested types are validated
- Spawn/insert operations work for supported types
- All mutation paths work for supported types
- Progress is saved after each type

## Failure Handling

**On failure**:
1. Mark type as "Failed" in types-all.json
2. Record failure details:
   - Operation that failed (spawn/insert/mutate)
   - Error message
   - Path (for mutations)
3. **STOP TESTING** - don't continue to other types
4. Save progress so test can resume later

## Known Issues

Types that require special handling:
1. **bevy_ecs::name::Name**: Schema shows struct but BRP expects string
2. **Option fields**: Some types use "None" string vs null
3. **Handle types**: May have complex serialization

These should be marked with special handling in the test logic.

## Resume Capability

The test can be resumed at any time:
1. Previously passed types are skipped
2. Failed types can be retried 
3. Untested types are processed in order

This allows incremental testing and debugging of specific type issues.