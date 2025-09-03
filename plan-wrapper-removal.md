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

#### Step 4a: Add EnumVariant Constructor for Consistency
**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_knowledge.rs`
**Action:** Add constructor method for `EnumVariant` knowledge keys:

```rust
impl KnowledgeKey {
    /// Create an enum variant match key
    pub fn enum_variant(
        enum_type: impl Into<String>,
        variant_name: impl Into<String>, 
        variant_pattern: impl Into<String>
    ) -> Self {
        Self::EnumVariant {
            enum_type: enum_type.into(),
            variant_name: variant_name.into(),
            variant_pattern: variant_pattern.into(),
        }
    }
}
```

#### Step 4b: Remove Generic Option Knowledge and Add Variant-Specific Option::None
**File:** Same file, in the knowledge map initialization
**Actions:**
1. **Remove conflicting generic knowledge:**
   ```rust
   // REMOVE THIS:
   map.insert(
       KnowledgeKey::generic("core::option::Option"),
       MutationKnowledge {
           example_value:  json!(null),
           subfield_paths: None,
           guidance:       KnowledgeGuidance::Teach,
       },
   );
   ```

2. **Add Option::None variant-specific knowledge:**
   ```rust
   // ADD THIS:
   map.insert(
       KnowledgeKey::enum_variant(
           "core::option::Option",
           "None", 
           "None"  // Unit variant pattern
       ),
       MutationKnowledge::simple(json!(null)), // Proper BRP format for None
   );
   ```

#### Step 4c: Update Unit Variant Example Building
**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/response_types.rs`
**Action:** Modify `EnumVariantInfo::build_example()` to check variant-specific knowledge:

```rust
pub fn build_example(&self, registry: &HashMap<BrpTypeName, Value>, enum_type: Option<&BrpTypeName>) -> Value {
    match self {
        Self::Unit(name) => {
            // Check for variant-specific knowledge first
            if let Some(enum_type) = enum_type {
                let variant_key = KnowledgeKey::enum_variant(
                    enum_type.to_string(),
                    name,
                    name  // Unit variant pattern is just the name
                );
                
                if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&variant_key) {
                    return knowledge.example_value.clone();
                }
            }
            // Fall back to default Unit variant behavior
            serde_json::json!(name)
        }
        // ... rest unchanged (Tuple and Struct variants)
    }
}
```

#### Step 4d: Thread Enum Type Through Build Calls  
**File:** Same file
**Action:** Update all callers of `build_example()` to pass enum type parameter through the build chain.

#### Step 4e: Fix Recursion Depth Threading in Enum Example Building
**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builders.rs`
**Critical Issue:** `build_variant_data_example()` calls `TypeInfo::build_example_value_for_type()` which resets to `RecursionDepth::ZERO`, breaking recursion safety.

**Actions:**
1. **Update `build_variant_data_example()` signature** to accept `RecursionDepth`:
   ```rust
   fn build_variant_data_example(
       type_name: &BrpTypeName,
       registry: &HashMap<BrpTypeName, Value>,
       enum_type: Option<&BrpTypeName>,
       variant_name: Option<&str>,
       depth: RecursionDepth,  // ADD: Accept recursion depth
   ) -> Value {
       // ... existing knowledge lookup logic ...
       
       // FIXED: Pass depth through to maintain recursion limits
       TypeInfo::build_example_value_for_type_with_depth(type_name, registry, depth.increment())
   }
   ```

2. **Update all callers** to pass depth parameter:
   - Lines ~695, ~713, ~737: Pass `depth.increment()` to `build_variant_data_example()`
   - Thread depth from `build_enum_example()` down through variant building

3. **Update `response_types.rs`** enum building to use depth-aware methods:
   - Lines ~212, ~230: Replace calls to depth-unaware API with `build_example_value_for_type_with_depth()`

**Risk:** Without this fix, deeply nested `Option<Option<Option<...>>>` or `Handle<Handle<Handle<...>>>` types could cause stack overflow.

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

**Option Type Examples:**
```json
// Option<Vec3> will generate:
"example": {
  "None": null,             // Proper BRP format via variant knowledge
  "Some": [1.0, 2.0, 3.0]   // Recursive type example  
}
```

**Option Mutation Preserved:**
- `field = null` still sets `Option::None` (handled by enum logic)
- `field = value` still sets `Option::Some(value)`

## Implementation Steps

### Phase 1: Preparation (Can Run in Parallel)
**Dependencies:** None
- **1a. Enhance enum handling** (Step 4a-4e from above)
  - Add EnumVariant constructor
  - Add Option::None variant knowledge
  - Update unit variant example building
  - Thread enum type through build calls
  - Fix recursion depth handling
- **1b. Document current behavior**
  - Record current wrapper_info usage patterns
  - Note all TypeKind::Option usage locations

### Phase 2: Usage Cleanup (Must Be Sequential)
**Dependencies:** Phase 1 complete
- **2a. Remove TypeKind::Option usage**
  - Clean `type_supports_mutation_with_depth` (~295)
  - Clean `TypeKind::build_paths` (~350, ~372)
  - Remove `extract_option_inner_type` method
- **2b. Remove wrapper detection from type_info.rs**
  - Clean `build_example_value_for_type_with_depth` (lines ~312-322)
  - Remove WrapperType imports and usage (~20, ~312-322, ~424-425)

### Phase 3: Core Removal (Must Be Sequential)
**Dependencies:** Phase 2 complete
- **3a. Remove TypeKind::Option variant**
  - Remove from TypeKind enum in response_types.rs (~715)
- **3b. Delete wrapper_types.rs**
  - Delete `/wrapper_types.rs` file
  - Remove module reference from `mod.rs`
- **3c. Clean mutation path context**
  - Remove `wrapper_info` field from MutationPathContext
  - Update all method signatures removing wrapper_info parameters
  - Simplify `try_build_hardcoded_paths`

### Phase 4: Validation (Can Run in Parallel)
**Dependencies:** Phase 3 complete, then RESTART REQUIRED
- **4a. Functional testing**
  - Run agentic validation tests (see section above)
  - Verify Handle/Option examples show proper recursive structures
  - Test null → None mutations still work
- **4b. Regression testing**
  - Run existing test suite
  - Verify JSON output improvements don't break compatibility

### Rollback Strategy
- **Before Phase 3a:** Simple code reversion
- **After Phase 3a:** Use Phase 1b documentation to recreate wrapper system
- **Critical checkpoints:** After Phase 2, Phase 3, Phase 4

## Risk Mitigation

- **Preserve Option mutation**: Ensure `null` assignments still work for Option fields
- **Maintain Handle functionality**: Strong/Weak variants must serialize correctly
- **No breaking changes**: JSON output format should improve, not break
- **Test coverage**: Verify examples show actual inner type structures instead of placeholders

## Agentic Validation Testing

### Critical: Stop and Install Before Testing
**IMPORTANT**: After implementing all code changes from Steps 1-6, you must:
1. **STOP coding immediately** 
2. **Request user to exit Claude Code and reinstall** (MCP changes require restart)
3. **Only proceed with validation after restart**

### Validation Test Cases

After restart, validate these specific wrapper types found in `extras_plugin` test environment:

#### Test Case 1: Option<f32> - Basic Option Type
**Command**: `brp_type_schema types=["core::option::Option<f32>"]`
**Expected Before Fix**:
```json
"example": "None"  // String - INCORRECT
"example_variants": {
  "None": "None",  // String - INCORRECT
  "Some": {"Some": 3.14}
}
```
**Expected After Fix**:
```json
"example": {"None": null, "Some": 3.14}  // Proper enum variants
"example_variants": {
  "None": null,  // Proper JSON null - CORRECT
  "Some": {"Some": 3.14}
}
```

#### Test Case 2: Option<Vec2> - Complex Inner Type  
**Command**: `brp_type_schema types=["core::option::Option<glam::Vec2>"]`
**Expected After Fix**:
```json
"example": {"None": null, "Some": [1.0, 2.0]}
"example_variants": {
  "None": null,  // Proper JSON null
  "Some": {"Some": [1.0, 2.0]}  // Proper recursive example
}
```

#### Test Case 3: Handle<Mesh> - Handle Enum Type
**Command**: `brp_type_schema types=["bevy_asset::handle::Handle<bevy_mesh::mesh::Mesh>"]`
**Expected After Fix**:
```json
"example": {"Strong": {...}, "Weak": {...}}  // Proper recursive examples instead of null placeholders
```

### Validation Success Criteria
✅ **Option::None variants return `null` (JSON null) not `"None"` (string)**  
✅ **Option::Some variants show proper recursive inner type examples**  
✅ **Handle types show proper Strong/Weak recursive examples instead of `null` placeholders**  
✅ **All enum `example_variants` contain proper data instead of hardcoded strings**

### Failure Indicators
❌ Any Option None variant returns `"None"` string instead of `null`  
❌ Any Handle type shows `{"Strong": null}` instead of recursive example  
❌ Generic hardcoded knowledge still overriding enum variant generation