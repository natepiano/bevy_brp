# Plan: Remove wrapper_types.rs and Fix TypeKind Inconsistencies

## Problem Analysis

The current system has multiple inconsistencies with Bevy's actual BRP schema generation:

1. **Unused TypeKind::Option**: Our `TypeKind` enum includes `Option` but Bevy BRP classifies all `Option<T>` as `"kind": "Enum"`
2. **Redundant wrapper system**: `wrapper_types.rs` creates hardcoded examples instead of using proper recursive enum handling
3. **Inconsistent examples**: Generic placeholders like `{"Strong": null}` instead of proper recursive examples

**Evidence from investigation:**
- Bevy's `SchemaKind` has NO `Option` variant - only `Struct`, `Enum`, `Map`, `Array`, `List`, `Tuple`, `TupleStruct`, `Set`, `Value`
- `Option<T>` gets `"kind": "Enum"` with `None`/`Some(T)` variants  
- `Handle<T>` gets `"kind": "Enum"` with `Strong`/`Weak` variants
- Both should use standard enum handling, not special wrapper logic

## Solution: Align with Bevy BRP Schema Generation

### Step 1: Remove TypeKind::Option
**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/response_types.rs`
**Action:** Remove `Option` variant from `TypeKind` enum (line ~715)

### Step 2: Remove TypeKind::Option Handling  
**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
**Actions:**
- Remove `TypeKind::Option` case from `type_supports_mutation_with_depth` (~295)
- Remove `TypeKind::Option` case from `TypeKind::build_paths` (~350, ~372)
- Remove `extract_option_inner_type` method (unused)

### Step 3: Remove wrapper_types.rs System
**Files to delete:**
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/wrapper_types.rs`

**Files to update:**
- `mod.rs` - Remove wrapper_types module
- `type_info.rs` - Remove WrapperType imports and detection (lines ~20, ~312-322, ~424-425)
- `mutation_path_builders.rs` - Remove WrapperType usage (~29, ~102, ~110, ~146, ~849, ~917, ~961, ~984)

### Step 4: Enhance Enum Handling for Option Semantics
**Special requirement:** Option fields need `null` → `None` mutation support
**File:** `mutation_path_builders.rs`
**Action:** Add Option-specific mutation logic in `EnumMutationBuilder` to preserve `null` handling

### Step 5: Clean Up type_info.rs Example Building
**File:** `type_info.rs`
**Location:** `build_example_value_for_type_with_depth` method (lines ~312-322)
**Action:** Remove wrapper detection block - let recursive enum handling build proper examples

### Step 6: Update Mutation Path Context
**File:** `mutation_path_builders.rs`
**Actions:**
- Remove `wrapper_info` field from `MutationPathContext` 
- Remove wrapper_info parameters from path building methods
- Simplify `try_build_hardcoded_paths` to not check wrapper detection

## Expected Outcomes

**Before (Inconsistent/Hardcoded):**
```json
"example": {
  "strong": {"Strong": [{"Strong": null}]},
  "weak_placeholder": {"Weak": [{}]}
}
```

**After (Proper Recursive Enum):**
```json  
"example": {
  "Strong": [{"Uuid": {"uuid": "example-uuid"}}],
  "Weak": [{"Uuid": {"uuid": "example-uuid"}}]
}
```

**Option Mutation Preserved:**
- `field = null` still sets `Option::None` (handled by enum logic)
- `field = value` still sets `Option::Some(value)`

## Implementation Steps

1. Remove `TypeKind::Option` enum variant and all its handling
2. Delete wrapper_types.rs file and remove module references
3. Clean wrapper detection from example building and mutation contexts
4. Ensure enum handling properly supports Option mutation semantics  
5. Test that Handle/Option examples show proper recursive structures
6. Verify no regression in existing functionality

## Risk Mitigation

- **Preserve Option mutation**: Ensure `null` assignments still work for Option fields
- **Maintain Handle functionality**: Strong/Weak variants must serialize correctly
- **No breaking changes**: JSON output format should improve, not break
- **Test coverage**: Verify examples show actual inner type structures instead of placeholders