# Migration Plan: Dict to Array for mutation_paths

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for user confirmation ("go ahead" or similar)

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & VALIDATE**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt
   cargo install --path mcp
   ```

5. **CONFIRM**: Wait for user to confirm the build succeeded

6. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

7. **PROCEED**: Move to next step only after confirmation
</Instructions>

<ExecuteImplementation>
    Find the next ⏳ PENDING step in the INTERACTIVE IMPLEMENTATION SEQUENCE below.

    For the current step:
    1. Follow the <Instructions/> above for executing the step
    2. When step is complete, use Edit tool to mark it as ✅ COMPLETED
    3. Continue to next PENDING step

    If all steps are COMPLETED:
        Display: "✅ Implementation complete! All steps have been executed."
</ExecuteImplementation>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### Step 1: Rust Migration to Array Structure ⏳ PENDING
**Objective**: Change HashMap<String, MutationPathExternal> to Vec<MutationPathExternal> in type guide system

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/guide.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

**Change Type**: ATOMIC GROUP - Both files must be changed together to avoid compilation breakage

**Build commands**:
```bash
cargo build && cargo +nightly fmt
cargo install --path mcp
```

**Notes**: This is a breaking change that converts the mutation_paths container from HashMap to Vec. Both files must be updated atomically to maintain compilation.

---

### Step 2: Create Migration Validation Script ⏳ PENDING
**Objective**: Write one-off script to validate dict→array migration preserves all data

**Files to create**:
- `.claude/scripts/validate_array_migration.py` (new file)

**Change Type**: Additive - Creates new validation script

**Build commands**: None (script file only)

**Notes**: This script compares baseline (dict format) with new file (array format) to ensure no data loss during migration.

---

### Step 3: Validate Dict→Array Migration ⏳ PENDING
**Objective**: Generate new array format, validate data integrity, promote if successful

**Prerequisites**:
- MCP server must be reconnected after Step 1
- Baseline already exists at `.claude/transient/all_types_baseline.json`

**Commands to run**:
```bash
# Generate new array format
/create_mutation_test_json

# Run validation
python3 .claude/scripts/validate_array_migration.py

# If validation passes, promote
.claude/scripts/create_mutation_test_json/promote_baseline.sh
```

**Change Type**: Validation only - No code changes

**Notes**: User must disconnect and reconnect MCP server after Step 1 for new Rust code to take effect.

---

### Step 4: Update Mutation Test Scripts ⏳ PENDING
**Objective**: Update prepare.py and initialize_test_metadata.py to work with array format

**Files to modify**:
- `.claude/scripts/mutation_test/prepare.py`
- `.claude/scripts/mutation_test/initialize_test_metadata.py`

**Change Type**: Dependent on Step 3 - Requires validated array format

**Build commands**: None (Python scripts)

**Validation**:
```bash
python3 .claude/scripts/mutation_test/prepare.py
# Should succeed with array structure
```

---

### Step 5: Update Comparison Scripts ⏳ PENDING
**Objective**: Update compare.py and read_comparison.py to work with array format

**Files to modify**:
- `.claude/scripts/create_mutation_test_json/compare.py`
- `.claude/scripts/create_mutation_test_json/read_comparison.py`

**Change Type**: Dependent on Step 3 - Requires validated array format

**Build commands**: None (Python scripts)

**Validation**:
```bash
python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural
# Should work with baseline (dict) vs current (array)
```

---

### Step 6: Update Bash Helper Scripts ⏳ PENDING
**Objective**: Update helper scripts to iterate arrays instead of dict keys

**Files to modify**:
- `.claude/scripts/get_mutation_path_list.sh`
- `.claude/scripts/get_mutation_path.sh`
- `.claude/scripts/get_type_kind.sh`

**Change Type**: Dependent on Step 3 - Requires validated array format

**Build commands**: None (bash scripts)

**Validation**:
```bash
.claude/scripts/get_mutation_path.sh extras_plugin::TestVariantChainEnum --file .claude/transient/all_types.json
.claude/scripts/get_mutation_path_list.sh extras_plugin::TestVariantChainEnum --file .claude/transient/all_types.json
.claude/scripts/get_type_kind.sh extras_plugin::TestVariantChainEnum --file .claude/transient/all_types.json
```

---

### Step 7: Update Augmentation Script ⏳ PENDING
**Objective**: Update jq expressions to count array elements instead of dict keys

**Files to modify**:
- `.claude/scripts/create_mutation_test_json/augment_response.sh`

**Change Type**: Dependent on Step 3 - Requires validated array format

**Build commands**: None (bash/jq script)

**Notes**: Changes jq expression from `mutation_paths // {} | keys` to `mutation_paths // []`

---

### Step 8: Update Documentation ⏳ PENDING
**Objective**: Update documentation to show array format examples

**Files to modify**:
- `CLAUDE.md` (workspace root)
- `.claude/plans/example.md`

**Change Type**: Documentation only

**Build commands**: None

**Notes**: Updates code examples and access patterns to use array iteration instead of dict lookups

---

### Step 9: Complete System Validation ⏳ PENDING
**Objective**: Run full test suite to verify all systems work with array format

**Test commands**:
```bash
# Test type guide generation
mcp__brp__brp_launch_bevy_example(target_name="extras_plugin", port=15702)
mcp__brp__brp_type_guide(types=["extras_plugin::TestVariantChainEnum"], port=15702)

# Test mutation test preparation
python3 .claude/scripts/mutation_test/prepare.py

# Test comparison scripts
python3 .claude/scripts/create_mutation_test_json/read_comparison.py structural

# Test helper scripts (already tested in Step 6)

# Run full mutation test
/mutation_test
```

**Change Type**: Validation only

**Success Criteria**: All tests pass, mutation test completes successfully

---

## Overview

Migrate `mutation_paths` from `HashMap<String, MutationPathExternal>` (serializes as JSON object) to `Vec<MutationPathExternal>` (serializes as JSON array). This is a container change only - all data remains identical, just repackaged.

**Current structure:**
```json
{
  "mutation_paths": {
    ".field1": { "path": ".field1", "description": "...", ... },
    ".field2": { "path": ".field2", "description": "...", ... }
  }
}
```

**Target structure:**
```json
{
  "mutation_paths": [
    { "path": ".field1", "description": "...", ... },
    { "path": ".field2", "description": "...", ... }
  ]
}
```

---

## STEP 1 DETAILS: Rust Source Changes

### File: `mcp/src/brp_tools/brp_type_guide/guide.rs`

**Line 46-47**: Change struct field type
```rust
// BEFORE
#[serde(skip_serializing_if = "HashMap::is_empty")]
pub mutation_paths: HashMap<String, MutationPathExternal>,

// AFTER
#[serde(skip_serializing_if = "Vec::is_empty")]
pub mutation_paths: Vec<MutationPathExternal>,
```

**Line 92**: Update TypeGuide::build() return type usage
```rust
// BEFORE
let mutation_paths = mutation_path_builder::build_mutation_paths(&brp_type_name, Arc::clone(&registry))?;

// AFTER (same - return type changes in api.rs)
let mutation_paths = mutation_path_builder::build_mutation_paths(&brp_type_name, Arc::clone(&registry))?;
```

**Line 105**: Initialize empty vector
```rust
// BEFORE
mutation_paths: HashMap::new(),

// AFTER
mutation_paths: Vec::new(),
```

**Line 121**: Initialize empty vector
```rust
// BEFORE
mutation_paths: HashMap::new(),

// AFTER
mutation_paths: Vec::new(),
```

**Line 134**: Update function signature
```rust
// BEFORE
fn generate_agent_guidance(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Result<String> {

// AFTER
fn generate_agent_guidance(
    mutation_paths: &[MutationPathExternal],
) -> Result<String> {
```

**Line 137-138**: Update iteration
```rust
// BEFORE
let has_entity = mutation_paths
    .values()
    .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));

// AFTER
let has_entity = mutation_paths
    .iter()
    .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));
```

**Line 158**: Update function signature
```rust
// BEFORE
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value> {

// AFTER
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &[MutationPathExternal],
) -> Option<Value> {
```

### File: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

**Line 31**: Update return type
```rust
// BEFORE
) -> Result<HashMap<String, MutationPathExternal>> {

// AFTER
) -> Result<Vec<MutationPathExternal>> {
```

**Lines 47-58**: Change from building HashMap to building Vec
```rust
// BEFORE
let external_paths = internal_paths
    .iter()
    .map(|mutation_path_internal| {
        // Keep empty path as empty for root mutations
        // BRP expects empty string for root replacements, not "."
        let key = (*mutation_path_internal.mutation_path).clone();
        let mutation_path = mutation_path_internal
            .clone()
            .into_mutation_path_external(&registry);
        (key, mutation_path)
    })
    .collect();

// AFTER
let external_paths = internal_paths
    .iter()
    .map(|mutation_path_internal| {
        mutation_path_internal
            .clone()
            .into_mutation_path_external(&registry)
    })
    .collect();
```

**Line 68**: Update function signature
```rust
// BEFORE
pub fn extract_spawn_format(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value> {

// AFTER
pub fn extract_spawn_format(
    mutation_paths: &[MutationPathExternal],
) -> Option<Value> {
```

**Lines 70-75**: Change from hash lookup to array search
```rust
// BEFORE
mutation_paths
    .get("")
    .and_then(|root_path| match &root_path.path_example {
        PathExample::Simple(val) => Some(val.clone()),
        PathExample::EnumRoot { groups, .. } => select_preferred_example(groups),
    })

// AFTER
mutation_paths
    .iter()
    .find(|p| p.path.as_ref() == "")
    .and_then(|root_path| match &root_path.path_example {
        PathExample::Simple(val) => Some(val.clone()),
        PathExample::EnumRoot { groups, .. } => select_preferred_example(groups),
    })
```

---

## STEP 2 DETAILS: Migration Validation Script

**File: `.claude/scripts/validate_array_migration.py`**

```python
#!/usr/bin/env python3
"""
One-off validation script to verify dict→array migration preserves all data.

Compares baseline (dict format) with new file (array format) to ensure:
- Same types present
- Same mutation paths per type (by path field value)
- Same data in each mutation path
- Only difference is container type (dict vs array)
"""

import json
import sys
from pathlib import Path

def load_json(filepath: Path) -> dict:
    with open(filepath) as f:
        return json.load(f)

def validate_migration(baseline_path: Path, new_path: Path) -> tuple[bool, list[str]]:
    """
    Compare baseline (dict) vs new (array) structure.

    Returns:
        (success, errors) tuple
    """
    baseline = load_json(baseline_path)
    new = load_json(new_path)

    errors = []

    baseline_types = baseline['type_guide']
    new_types = new['type_guide']

    # Check same types exist
    baseline_type_names = set(baseline_types.keys())
    new_type_names = set(new_types.keys())

    if baseline_type_names != new_type_names:
        missing = baseline_type_names - new_type_names
        added = new_type_names - baseline_type_names
        if missing:
            errors.append(f"Types missing in new: {missing}")
        if added:
            errors.append(f"Types added in new: {added}")
        return False, errors

    # Check each type
    for type_name in baseline_type_names:
        baseline_type = baseline_types[type_name]
        new_type = new_types[type_name]

        # Get mutation_paths (dict in baseline, array in new)
        baseline_paths = baseline_type.get('mutation_paths', {})
        new_paths = new_type.get('mutation_paths', [])

        # Convert both to sets of path values for comparison
        baseline_path_set = set(baseline_paths.keys())
        new_path_set = {p['path'] for p in new_paths}

        if baseline_path_set != new_path_set:
            missing = baseline_path_set - new_path_set
            added = new_path_set - baseline_path_set
            if missing:
                errors.append(f"{type_name}: paths missing in new: {missing}")
            if added:
                errors.append(f"{type_name}: paths added in new: {added}")
            continue

        # Check each path has same data (minus container structure)
        for path_key in baseline_path_set:
            baseline_path_data = baseline_paths[path_key]
            # Find matching path in new array
            new_path_data = next((p for p in new_paths if p['path'] == path_key), None)

            if not new_path_data:
                errors.append(f"{type_name}.{path_key}: not found in new array")
                continue

            # Compare all fields (both should have same keys now)
            baseline_keys = set(baseline_path_data.keys())
            new_keys = set(new_path_data.keys())

            if baseline_keys != new_keys:
                errors.append(f"{type_name}.{path_key}: field mismatch - baseline: {baseline_keys}, new: {new_keys}")
                continue

            # Deep compare each field value
            for field in baseline_keys:
                if baseline_path_data[field] != new_path_data[field]:
                    errors.append(f"{type_name}.{path_key}.{field}: value mismatch")

    success = len(errors) == 0
    return success, errors

def main():
    baseline_path = Path('.claude/transient/all_types_baseline.json')
    new_path = Path('.claude/transient/all_types.json')

    if not baseline_path.exists():
        print(f"❌ Baseline not found: {baseline_path}")
        sys.exit(1)

    if not new_path.exists():
        print(f"❌ New file not found: {new_path}")
        sys.exit(1)

    print("Validating dict→array migration...")
    print(f"  Baseline: {baseline_path}")
    print(f"  New:      {new_path}")
    print()

    success, errors = validate_migration(baseline_path, new_path)

    if success:
        print("✅ VALIDATION PASSED")
        print("   All types present")
        print("   All mutation paths present")
        print("   All data identical")
        print("   Migration successful!")
        sys.exit(0)
    else:
        print("❌ VALIDATION FAILED")
        print(f"   Found {len(errors)} error(s):")
        for error in errors:
            print(f"   - {error}")
        sys.exit(1)

if __name__ == '__main__':
    main()
```

---

## STEP 4 DETAILS: Mutation Test Script Updates

### File: `.claude/scripts/mutation_test/prepare.py`

**Line 61**: Update TypedDict (already done - has `path` field)
```python
class MutationPathData(TypedDict, total=False):
    path: str  # Already added
    description: str
    example: object
    examples: list[object]
    path_info: PathInfo
```

**Line 535**: Change iteration from dict to array
```python
# BEFORE
for _key, path_info in mutation_paths.items():
    path = cast(str, cast(dict[str, object], path_info)["path"])

# AFTER
for path_info in mutation_paths:
    path = cast(str, cast(dict[str, object], path_info)["path"])
```

**Line 1054**: Change iteration from dict to array
```python
# BEFORE
for _key, path_data in mutation_paths.items():
    path = cast(str, cast(dict[str, object], cast(object, path_data))["path"])

# AFTER
for path_data in mutation_paths:
    path = cast(str, cast(dict[str, object], cast(object, path_data))["path"])
```

**Lines 1082-1086**: Change from dict reconstruction to list filtering
```python
# BEFORE
if paths_to_keep:
    # Reconstruct dict with only kept paths
    available_dict: dict[str, object] = {k: cast(object, mutation_paths[k]) for k in paths_to_keep}
    type_data["mutation_paths"] = available_dict

# AFTER
if paths_to_keep:
    # Filter to only kept paths
    available_list = [
        p for p in mutation_paths
        if cast(str, cast(dict[str, object], p)["path"]) in paths_to_keep
    ]
    type_data["mutation_paths"] = available_list
```

### File: `.claude/scripts/mutation_test/initialize_test_metadata.py`

**Line 39**: Update TypedDict
```python
# BEFORE
class TypeGuide(TypedDict, total=False):
    type: str
    mutation_paths: dict[str, Any]  # pyright: ignore[reportExplicitAny]
    batch_number: int | None
    test_status: str
    fail_reason: str

# AFTER
class TypeGuide(TypedDict, total=False):
    type: str
    mutation_paths: list[Any]  # pyright: ignore[reportExplicitAny]
    batch_number: int | None
    test_status: str
    fail_reason: str
```

**Line 69**: Change from dict value access to array index
```python
# BEFORE
if len(mutation_paths) == 1:
    root_path: dict[str, Any] = next(iter(mutation_paths.values()))
    if root_path.get("path") != "":
        return False

# AFTER
if len(mutation_paths) == 1:
    root_path: dict[str, Any] = mutation_paths[0]
    if root_path.get("path") != "":
        return False
```

---

## STEP 5 DETAILS: Comparison Script Updates

### File: `.claude/scripts/create_mutation_test_json/compare.py`

**Line 103-115**: Update `extract_mutation_path()` to handle array indices
```python
# BEFORE
def extract_mutation_path(path: str) -> str | None:
    """Extract mutation path from JSON path like 'mutation_paths.PATH.field'"""
    if not path.startswith("mutation_paths."):
        return None

    parts = path.split(".", 2)
    if len(parts) < 2:
        return None

    # mutation_paths.PATH or mutation_paths.PATH.field
    return parts[1]

# AFTER
def extract_mutation_path(path: str) -> str | None:
    """Extract mutation path from JSON path like 'mutation_paths[INDEX].field'"""
    if not path.startswith("mutation_paths["):
        return None

    # Path format: mutation_paths[INDEX] or mutation_paths[INDEX].field
    # We need to look up the actual path value from the data
    # This function now needs access to the data structure to resolve
    # For now, return None and handle in caller
    return None  # Will be handled by correlation logic
```

**Note**: The comparison system will need to correlate array elements by `path` field value. Since arrays don't have inherent keys, we match elements by their `path` field. This is straightforward:

```python
def correlate_mutation_paths(baseline_array: list, current_array: list) -> dict:
    """
    Correlate baseline and current mutation paths by their 'path' field.

    Returns dict with:
        'matched': [(baseline_item, current_item), ...],
        'removed': [baseline_item, ...],
        'added': [current_item, ...]
    """
    baseline_by_path = {item['path']: item for item in baseline_array}
    current_by_path = {item['path']: item for item in current_array}

    baseline_paths = set(baseline_by_path.keys())
    current_paths = set(current_by_path.keys())

    matched = [
        (baseline_by_path[p], current_by_path[p])
        for p in baseline_paths & current_paths
    ]
    removed = [baseline_by_path[p] for p in baseline_paths - current_paths]
    added = [current_by_path[p] for p in current_paths - baseline_paths]

    return {'matched': matched, 'removed': removed, 'added': added}
```

### File: `.claude/scripts/create_mutation_test_json/read_comparison.py`

**Line 359-366**: Update path lookup to search array
```python
# BEFORE
def get_mutation_path_data(type_data: dict, mutation_path: str) -> dict | None:
    mutation_paths = type_data["mutation_paths"]
    if mutation_path not in mutation_paths:
        return None
    return mutation_paths[mutation_path]

# AFTER
def get_mutation_path_data(type_data: dict, mutation_path: str) -> dict | None:
    mutation_paths = type_data["mutation_paths"]
    # Search array for matching path field
    for path_obj in mutation_paths:
        if path_obj.get("path") == mutation_path:
            return path_obj
    return None
```

---

## STEP 6 DETAILS: Bash Helper Script Updates

### File: `.claude/scripts/get_mutation_path_list.sh`

**Line 84**: Change from dict keys to array iteration
```python
# BEFORE
for path in mutation_paths.keys():
    print(path)

# AFTER
for path_obj in mutation_paths:
    path = path_obj.get("path", "")
    print(path)
```

### File: `.claude/scripts/get_mutation_path.sh`

**Lines 112-145**: Update path listing and lookup
```python
# BEFORE - List all paths
for i, path in enumerate(list(mutation_paths.keys())[:20]):
    print(f"{i+1}. {path}")

# AFTER - List all paths
for i, path_obj in enumerate(mutation_paths[:20]):
    path = path_obj.get("path", "")
    print(f"{i+1}. {path}")

# BEFORE - Get specific path
if mutation_path not in mutation_paths:
    print(f"❌ Mutation path not found: {mutation_path}")
    sys.exit(1)
path_data = mutation_paths[mutation_path]

# AFTER - Get specific path
path_data = None
for path_obj in mutation_paths:
    if path_obj.get("path") == mutation_path:
        path_data = path_obj
        break

if not path_data:
    print(f"❌ Mutation path not found: {mutation_path}")
    sys.exit(1)
```

### File: `.claude/scripts/get_type_kind.sh`

**Lines 36-39**: Update iteration
```python
# BEFORE
for path, path_data in guide['mutation_paths'].items():
    # ... process

# AFTER
for path_data in guide['mutation_paths']:
    path = path_data.get('path', '')
    # ... process
```

---

## STEP 7 DETAILS: Augmentation Script Update

### File: `.claude/scripts/create_mutation_test_json/augment_response.sh`

**Lines 93-108**: Update jq path counting
```bash
# BEFORE
"total_mutation_paths": [
    $types | to_entries | .[] |
    .value.mutation_paths // {} | keys | .[]
] | length,

# AFTER
"total_mutation_paths": [
    $types | to_entries | .[] |
    .value.mutation_paths // [] | .[]
] | length,
```

---

## STEP 8 DETAILS: Documentation Updates

### File: `CLAUDE.md` (workspace root)

Update the section describing all_types.json structure:

```markdown
## all_types.json structure
The `.claude/transient/all_types.json` file stores complete BRP type guides with test metadata.

**Top-level structure**:
```json
{
  "discovered_count": 252,
  "requested_types": [...],
  "summary": {...},
  "type_guide": {
    "bevy_camera::camera::Camera": {
      "type_name": "bevy_camera::camera::Camera",
      "mutation_paths": [
        {
          "path": ".is_active",
          "description": "Mutate the is_active field of Camera",
          "path_info": {...},
          "example": true
        }
      ],
      "spawn_format": {...},
      "schema_info": {...},
      "batch_number": 1,
      "test_status": "passed",
      "fail_reason": ""
    }
  }
}
```

**Accessing types**: `all_types['type_guide'][type_name]`
**Accessing mutation paths**: Iterate the array: `for path_obj in all_types['type_guide'][type_name]['mutation_paths']`
**Finding specific path**: Search by path field: `next((p for p in paths if p['path'] == '.is_active'), None)`
**Test metadata fields**: `batch_number`, `test_status`, `fail_reason` (added by augmentation script)
```

### File: `.claude/plans/example.md`

Update any examples showing mutation_paths structure to use array format.

---

## Summary of Changed Files

### Rust (2 files)
- ✅ `mcp/src/brp_tools/brp_type_guide/guide.rs`
- ✅ `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

### Python Scripts (5 files)
- ✅ `.claude/scripts/mutation_test/prepare.py`
- ✅ `.claude/scripts/mutation_test/initialize_test_metadata.py`
- ✅ `.claude/scripts/create_mutation_test_json/compare.py`
- ✅ `.claude/scripts/create_mutation_test_json/read_comparison.py`
- ✅ `.claude/scripts/validate_array_migration.py` (new file)

### Bash Scripts (4 files)
- ✅ `.claude/scripts/get_mutation_path_list.sh`
- ✅ `.claude/scripts/get_mutation_path.sh`
- ✅ `.claude/scripts/get_type_kind.sh`
- ✅ `.claude/scripts/create_mutation_test_json/augment_response.sh`

### Documentation (2 files)
- ✅ `CLAUDE.md` (workspace root)
- ✅ `.claude/plans/example.md`

**Total**: 15 files to modify, 1 file to create

---

## Key Benefits of Array Structure

1. **Simpler conceptually** - mutation paths are just a list, not a lookup table
2. **Natural iteration** - `for path in mutation_paths` instead of `for _, path in mutation_paths.items()`
3. **Order preservation** - arrays maintain insertion order naturally
4. **Type safety** - no confusion about whether key and path field should match
5. **Future proof** - easier to extend with additional metadata without worrying about key conflicts

---
