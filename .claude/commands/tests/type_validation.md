# Type Schema Comprehensive Validation Test

## Objective
Systematically validate all BRP component types by testing spawn/insert and mutation operations using individual type schema files. This test tracks progress in `all_types.json` to avoid retesting passed types.

**NOTE**: The extras_plugin app is already running on the specified port - focus on comprehensive type validation.

## Schema Files Location
- **Type schemas**: `.claude/commands/tests/type_schemas/`
- **Progress tracking**: `.claude/commands/tests/type_schemas/all_types.json`
- **Individual schemas**: `.claude/commands/tests/type_schemas/{type_name}.json`

## Progress Tracking

The test uses `all_types.json` to track testing progress. The file structure is updated from a simple array to include status:

```json
[
  {
    "type": "bevy_ecs::name::Name",
    "status": "Passed"  // or "Failed"
  },
  {
    "type": "bevy_transform::components::transform::Transform"
    // No status field means not tested yet
  }
]
```

## Test Strategy

1. **Load progress**: Read `all_types.json` to see which types have been tested
2. **Skip passed types**: Don't retest types marked as "Passed"
3. **Build todo list**: Create tasks only for untested or failed types
4. **Test each type**: Load individual schema file and test operations
5. **Update progress**: Mark types as "Passed" or "Failed" in `all_types.json`
6. **STOP ON FIRST FAILURE** for immediate issue identification

## Test Steps

### 1. Load Progress and Build Todo List

```python
import json
import os

# Load current progress
with open('.claude/commands/tests/type_schemas/all_types.json', 'r') as f:
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

#### 2b. Test Spawn/Insert Operations
If "spawn" or "insert" in supported operations:
```python
if 'spawn' in supported_ops:
    spawn_format = type_schema.get('spawn_format')
    # Execute mcp__brp__bevy_spawn with spawn_format
    # Record entity ID if successful
```

**KNOWN ISSUES to handle**:
- `bevy_ecs::name::Name`: Use plain string instead of struct format
- Option fields: Use "None" string instead of null

#### 2c. Test All Mutation Paths
If "mutate" in supported operations:
```python
if 'mutate' in supported_ops:
    mutation_paths = type_schema.get('mutation_paths', {})
    
    for path, path_info in mutation_paths.items():
        # Determine value to use
        if 'example' in path_info:
            value = path_info['example']
        elif 'enum_variants' in path_info:
            value = path_info['enum_variants'][0]
        elif 'example_some' in path_info:
            # Test both Some and None
            test_values = [path_info['example_some'], path_info['example_none']]
        
        # Execute mcp__brp__bevy_mutate_component
        # Stop on first failure
```

### 3. Update Progress

After testing each type:

```python
def update_progress(type_name, status):
    with open('.claude/commands/tests/type_schemas/all_types.json', 'r') as f:
        all_types = json.load(f)
    
    # Convert to dict format if needed
    if isinstance(all_types, list) and all(isinstance(t, str) for t in all_types):
        all_types = [{"type": t} for t in all_types]
    
    # Update status
    for entry in all_types:
        if entry.get("type") == type_name:
            entry["status"] = status
            break
    
    # Save updated progress
    with open('.claude/commands/tests/type_schemas/all_types.json', 'w') as f:
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
1. Mark type as "Failed" in all_types.json
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