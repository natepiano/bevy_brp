# Plan: Update plan-recursion.md with Correct Implementation Details

## Purpose
Update plan-recursion.md to reflect similar migrated implementation in MapMutationBuilder and SetMutationBuilder, fixing incorrect method names, PathKind structures, and adding migration guidance for unmigrated builders.

## Critical Corrections Needed

### 1. Global Method Name Changes
**Problem**: plan-recursion.md references `include_child_paths()` returning `bool`
**Reality**: Actual implementation uses `child_path_action()` returning `PathAction` enum

**Changes Required**:
- Line 55-64: Replace entire `include_child_paths()` method definition
- Line 149: Update ProtocolEnforcer to call `child_path_action()`
- Lines 454, 464: Update completed builder notes
- Line 884: Update migration checklist

### 2. PathKind Structure Corrections
**Problem**: Examples show wrong PathKind fields
**Reality**: PathKind has `type_name` and `parent_type`, not `field_type` and `optional`

**Example of WRONG (current plan-recursion.md)**:
```rust
PathKind::StructField {
    field_name: "key",
    field_type: key_t,
    optional: false,
}
```

**Example of CORRECT (actual implementation)**:
```rust
PathKind::StructField {
    field_name: "key",
    type_name: key_t,
    parent_type: ctx.type_name().clone(),
}
```

### 3. Method Signature Corrections
**Line 44-47**: Update `collect_children` to return `Result<Vec<PathKind>>` not `Vec<(String, RecursionContext)>`

## Sections to KEEP (Valuable Content)

### Preserve These Sections Entirely:
1. **Lines 1-43**: Current state and overview - still accurate
2. **Lines 70-241**: ProtocolEnforcer implementation (except line 149)
3. **Lines 242-253**: Phase 5a completion notes
4. **Lines 254-321**: ExampleBuilder removal overview and patterns
5. **Lines 322-342**: Migration pattern for each builder - excellent guidance
6. **Lines 344-379**: DefaultMutationBuilder completion notes
7. **Lines 443-467**: MapMutationBuilder and SetMutationBuilder completion notes
8. **Lines 642-703**: Cleanup steps 25-31
9. **Lines 704-856**: Phase 6 and 7 future plans
10. **Lines 857-949**: Complete execution order and end result

## Sections to UPDATE

### 1. Trait Method Definitions (Lines 55-64)
**REPLACE**:
```rust
fn include_child_paths(&self) -> bool {
    true  // Default: include child paths
}
```

**WITH**:
```rust
/// Controls path creation action for child elements
///
/// Container types (Map, Set) that only support whole-value replacement
/// should return PathAction::Skip to prevent exposing invalid mutation paths
/// for child elements that cannot be individually addressed through BRP's
/// reflection system.
///
/// Default: PathAction::Create (include child paths for structured types)
fn child_path_action(&self) -> PathAction {
    PathAction::Create  // Default: create paths for structured types like Struct, Array, Tuple
}
```

### 2. ProtocolEnforcer (Line 149)
**REPLACE**:
```rust
if self.inner.include_child_paths() {
```

**WITH**:
```rust
if matches!(self.inner.child_path_action(), PathAction::Create) {
```

### 3. Completed Builder Notes (Lines 454, 464)
**REPLACE**: All mentions of `include_child_paths() -> false`
**WITH**: `child_path_action() -> PathAction::Skip`

### 4. StructMutationBuilder Example (Lines 380-441)
Update the example to show correct implementation:
- Fix PathKind structure
- Show `Result<Vec<PathKind>>` return type
- Remove any mention of `include_child_paths()`

### 5. Each Unmigrated Builder Section (Lines 468-640)
For **ListMutationBuilder**, **ArrayMutationBuilder**, **TupleMutationBuilder**, **StructMutationBuilder**, **EnumMutationBuilder**

**ADD at the beginning of each section**:

```markdown
**📋 MIGRATION GUIDANCE - FOLLOW THESE EXACT PATTERNS:**

✅ **Reference Implementations** (study these for the exact pattern):
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`

**Key Implementation Details:**
1. **collect_children()** must return `Result<Vec<PathKind>>` (NOT Vec<(String, RecursionContext)>)
2. **PathKind structure** uses:
   - `type_name` (NOT field_type)
   - `parent_type` (NOT optional)
3. **assemble_from_children()** returns `Result<Value>` for proper error handling
4. **For container types** that skip child paths: Override `child_path_action()` to return `PathAction::Skip`

**Pattern Summary:**
- Implement ONLY: `collect_children()` and `assemble_from_children()`
- Optional: Override `child_path_action()` if this type shouldn't expose child paths
- ProtocolEnforcer handles ALL: depth checks, registry validation, mutation status
```

**THEN keep existing removal instructions but enhance**:

```markdown
**🗑️ CODE TO REMOVE (ProtocolEnforcer handles these):**
- ❌ ALL lines with `depth.exceeds_limit()`
- ❌ ALL `ctx.require_registry_schema() else` blocks creating NotMutable paths
- ❌ ENTIRE `build_not_Mutable_path` method
- ❌ ALL `mutation_status` and `mutation_status_reason` field assignments
- ❌ ALL `NotMutableReason` imports and usage
- ❌ ALL direct `BRP_MUTATION_KNOWLEDGE` lookups

**♻️ CODE TO ADAPT (keep logic but change format):**
- ✏️ Schema extraction → Keep but return PathKinds with correct structure
- ✏️ Child identification → Convert to PathKind format with type_name/parent_type
- ✏️ For arrays/lists: Create indexed PathKinds
- ✏️ For structs: Create field PathKinds with parent_type
- ✏️ For enums: Create variant PathKinds
```

## New Section to ADD (After Line 640)

```markdown
## 🎯 Responsibilities After Migration

### ProtocolEnforcer Now Handles ALL:
1. **Depth limit checking** - No builder should check depth
2. **Registry validation** - No builder should validate registry presence
3. **Knowledge lookups** - No builder should access BRP_MUTATION_KNOWLEDGE
4. **NotMutable path creation** - Builders return errors, never create paths
5. **Mutation status computation** - Computed from child statuses
6. **Child path filtering** - Via `child_path_action()` method

### Builders ONLY Handle:
1. **Identifying children** → Return `Result<Vec<PathKind>>` from `collect_children()`
2. **Assembling examples** → Return `Result<Value>` from `assemble_from_children()`
3. **Path control (optional)** → Override `child_path_action()` for containers

### Critical Pattern:
```rust
// Migrated builder pattern (Map/Set as examples)
impl MutationPathBuilder for SomeBuilder {
    fn is_migrated(&self) -> bool { true }

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        // Extract children, return PathKinds with type_name/parent_type
    }

    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<String, Value>) -> Result<Value> {
        // Assemble parent from children
    }

    // Optional: for containers that don't expose child paths
    fn child_path_action(&self) -> PathAction {
        PathAction::Skip  // For Map, Set, etc.
    }
}
```
```

## Migration Checklist Line (884)
**REPLACE**:
```markdown
- **For Map and Set only**: Override include_child_paths() -> false with explanatory comment
```

**WITH**:
```markdown
- **For Map and Set only**: Override child_path_action() -> PathAction::Skip with explanatory comment
```

## Implementation Steps

1. **First Pass - Global Replacements**:
   - Find/replace all `include_child_paths` with `child_path_action`
   - Find/replace `-> bool` with `-> PathAction` for this method
   - Find/replace `false` with `PathAction::Skip` in this context
   - Find/replace `true` with `PathAction::Create` in this context

2. **Second Pass - Structural Fixes**:
   - Fix trait method definitions (lines 55-64)
   - Fix collect_children signature (line 44-47)
   - Fix ProtocolEnforcer call (line 149)
   - Fix StructMutationBuilder example (lines 380-441)

3. **Third Pass - Add Migration Guidance**:
   - Add migration guidance box to each unmigrated builder section
   - Ensure each references Map/Set as examples
   - Emphasize correct PathKind structure

4. **Fourth Pass - Add New Section**:
   - Add "Responsibilities After Migration" section after line 640
   - Include the critical pattern example

5. **Final Pass - Validation**:
   - Verify all method names are correct
   - Verify all PathKind examples use type_name/parent_type
   - Verify all signatures match actual implementation
   - Ensure Map/Set builders are consistently referenced as the pattern to follow

## Expected Outcome

After these updates, plan-recursion.md will:
1. Accurately reflect the actual implementation
2. Provide clear migration guidance for remaining builders
3. Emphasize Map/Set as reference implementations
4. Clearly separate ProtocolEnforcer vs Builder responsibilities
5. Use correct method names and structures throughout
6. Preserve all valuable planning and documentation work already done

## Why This Matters

The current plan-recursion.md would mislead anyone trying to migrate the remaining builders because:
- They'd implement the wrong method (`include_child_paths` instead of `child_path_action`)
- They'd use the wrong PathKind structure
- They'd return the wrong type from collect_children
- They wouldn't know to look at Map/Set as examples

This update ensures the plan accurately guides the remaining migrations.
