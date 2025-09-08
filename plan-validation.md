# Plan Validation Procedure

## Overview
This document tracks the validation procedure for the spawn format unification plan implementation. It serves as a reference for tracking progress, comparing outputs, and ensuring the migration maintains correctness while extending functionality.

## Key Metrics to Track

### spawn_format Generation
- **Current Status**: 20 types have spawn_format (only Struct and Tuple types)
- **Goal**: All spawnable types should have spawn_format examples
- **Gap**: 9 types can be spawned but don't get examples (Arrays, Lists, Maps, Sets, Enums, etc.)

### spawn_support vs spawn_format
- **spawn_support**: Whether BRP's `supported_operations` includes "spawn" or "insert"
- **spawn_format**: Whether `TypeInfo::build_spawn_format` generates an actual example
- **Current**: 29 types have spawn_support, only 20 have spawn_format

## Tracked Output Files

### Primary Validation Files (brp_all_type_schemas outputs)
These files contain the full BRP response with spawn_format fields:

1. **`$TMPDIR/all_types_previous_commit.json`** (≈826KB)
   - Baseline from commit before Phase 3 changes
   - Contains 140 types, 20 with spawn_format
   - Use for regression testing

2. **`$TMPDIR/all_types_phase3.json`** (≈904KB)
   - Output with Phase 3 dynamic dispatch implementation
   - Shows enum improvements (more complete examples)
   - Same 20 types with spawn_format

3. **`$TMPDIR/all_types_phase4_complete.json`** (≈1.0MB)
   - Final output with Phase 4 complete
   - Contains 140 types, 28 with spawn_format
   - 96.6% coverage (28/29 spawnable types)
   - Includes Entity fix for ChildOf

4. **Latest MCP response files** (`mcp_response_brp_all_type_schemas*.json`)
   - Current output after each change
   - Compare against baseline for differences

### Secondary Files (Mutation Test Tracking)
These files track spawn_support but NOT actual spawn_format examples:

- **`$TMPDIR/all_types_baseline.json`** (≈60KB) - Mutation test baseline
- **`$TMPDIR/all_types.json`** (≈59KB) - Current mutation test file

**Note**: These are NOT useful for spawn_format validation, only for tracking which types BRP says can be spawned.

## Validation Commands

### 1. Generate Current Output
```bash
mcp__brp__brp_all_type_schemas(port=15702)
```
Save the output filepath for comparison.

### 2. Check spawn_format Coverage
```python
import json

with open('[OUTPUT_FILE]', 'r') as f:
    data = json.load(f)

types_with_spawn = sum(1 for t in data['type_info'].values() if t.get('spawn_format'))
types_with_ops = sum(1 for t in data['type_info'].values() 
                     if 'spawn' in t.get('supported_operations', []) or 
                        'insert' in t.get('supported_operations', []))

print(f'Types with spawn_format: {types_with_spawn}')
print(f'Types with spawn/insert ops: {types_with_ops}')
print(f'Gap (spawnable without examples): {types_with_ops - types_with_spawn}')
```

### 3. Compare Against Baseline
```python
import json

with open('$TMPDIR/all_types_previous_commit.json', 'r') as f:
    baseline = json.load(f)

with open('[NEW_OUTPUT]', 'r') as f:
    current = json.load(f)

# Compare spawn_format counts
baseline_spawn = sum(1 for t in baseline['type_info'].values() if t.get('spawn_format'))
current_spawn = sum(1 for t in current['type_info'].values() if t.get('spawn_format'))

print(f'Baseline spawn_format count: {baseline_spawn}')
print(f'Current spawn_format count: {current_spawn}')

if current_spawn < baseline_spawn:
    print('⚠️ REGRESSION: Fewer types have spawn_format')
elif current_spawn > baseline_spawn:
    print('✅ IMPROVEMENT: More types have spawn_format')
else:
    print('➡️ NO CHANGE: Same spawn_format coverage')
```

### 4. Check Specific Type Examples
```python
test_types = [
    'bevy_transform::components::transform::Transform',  # Should have spawn_format
    'bevy_sprite::sprite::Sprite',                       # Currently missing
    '[f32; 3]',                                         # Array - currently missing
    'alloc::vec::Vec<f32>',                             # List - currently missing
]

for type_name in test_types:
    if type_name in current['type_info']:
        has_spawn = bool(current['type_info'][type_name].get('spawn_format'))
        has_root_mutation = any(p.get('path_info', {}).get('path_kind') == 'RootValue' 
                               for p in current['type_info'][type_name].get('mutation_paths', {}).values())
        print(f'{type_name}: {"✓" if has_spawn else "✗"} spawn_format, {"✓" if has_root_mutation else "✗"} root_mutation')
```

### 5. Check Root Mutation Path Coverage (Phase 3 Validation)
```python
# Count types with mutation paths that now have root mutations
types_with_mutations = [t for t in data['type_info'].values() 
                       if t.get('mutation_paths', {})]

types_with_root_mutation = []
for type_info in types_with_mutations:
    has_root = any(p.get('path_info', {}).get('path_kind') == 'RootValue' 
                   for p in type_info.get('mutation_paths', {}).values())
    if has_root:
        types_with_root_mutation.append(type_info)

print(f'Types with mutation_paths: {len(types_with_mutations)}')
print(f'Types with root mutation: {len(types_with_root_mutation)}')
print(f'Coverage: {len(types_with_root_mutation)/len(types_with_mutations)*100:.1f}%')

# Should be 100% after Phase 3 - all types with mutations should have root mutation
if len(types_with_root_mutation) == len(types_with_mutations):
    print('✅ PHASE 3 SUCCESS: All types with mutations have root mutation path')
else:
    print(f'⚠️ PHASE 3 ISSUE: {len(types_with_mutations) - len(types_with_root_mutation)} types missing root mutation')
```

## Implementation Progress

### Phase 1: Break Circular Dependencies ✅
- Extract inline logic to static methods
- Create ExampleBuilder as temporary scaffolding
- **Status**: Complete

### Phase 2: Add Trait Infrastructure ✅
- Add trait methods to MutationPathBuilder
- Implement build_schema_example in all builders
- **Status**: Complete

### Phase 3: Migrate to Trait Dispatch ✅
- Switch from static to dynamic dispatch
- Fixed infinite recursion in DefaultMutationBuilder
- **Status**: Complete (using dynamic dispatch)
- **Key Change**: All types with mutation paths now have root mutation path (PathKind::RootValue), not just enums

### Phase 4: Extend spawn_format Generation ✅
- Updated TypeInfo::extract_spawn_format_from_paths to use root mutation path examples
- Added Entity to BRP_MUTATION_KNOWLEDGE to fix cascade issues
- **Status**: Complete
- **Result**: 28/29 spawnable types now have spawn_format (96.6% coverage)

## Known Issues

1. **Near-Complete spawn_format Coverage**
   - 28/29 spawnable types now have examples (up from 20/29)
   - Fixed: ChildOf now has spawn_format after adding Entity knowledge
   - Remaining gap: 1 type (identity unknown)

2. **Entity Knowledge Addition**
   - Added `bevy_ecs::entity::Entity` to BRP_MUTATION_KNOWLEDGE
   - Entity serializes as u64, not as struct
   - Includes warning about using valid entity IDs in actual operations

## Success Criteria

1. **No Regressions**: ✅ All 20 types that currently have spawn_format kept it
2. **Extended Coverage**: ✅ 28/29 spawnable types now have spawn_format (96.6% coverage)
3. **Example Quality**: ✅ Generated examples are valid for BRP operations
4. **Performance**: ✅ No significant performance degradation observed

## Next Steps

1. ✅ Phase 4 Complete - spawn_format generation extended via root mutation paths
2. ✅ Used extract_spawn_format_from_paths approach successfully
3. ✅ Added Entity to knowledge base to fix cascade issues
4. Optional: Identify and fix the remaining 1 type without spawn_format

## Testing After Changes

After any changes to example generation:

1. Build and install: `cargo build && cargo install --path mcp`
2. Reload MCP: `/mcp reconnect brp`
3. Run validation: Execute commands from "Validation Commands" section
4. Compare results: Ensure no regressions and check for improvements
5. Save successful outputs as new baselines when extending functionality