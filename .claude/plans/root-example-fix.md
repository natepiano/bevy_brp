# Root Example Collision Fix

## Problem

When multiple enum variants share the same field names, they create mutation paths with identical keys that collide in the HashMap, silently overwriting all but the last variant.

### Example with `Color` enum

All 10 color variants have an `alpha` field:
- `Xyza` → creates `.0.alpha` path
- `Hsla` → creates `.0.alpha` path
- `Srgba` → creates `.0.alpha` path
- etc.

Currently in `api.rs:47-58`, these paths are collected into a `HashMap<String, MutationPathExternal>` where the key is the mutation_path string. **Only the last variant wins** - all others are silently overwritten.

### Current Behavior
```json
{
  "mutation_paths": {
    ".0.alpha": {
      "applicable_variants": ["Color::Xyza"],  // Only shows last variant!
      "root_example": {"Xyza": {"alpha": 1.0, "x": 1.0, "y": 1.0, "z": 1.0}}
    }
  }
}
```

### Why This Happens

In `api.rs:47-58`:
```rust
let external_paths = internal_paths
    .iter()
    .map(|mutation_path_internal| {
        let key = (*mutation_path_internal.mutation_path).clone();  // ".0.alpha"
        let mutation_path = mutation_path_internal
            .clone()
            .into_mutation_path_external(&registry);
        (key, mutation_path)  // All variants create same key!
    })
    .collect();  // Last entry wins, others silently discarded
```

## Solution: Change HashMap to Array

Instead of using a HashMap with paths as keys, output an array where `path` is a field within each `MutationPathExternal`. This naturally allows duplicate paths - each variant gets its own array entry.

### Current Structure (HashMap)
```rust
HashMap<String, MutationPathExternal>

// JSON output:
{
  "mutation_paths": {
    ".0.alpha": { description: "...", path_info: {...} },
    ".0.hue": { description: "...", path_info: {...} }
  }
}
```

### New Structure (Array)
```rust
Vec<MutationPathExternal>

// JSON output:
{
  "mutation_paths": [
    {
      "path": ".0.alpha",
      "description": "Mutate the 'alpha' field...",
      "applicable_variants": ["Color::Xyza"],
      "root_example": {"Xyza": {"alpha": 1.0, "x": 1.0, "y": 1.0, "z": 1.0}}
    },
    {
      "path": ".0.alpha",  // Same path, different variant!
      "description": "Mutate the 'alpha' field...",
      "applicable_variants": ["Color::Hsla"],
      "root_example": {"Hsla": {"alpha": 1.0, "hue": 1.0, "saturation": 1.0, "lightness": 1.0}}
    },
    {
      "path": ".0.hue",
      "description": "Mutate the 'hue' field...",
      "applicable_variants": ["Color::Hsla"],
      "root_example": {"Hsla": {"alpha": 1.0, "hue": 1.0, "saturation": 1.0, "lightness": 1.0}}
    }
  ]
}
```

## Implementation Steps

### 1. Add `path` field to `MutationPathExternal`

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MutationPathExternal {
    /// The mutation path string (e.g., ".0.alpha", ".translation.x")
    ///
    /// Previously this was the HashMap key, now it's a field within the struct.
    /// Multiple entries can have the same path if they apply to different enum variants.
    pub path: String,

    pub description: String,

    #[serde(flatten)]
    pub path_info: PathInfo,
}
```

### 2. Update `api.rs` to return Vec instead of HashMap

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs:47-58`

```rust
pub fn build_mutation_paths(
    type_path: &str,
    registry: &TypeRegistry,
) -> Result<Vec<MutationPathExternal>, BuilderError> {
    let internal_paths = PathBuilder::build_paths_from_type_path(type_path, registry)?;

    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            mutation_path_internal
                .clone()
                .into_mutation_path_external(registry)
        })
        .collect();

    Ok(external_paths)
}
```

**Change summary**:
- Return type: `HashMap<String, MutationPathExternal>` → `Vec<MutationPathExternal>`
- Remove tuple mapping with key
- Simple map and collect

### 3. Update `into_mutation_path_external` to include path

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

Find the `into_mutation_path_external` method and add the `path` field:

```rust
impl MutationPathInternal {
    pub fn into_mutation_path_external(self, registry: &TypeRegistry) -> MutationPathExternal {
        MutationPathExternal {
            path: (*self.mutation_path).clone(),  // NEW: add path field
            description: self.description,
            path_info: PathInfo {
                // ... existing fields unchanged ...
            }
        }
    }
}
```

### 4. Update consumers to work with array structure

**Scripts to update**:

1. **`.claude/scripts/mutation_test_process_results.py`**
   - Change: `for path, data in type_guide['mutation_paths'].items()`
   - To: `for path_entry in type_guide['mutation_paths']:`
   - Access path via: `path_entry['path']`

2. **`.claude/scripts/create_mutation_test_json_deep_comparison.py`**
   - Change: Dictionary access `mutation_paths[path_name]`
   - To: Array iteration with filter `[p for p in mutation_paths if p['path'] == path_name]`

3. **Integration tests**
   - Update any tests that expect `mutation_paths` to be a dict
   - Update to iterate array or filter by path field

4. **Tool response handlers**
   - `mcp/src/brp_tools/brp_type_guide/mod.rs` - update response serialization
   - Any code that builds the JSON response structure

## Files to Modify

### Core Implementation (Rust)

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`**
   - Add `path: String` field to `MutationPathExternal` struct
   - Update serialization to include path field

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`** (lines 47-58)
   - Change return type: `HashMap<String, MutationPathExternal>` → `Vec<MutationPathExternal>`
   - Remove tuple mapping with key
   - Simplify to `.map()` and `.collect()`

3. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`**
   - Add `path: (*self.mutation_path).clone()` to `MutationPathExternal` construction in `into_mutation_path_external()`

4. **`mcp/src/brp_tools/brp_type_guide/mod.rs`**
   - Update response serialization to output array instead of HashMap

### Python Scripts (Dict → Array Access)

5. **`.claude/scripts/mutation_test_prepare.py`**
   - Line 37: Change type definition from `dict[str, MutationPathData]` to `list[MutationPathData]`
   - Line 27-30: Update `MutationPathData` TypedDict to include `path: str` field
   - Line 257: Change `mutation_paths = type_data.get("mutation_paths") or {}` to `or []`
   - Line 337: Change `for path, path_info in mutation_paths.items():` to iterate array
   - Access pattern: Change from `path` (key) + `path_info` (value) to `path_entry['path']` + `path_entry`

6. **`.claude/scripts/create_mutation_test_json/read_comparison.py`**
   - Line 358: Keep `mutation_paths = type_data["mutation_paths"]` (now returns array)
   - Line 359-362: Replace dict membership check with array filter:
     ```python
     # OLD: if mutation_path not in mutation_paths: return None
     # OLD: return mutation_paths[mutation_path]
     # NEW:
     matching = [p for p in mutation_paths if p.get('path') == mutation_path]
     return matching[0] if matching else None
     ```

7. **`.claude/scripts/create_mutation_test_json/compare.py`**
   - Line 419: Update truthiness check (array is truthy if non-empty)
   - Line 422: Change `.get("mutation_paths", {})` to `.get("mutation_paths", [])`
   - Line 424: Change `isinstance(t.get("mutation_paths"), dict)` to `isinstance(t.get("mutation_paths"), list)`
   - Update `len()` call - works for both dict and list, but validate it's counting correctly

### Shell Scripts (Bash + Python/jq)

8. **`.claude/scripts/create_mutation_test_json/augment_response.sh`**
   - Lines 85-92, 122, 126: Update jq filters:
     ```bash
     # OLD: if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == {})
     # NEW: if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == [])

     # OLD: select(.value.mutation_paths != null and .value.mutation_paths != {})
     # NEW: select(.value.mutation_paths != null and .value.mutation_paths != [])

     # OLD: .value.mutation_paths // {} | keys | .[]
     # NEW: .value.mutation_paths // [] | .[] | .path
     ```

9. **`.claude/scripts/get_type_kind.sh`**
   - Lines 36-38, 73-74: Update Python iteration:
     ```python
     # OLD: for path, path_data in guide['mutation_paths'].items():
     # NEW: for path_entry in guide['mutation_paths']:
     #      path = path_entry['path']
     #      path_data = path_entry
     ```

10. **`.claude/scripts/get_mutation_path.sh`**
    - Lines 108-150: Update Python dict access patterns:
      ```python
      # Line 112: mutation_paths = type_data['mutation_paths']  # Now returns array
      # Line 121: for i, path in enumerate(list(mutation_paths.keys())[:20]):
      #   NEW: for i, entry in enumerate(mutation_paths[:20]):
      #        path = entry['path']

      # Line 134: if mutation_path not in mutation_paths:
      #   NEW: if not any(p['path'] == mutation_path for p in mutation_paths):

      # Line 138: matching = [p for p in mutation_paths.keys() if mutation_path in p]
      #   NEW: matching = [p['path'] for p in mutation_paths if mutation_path in p['path']]

      # Line 145: path_data = mutation_paths[mutation_path]
      #   NEW: path_data = next((p for p in mutation_paths if p['path'] == mutation_path), None)
      ```

11. **`.claude/scripts/get_mutation_path_list.sh`**
    - Lines 77-88: Update Python iteration:
      ```python
      # Line 81: mutation_paths = type_data['mutation_paths']  # Now returns array
      # Line 84: for path in mutation_paths.keys():
      #   NEW: for entry in mutation_paths:
      #        path = entry['path']
      ```

12. **`.claude/scripts/type_guide_test_extract.sh`**
    - Lines 45-51: Update jq extraction (works as-is, extracts whole array)
    - Lines 66-73: Update path validation from dict key check to array search:
      ```bash
      # OLD: jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | has($path)'
      # NEW: jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | any(.[]; .path == $path)'
      ```

### Documentation Files

13. **`.claude/commands/create_mutation_test_json.md`**
    - Line 50: Remove or fix misleading "array" reference
    - Lines 304-313: **CRITICAL** - Update `<MutationPathsExplanation/>` section:
      - Change dict access examples to array iteration examples
      - Update: `type_guide['TypeName']['mutation_paths']['.path']`
      - To: `next(p for p in type_guide['TypeName']['mutation_paths'] if p['path'] == '.path')`
    - Lines 159-161: Change "objects with path keys" to "array of objects with path field"
    - Lines 268-277: Update comparison format examples

14. **`.claude/commands/get_guide_current.md`**
    - Line 36: Update "Available paths" to reference array structure
    - Lines 34-37: Update filtering examples to use array search

15. **`.claude/commands/get_guide_baseline.md`**
    - Lines 51-54, 69: Update examples to show array structure
    - Line 102: Update JSON example from object to array

16. **`.claude/commands/get_kind_baseline.md`**
    - Line 50: Fix "array" reference to be accurate

17. **`.claude/commands/get_path_baseline.md`**
    - Lines 65-68: Update access examples

18. **`.claude/commands/compare_mutation_path.md`**
    - Lines 137-149: Update output section examples

### Integration Tests

19. **`.claude/integration_tests/type_guide.md`**
    - Line 66: Update extraction script call (script will handle array internally)
    - Line 67: Update comment about structure
    - Lines 81, 134, 197, 247: Update references to dict structure
    - Update all test assertions to expect array structure

20. **`.claude/integration_tests/data_operations.md`**
    - Lines 36-55: Update references to mutation paths discovery to reflect array structure

### Data Files (Auto-regenerated - No Manual Changes)

The following files will be automatically regenerated with the new structure:
- `.claude/transient/all_types.json`
- `.claude/transient/all_types_baseline.json`
- `.claude/transient/all_types_stats.json`
- `.claude/transient/all_types_good_*.json`
- `.claude/transient/all_types_review_failures_*.json`

## Detailed Changes by File

### Rust Implementation (4 files)

#### 1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` (Lines 228-238)

**Current:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    pub description:  String,
    pub path_info:    PathInfo,
    #[serde(flatten)]
    pub path_example: PathExample,
}
```

**New:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    pub path:         String,
    pub description:  String,
    pub path_info:    PathInfo,
    #[serde(flatten)]
    pub path_example: PathExample,
}
```

#### 2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs` (Lines 28-61)

**Current:**
```rust
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<HashMap<String, MutationPathExternal>> {
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            let key = (*mutation_path_internal.mutation_path).clone();
            let mutation_path = mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry);
            (key, mutation_path)
        })
        .collect();
    Ok(external_paths)
}
```

**New:**
```rust
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<Vec<MutationPathExternal>> {
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            let path = (*mutation_path_internal.mutation_path).clone();
            mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry, path)
        })
        .collect();
    Ok(external_paths)
}
```

Also update `extract_spawn_format` (Lines 63-76):

**Current:**
```rust
pub fn extract_spawn_format(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value> {
    mutation_paths.get("")
}
```

**New:**
```rust
pub fn extract_spawn_format(
    mutation_paths: &[MutationPathExternal],
) -> Option<Value> {
    mutation_paths.iter().find(|path| path.path.is_empty())
}
```

#### 3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` (Lines 76-110)

**Current:**
```rust
pub fn into_mutation_path_external(
    mut self,
    registry: &HashMap<BrpTypeName, Value>,
) -> MutationPathExternal {
    MutationPathExternal {
        description,
        path_info: PathInfo { ... },
        path_example,
    }
}
```

**New:**
```rust
pub fn into_mutation_path_external(
    mut self,
    registry: &HashMap<BrpTypeName, Value>,
    path: String,
) -> MutationPathExternal {
    MutationPathExternal {
        path,
        description,
        path_info: PathInfo { ... },
        path_example,
    }
}
```

#### 4. `mcp/src/brp_tools/brp_type_guide/guide.rs` (Multiple locations)

**Lines 38-58:**
```rust
// OLD: pub mutation_paths: HashMap<String, MutationPathExternal>,
pub mutation_paths: Vec<MutationPathExternal>,
```

**Lines 100-111, 113-127:**
```rust
// OLD: mutation_paths: HashMap::new(),
mutation_paths: Vec::new(),
```

**Lines 129-150:**
```rust
// OLD: mutation_paths: &HashMap<String, MutationPathExternal>
// NEW: mutation_paths: &[MutationPathExternal]
// OLD: mutation_paths.values().any(...)
// NEW: mutation_paths.iter().any(...)
```

### Python Scripts (3 files)

#### 5. `.claude/scripts/mutation_test_prepare.py`

**Lines 27-30 (Add path field):**
```python
class MutationPathData(TypedDict, total=False):
    path: str  # NEW
    description: str
    example: Any
    path_info: dict[str, str]
```

**Line 37:**
```python
# OLD: mutation_paths: dict[str, MutationPathData] | None
mutation_paths: list[MutationPathData] | None
```

**Line 257:**
```python
# OLD: mutation_paths = type_data.get("mutation_paths") or {}
mutation_paths = type_data.get("mutation_paths") or []
```

**Line 337:**
```python
# OLD: for path, path_info in mutation_paths.items():
for path_info in mutation_paths:
    path = path_info.get("path", "")
```

#### 6. `.claude/scripts/create_mutation_test_json/read_comparison.py` (Lines 355-362)

**Current:**
```python
mutation_paths = type_data["mutation_paths"]
if mutation_path not in mutation_paths:
    return None
return mutation_paths[mutation_path]
```

**New:**
```python
mutation_paths = type_data["mutation_paths"]
if not isinstance(mutation_paths, list):
    return None
for path_data in mutation_paths:
    if isinstance(path_data, dict) and path_data.get("path") == mutation_path:
        return path_data
return None
```

#### 7. `.claude/scripts/create_mutation_test_json/compare.py` (Lines 419-427)

**Current:**
```python
total_paths = sum(
    len(cast(dict[str, JsonValue], t.get("mutation_paths", {})))
    if isinstance(t.get("mutation_paths"), dict)
    else 0
    for t in data.values()
    if isinstance(t, dict)
)
```

**New:**
```python
total_paths = sum(
    len(cast(list[JsonValue], t.get("mutation_paths", [])))
    if isinstance(t.get("mutation_paths"), list)
    else 0
    for t in data.values()
    if isinstance(t, dict)
)
```

### Shell Scripts (5 files)

#### 8. `.claude/scripts/create_mutation_test_json/augment_response.sh`

**Lines 85-92:**
```bash
# OLD: if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == {})
if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == [])

# OLD: elif (($entry.value.mutation_paths | type == "object") and ($entry.value.mutation_paths | length == 1) and ($entry.value.mutation_paths | has("")))
elif (($entry.value.mutation_paths | type == "array") and ($entry.value.mutation_paths | length == 1) and ($entry.value.mutation_paths[0].path == ""))

# OLD: if (($entry.value.mutation_paths[""].path_info.mutability // "") == "not_mutable")
if (($entry.value.mutation_paths[0].path_info.mutability // "") == "not_mutable")
```

**Line 122:**
```bash
# OLD: select(.value.mutation_paths != null and .value.mutation_paths != {})
select(.value.mutation_paths != null and .value.mutation_paths != [])
```

**Line 126:**
```bash
# OLD: .value.mutation_paths // {} | keys | .[]
.value.mutation_paths // [] | .[]
```

#### 9. `.claude/scripts/get_type_kind.sh` (Lines 36-38, 73-74)

**Current:**
```python
for path, path_data in guide['mutation_paths'].items():
```

**New:**
```python
for path_data in guide['mutation_paths']:
```

#### 10. `.claude/scripts/get_mutation_path.sh` (Lines 108-150)

**Line 121:**
```python
# OLD: for i, path in enumerate(list(mutation_paths.keys())[:20]):
for i, path_obj in enumerate(mutation_paths[:20]):
    path = path_obj['path']
```

**Lines 134-145:**
```python
# OLD: if mutation_path not in mutation_paths:
path_data = None
for path_obj in mutation_paths:
    if path_obj['path'] == mutation_path:
        path_data = path_obj
        break

if path_data is None:
    # ... error handling ...
    matching = [path_obj['path'] for path_obj in mutation_paths if mutation_path in path_obj['path']]
```

#### 11. `.claude/scripts/get_mutation_path_list.sh` (Lines 77-88)

**Current:**
```python
for path in mutation_paths.keys():
```

**New:**
```python
for path_obj in mutation_paths:
    path = path_obj['path']
```

#### 12. `.claude/scripts/type_guide_test_extract.sh` (Lines 66-73)

**Current:**
```bash
jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | has($path)'
```

**New:**
```bash
jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | any(.path == $path)'
```

### Documentation Files (6 files)

#### 13. `.claude/commands/create_mutation_test_json.md`

**Lines 159-161:**
```markdown
# OLD: Complete mutation_paths as objects with path keys and example values
Complete mutation_paths as arrays of objects containing path and example data
```

**Lines 304-313 (CRITICAL section):**
```markdown
<MutationPathsExplanation>
**Understanding Mutation Paths Structure**

Mutation paths are stored as an array of objects, NOT a dictionary:
- **Structure**: `mutation_paths` is an array where each element has a `path` field
- **Example path**: `.image_mode.0.center_scale_mode`
- **Access**: `[obj for obj in type_guide['TypeName']['mutation_paths'] if obj['path'] == '.image_mode.0.center_scale_mode'][0]`
- **Alternative**: Use helper functions to find paths by string key

Path notation patterns: `.field.0` (variant), `.field[0]` (array), `.field.0.nested` (nested in variant)
</MutationPathsExplanation>
```

#### 14. `.claude/commands/get_guide_current.md` (Lines 102-105)

**Current:**
```json
"mutation_paths": {
  "": { /* root mutation */ },
  ".field": { /* field mutations */ }
}
```

**New:**
```json
"mutation_paths": [
  { "path": "", /* root mutation */ },
  { "path": ".field", /* field mutations */ }
]
```

#### 15. `.claude/commands/get_guide_baseline.md` (Lines 84-87)

Same change as above - show array structure with path fields.

#### 16. `.claude/commands/get_kind_baseline.md` (Line 50)

**Current:**
```markdown
The baseline file must have the expected structure with `type_guide` array containing types with `mutation_paths`
```

**New:**
```markdown
The baseline file must have the expected structure with `type_guide` containing types with `mutation_paths` arrays
```

#### 17. `.claude/commands/get_path_baseline.md` (Lines 94-104)

Update output format to show flattened structure with path as a field.

#### 18. `.claude/commands/compare_mutation_path.md` (Lines 141-148)

Update terminology from "dict values" to "array elements" and "path objects".

### Integration Tests (2 files)

#### 19. `.claude/integration_tests/type_guide.md`

**Lines 66-67:** Add note about array format
**Lines 81-82:** Explicitly state Vec3/Quat use array format
**Lines 186-187:** Add note that object format intentionally fails
**Lines 196-197:** Emphasize array format in error validation
**Lines 238-251:** Add explicit wrong format notes

#### 20. `.claude/integration_tests/data_operations.md`

**Lines 13-14:** Add critical note about array format requirement
**Lines 37, 40, 45:** Update to reflect type_guide returns array
**Line 57:** Add success criterion for array format usage

## Testing

Test with `Color` enum after removing all `enum_variant_signature` knowledge entries:

1. **Verify duplicate paths preserved**
   - Array should contain 10 separate `.0.alpha` entries (one per variant)
   - Array should contain separate `.0.hue`, `.0.saturation`, `.0.lightness` entries for applicable variants

2. **Verify each entry is independent**
   - Each entry has its own `applicable_variants` (single variant)
   - Each entry has its own `root_example` specific to that variant
   - No `root_examples` (plural) field needed

3. **Verify scripts work with new structure**
   - Update and run `mutation_test_process_results.py`
   - Update and run `create_mutation_test_json_deep_comparison.py`
   - Run any integration tests that parse mutation paths

## Benefits

- **Simple implementation**: ~10 lines of changes to core code
- **No merge logic**: Each path is independent, no complex state transitions
- **Natural semantics**: Duplicate paths are naturally represented as separate array entries
- **Clearer structure**: Each entry is self-contained with all context needed
- **Easy to filter**: Agents can filter array by path, variant, or any field
- **Maintainable**: No complex RootExample enum or merge methods to maintain

## Migration Impact

**Breaking change**: External consumers expecting HashMap structure will need updates.

**Internal impact**:
- Python scripts need updates to iterate arrays instead of dicts
- Integration tests need updates for new structure
- All changes are straightforward dict→array conversions

**Timeline**: Estimate ~2-4 hours to update all consumers after core implementation.
