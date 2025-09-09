# Plan: Fix Mutation Test System to Use Full Type Schemas

## Problem Statement

The current mutation test system only stores mutation path names (strings) without the actual examples and type information. This critical flaw means:
- Changes in mutation formats (like Vec3 object → array) go undetected
- Mutation tests may use incorrect examples
- Comparisons can't detect behavioral changes, only structural changes
- We lose the source of truth from BRP

## Core Design Change

**From**: Storing only path names and metadata
**To**: Storing complete BRP type schemas with examples PLUS test metadata

## File Structure Changes

### Primary Files
- `all_types.json` - Full BRP schema response + test metadata (batch_number, test_status, fail_reason)
- `all_types_baseline.json` - Full BRP schema response (for comparison)
- `all_types_good_TIMESTAMP.json` - Full BRP schema snapshots (historical records)

### Key Principle
The full BRP response from `brp_all_type_schemas` becomes the source of truth. We augment it with test metadata but NEVER lose the original schema information.

## Detailed Changes Required

### 1. Update `create_mutation_test_json.md`

#### Current Behavior
- Calls `brp_all_type_schemas` to get full schemas
- Transforms to minimal format (only path names)
- Loses all example data

#### New Behavior
- Calls `brp_all_type_schemas` to get full schemas
- Preserves ENTIRE response structure
- Adds test metadata to each type entry
- Stores complete schema as baseline

#### Specific Changes
```markdown
<FileTransformation>
    Transform the BRP response by ADDING test metadata, not extracting paths:

    Execute the transformation script:
    ```bash
    .claude/commands/scripts/create_mutation_test_json_augment_response.sh [FILEPATH] [TARGET_FILE]
    ```

    The script augments the full BRP response with test metadata for each type:
    - Preserves ALL original schema data (mutation_paths with examples, spawn_format, etc.)
    - Adds: batch_number: null
    - Adds: test_status: "untested" (or "passed" for auto-pass types)
    - Adds: fail_reason: ""

    **File Structure**: The file is the COMPLETE BRP response with added test fields
</FileTransformation>
```

### 2. Update `mutation_test.md`

#### Current Behavior
- Reads type names and paths from minimal format
- Gets fresh schemas during testing
- May generate incorrect examples

#### New Behavior
- Reads complete schemas from `all_types.json`
- Uses stored examples for all mutations
- Never calls `brp_type_schema` during testing (uses stored data)

#### Specific Changes

**Step 1: Load Type Data**
```markdown
For each type in batch:
1. Read complete type info from all_types.json (includes full schema)
2. Extract spawn_format for spawn tests
3. Extract mutation_paths WITH their examples for mutation tests
4. Use EXACT examples from stored schema
```

**Subagent Instructions Update**
```markdown
<TestInstructions>
The main agent will provide you with COMPLETE type schemas including examples.
DO NOT call brp_type_schema - use the provided schema data.

Your assigned types with full schemas:
[FULL TYPE SCHEMA DATA HERE - includes mutation paths with examples]

For mutations, use the EXACT example values provided in the schema.
</TestInstructions>
```

### 3. Script Changes

#### A. New Script: `create_mutation_test_json_augment_response.sh`
**Purpose**: Augment full BRP response with test metadata
**Input**: Raw BRP response JSON, target file path
**Process**:
```bash
#!/bin/bash
# Read full BRP response
# For each type in result.type_info:
#   - Keep ALL existing fields
#   - Add batch_number: null
#   - Add test_status: "untested" or "passed" (based on spawn-only types)
#   - Add fail_reason: ""
# Output augmented JSON maintaining full schema structure
```

#### B. Update: `create_mutation_test_json_structured_comparison.sh`
**Current**: Compares path lists
**New**: Compare full schemas including examples
```bash
# Compare not just paths but actual example values
# Detect changes in:
#   - Mutation path lists
#   - Example formats (object vs array)
#   - Spawn formats
#   - Type registration changes
# Report detailed differences in examples
```

#### C. Update: `mutation_test_renumber_batches.sh`
**Current**: Updates batch numbers in minimal format
**New**: Updates batch numbers in full schema format
```bash
# Navigate nested structure to update batch_number field
# Preserve all schema data while updating test metadata
```

#### D. Update: `mutation_test_merge_batch_results.sh`
**Current**: Updates test_status in minimal format
**New**: Updates test metadata in full schema format
```bash
# For each result:
#   - Find type in full schema structure
#   - Update test_status and fail_reason
#   - Preserve ALL schema data
```

#### E. Update: `mutation_test_get_batch_types.py`
**Current**: Returns type names for batch
**New**: Returns full type schemas for batch
```python
# Instead of returning just type names
# Return complete type info including:
#   - Full mutation_paths with examples
#   - spawn_format
#   - All schema data
# This becomes input to subagents
```

### 4. Comparison Enhancement

The comparison system should now detect:
1. **Path Changes**: New/removed mutation paths
2. **Format Changes**: Example value format changes (object → array)
3. **Type Changes**: Changes in field types
4. **Default Changes**: Changes in example values
5. **Spawn Format Changes**: Changes in spawn/insert structure

Example comparison output:
```
Type: extras_plugin::TestComplexComponent
  Path: .points[0]
    OLD: {"x": 3.14, "y": 3.14, "z": 3.14}
    NEW: [1.0, 2.0, 3.0]
    CHANGE: Format changed from object to array
```

### 5. Migration Strategy

#### Phase 1: Script Development
1. Create new augmentation script
2. Update comparison script to handle both formats
3. Update batch management scripts

#### Phase 2: Test System Update
1. Update `create_mutation_test_json.md` workflow
2. Update `mutation_test.md` to use stored schemas
3. Test with small batch

#### Phase 3: Baseline Recreation
1. Run full type discovery with new system
2. Create new baseline with full schemas
3. Archive old minimal-format files

#### Phase 4: Validation
1. Run comparison between old and new baselines
2. Verify Vec3 format change is detected
3. Run sample mutation tests with stored examples

## Benefits of This Approach

1. **Complete Fidelity**: Never lose information from BRP
2. **Change Detection**: Can detect any change in behavior, not just structure
3. **Single Source of Truth**: The stored schema IS the test specification
4. **Debugging**: Full information available for troubleshooting
5. **Reproducibility**: Tests use exact same examples every time
6. **Historical Comparison**: Can see exactly what changed between versions

## Implementation Priority

1. **CRITICAL**: Create augmentation script (not extraction)
2. **HIGH**: Update comparison to detect example changes
3. **HIGH**: Update mutation test to use stored schemas
4. **MEDIUM**: Update batch management scripts
5. **LOW**: Create migration tools for old format

## Success Criteria

- [ ] Full BRP schemas are preserved in all_types.json
- [ ] Mutation tests use examples from stored schemas
- [ ] Comparisons detect format changes (like Vec3 fix)
- [ ] No information loss from BRP response
- [ ] Test reproducibility improved
- [ ] Historical tracking includes full schemas

## Notes

The key insight is that `all_types.json` should be the full BRP response PLUS test metadata, not a reduction of it. This maintains complete fidelity while adding the tracking we need.