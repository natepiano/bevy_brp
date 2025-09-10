# Interactive Plan: Schema Error Handling for Mutation Path Builders

## Overview
Replace panics and incorrect fallback values in mutation path builders with proper error handling using the existing error types:
- `Error::InvalidState` - For protocol violations (e.g., missing required children)
- `Error::SchemaProcessing` - For schema extraction and processing issues

## COMPREHENSIVE CHANGES REQUIRED
- **2 panics to replace** (map_builder.rs:35, default_builder.rs:30)
- **2 incorrect fallback returns** (map_builder.rs:118,126)
- **14 json! fallback returns** that may need error handling
- **30+ unwrap_or/unwrap_or_else calls** to review
- **50+ tracing calls** to clean up
- **4 assemble_from_children implementations** to update (Map, Default, ProtocolEnforcer + trait)
- **2 assemble_from_children callers** to update (in ProtocolEnforcer)

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for "go ahead" from user

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & INSTALL**: Execute the build process:
   ```bash
   cargo build && cargo +nightly fmt && cargo install --path mcp
   ```
   Then inform user to run: `/mcp reconnect brp`

5. **VALIDATE**: Wait for user to confirm the build succeeded

6. **TEST** (if applicable): Run validation tests specific to that step

7. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

8. **PROCEED**: Move to next step only after confirmation
</Instructions>

## INTERACTIVE IMPLEMENTATION SEQUENCE

### STEP 1: Update Error Type
**Status:** ✅ COMPLETED

**Objective:** Enhance SchemaProcessing error to support structured information

**Changes to make:**
1. Update `Error::SchemaProcessing` from simple String to structured fields
2. Update Debug implementation
3. Add builder methods for convenience

**Files to modify:**
- `/mcp/src/error.rs`

**Expected outcome:**
- Code compiles successfully
- No functional changes yet (SchemaProcessing isn't used anywhere)

### STEP 2: Update MutationPathBuilder Trait
**Status:** ✅ COMPLETED

**Objective:** Change trait to support error propagation

**Changes to make:**
1. Change `assemble_from_children` signature to return `Result<Value>`
2. Update default implementation to wrap return in `Ok(...)`

**Files to modify:**
- `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`

**Expected outcome:**
- Code compiles (default impl prevents breakage)
- Sets foundation for error propagation

### STEP 3: Update MapMutationBuilder Completely
**Status:** ✅ COMPLETED

**Objective:** Fully migrate MapMutationBuilder to new error handling

**Changes to make:**
1. Replace panic at line 35 with `Err(Error::InvalidState(...))`
2. Fix line 118: Replace `json!({"example_key": "example_value"})` with error
3. Fix line 126: Replace `json!({"example_key": "example_value"})` with error
4. Update `assemble_from_children` signature to `Result<Value>`
5. Wrap successful return in `Ok(...)`
6. Remove/downgrade excessive `tracing::warn!` calls

**Files to modify:**
- `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`

**Expected outcome:**
- MapMutationBuilder fully migrated with no panics or placeholder values
- Cleaner logging

### STEP 4: Update DefaultBuilder Completely
**Status:** ✅ COMPLETED

**Objective:** Fully migrate DefaultBuilder to new error handling

**Changes to make:**
1. Replace panic at line 30 with `Err(Error::InvalidState(...))`
2. Change `assemble_from_children` return type to `Result<Value>`
3. Wrap `json!(null)` return in `Ok(...)`

**Files to modify:**
- `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/default_builder.rs`

**Expected outcome:**
- DefaultBuilder fully migrated with no panics
- Matches new trait signature

### STEP 5: Update ProtocolEnforcer to Handle Results
**Status:** ✅ COMPLETED

**Objective:** Complete error propagation chain

**Changes to make:**
1. Line 104: Add `?` operator to propagate errors
2. Update `assemble_from_children` signature to `Result<Value>`
3. Line 67: Handle missing schema properly
4. Line 90: Handle missing child example properly
5. Remove all 6 `tracing::warn!` calls (debug traces)

**Files to modify:**
- `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Expected outcome:**
- Full error propagation working for both migrated builders
- System ready for testing
- Cleaner logs

### STEP 6: Review Other Builders' Fallbacks
**Status:** 🔄 DEFERRED - Will be addressed during builder migrations in plan-recursion.md

**Objective:** Investigate and fix other potential fallback issues

**Deferred Reason:**
These builders (SetMutationBuilder, ListMutationBuilder, ArrayMutationBuilder, TupleMutationBuilder, StructMutationBuilder, EnumMutationBuilder) will be completely migrated to the new recursion protocol as part of plan-recursion.md. During those migrations, all `json!(...)` fallback returns will be reviewed and replaced with proper error handling where appropriate.

**Files to review (during migration):**
- `list_builder.rs:113`
- `tuple_builder.rs:193`
- `enum_builder.rs:592, 597`
- `array_builder.rs:139`
- `struct_builder.rs:302, 306, 509, 513, 544, 548`
- `set_builder.rs:58`

**Expected outcome:**
- All error conditions properly handled during builder migrations
- Legitimate fallbacks documented

### STEP 7: Clean Up Excessive Logging
**Status:** ✅ COMPLETED

**Objective:** Remove debug logging from production code

**Changes to make:**
1. **Batch 1**: Remove 30+ `tracing::error!` from struct_builder.rs
2. **Batch 2**: Remove 20+ `tracing::error!` from enum_builder.rs

**Files to modify:**
- `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`
- `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`

**Expected outcome:**
- Clean production logs
- Better performance

### STEP 8: Final Validation
**Status:** ✅ COMPLETED

**Objective:** Verify all changes work correctly

**Validation checklist:**
- [x] No panics in builder code - replaced with Error::InvalidState
- [x] Errors propagate correctly - Result<Value> flows through chain
- [x] No placeholder values (example_key/example_value) - returns proper errors
- [x] Logging is clean - removed 27+ debug traces
- [x] Tool outputs identical - positive path validated

**Expected outcome:**
- System fully migrated to proper error handling
- Ready for production use

## Phase 1: Update Error Type

### Update `Error::SchemaProcessing` in `/mcp/src/error.rs`
Current definition is too simple - just a String. Need to enhance it to provide more context for mutation path building errors.

**Current:**
```rust
#[error("Schema processing error: {0}")]
SchemaProcessing(String),
```

**Proposed:**
```rust
#[error("Schema processing error: {message}")]
SchemaProcessing {
    message: String,
    type_name: Option<String>,
    operation: Option<String>,
    details: Option<String>,
}
```

Add builder methods:
```rust
impl Error {
    pub fn schema_processing(message: impl Into<String>) -> Self {
        Self::SchemaProcessing {
            message: message.into(),
            type_name: None,
            operation: None,
            details: None,
        }
    }

    pub fn schema_processing_for_type(
        type_name: impl Into<String>,
        operation: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::SchemaProcessing {
            message: format!("Failed to process schema for type"),
            type_name: Some(type_name.into()),
            operation: Some(operation.into()),
            details: Some(details.into()),
        }
    }
}
```

## Phase 2: Update MutationPathBuilder Trait

### Update trait signature to propagate errors properly
The `assemble_from_children` method should return `Result<Value>` instead of `Value`.

**File:** `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs`
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<String, Value>,
) -> Result<Value>;  // Changed from -> Value
```

## Phase 3: Update ALL Builder Implementations

### CRITICAL FALLBACK RETURNS TO FIX

These return placeholder values instead of proper errors:

#### Fallback json! returns that MUST become errors:
1. **map_builder.rs:118** - `return json!({"example_key": "example_value"});` - Missing key child
2. **map_builder.rs:126** - `return json!({"example_key": "example_value"});` - Missing value child

#### Fallback json! returns to REVIEW (may be valid in context):
3. **list_builder.rs:113** - `return json!(null);`
4. **tuple_builder.rs:193** - `return json!(null);`
5. **enum_builder.rs:592** - `return json!(null);`
6. **enum_builder.rs:597** - `return json!("...");`
7. **array_builder.rs:139** - `return json!(null);`
8. **struct_builder.rs:302** - `return json!("...");`
9. **struct_builder.rs:306** - `return json!(null);`
10. **struct_builder.rs:509** - `return json!("...");`
11. **struct_builder.rs:513** - `return json!({});`
12. **struct_builder.rs:544** - `return json!("...");`
13. **struct_builder.rs:548** - `return json!({});`
14. **set_builder.rs:58** - `return json!(null);`

### Functions That Need Updates

#### 1. MapMutationBuilder (`map_builder.rs`)

**`build_paths()`** - Lines 26-39
- Replace `panic!` with:
  ```rust
  Err(Error::InvalidState(format!(
      "MapMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
      ctx.type_name()
  )).into())
  ```

**`assemble_from_children()`** - Lines 104-154
- Replace fallback returns at lines 118 and 126 with:
  ```rust
  return Err(Error::InvalidState(format!(
      "Protocol violation: Map type {} missing required 'key' child example",
      ctx.type_name()
  )).into());
  ```
- Replace complex key serialization error at lines 139-145 with proper error propagation

#### 2. DefaultBuilder (`default_builder.rs`)

**`build_paths()`** - Lines 21-34
- Replace `panic!` at line 30 with:
  ```rust
  Err(Error::InvalidState(format!(
      "DefaultBuilder::build_paths() called directly! Type: {}",
      ctx.type_name()
  )).into())
  ```

**`assemble_from_children()`** - Lines 44-52
- Change return type to `Result<Value>`
- Return `Ok(json!(null))`

#### 3. StructBuilder (`struct_builder.rs`)

**`build_paths()`** - Lines 30-292
- Remove all `tracing::error!` calls (they're debug traces)
- Keep core logic unchanged

#### 4. EnumBuilder (`enum_builder.rs`)

**`build_paths()`** - Lines 345-587
- Remove all `tracing::error!` calls
- At line 334, replace `tracing::warn!` with proper error:
  ```rust
  return Err(Error::SchemaProcessing(format!(
      "Failed to parse enum variant {} for type {} - invalid schema from BRP",
      i, ctx.type_name()
  )).into());
  ```

#### 5. ArrayBuilder (`array_builder.rs`)

**`build_paths()`** - Lines 31+
- Check for panics or unwraps (need to verify)

#### 6. ListBuilder (`list_builder.rs`)

**`build_paths()`** - Lines 31+
- Check for panics or unwraps (need to verify)

#### 7. SetBuilder (`set_builder.rs`)

**`build_paths()`** - Lines 30+
- Check for panics or unwraps (need to verify)

#### 8. TupleBuilder (`tuple_builder.rs`)

**`build_paths()`** - Lines 29+
- Check for panics or unwraps (need to verify)

### Functions in ProtocolEnforcer

**File:** `protocol_enforcer.rs`

**`build_paths()`** - Lines 27-119
- Remove all `tracing::warn!` calls (lines 27, 60, 68, 76, 84, 92)
- Update line 104 to handle Result from `assemble_from_children`:
  ```rust
  let parent_example = self.inner.assemble_from_children(ctx, child_examples)?;
  ```
- Fix line 67: `let child_schema = child_ctx.require_schema().unwrap_or(&json!(null));`
  - Should handle missing schema properly
- Fix line 90: `.unwrap_or(json!(null));`
  - Should handle missing child example

**`assemble_from_children()`** - Lines 131-137
- Update to return `Result<Value>`
- Change line 136 to propagate error:
  ```rust
  self.inner.assemble_from_children(ctx, children)
  ```

## Phase 4: Review All unwrap_or_else Patterns

### Critical unwrap_or_else to review:
1. **protocol_enforcer.rs:67** - `child_ctx.require_schema().unwrap_or(&json!(null))`
2. **protocol_enforcer.rs:90** - `.unwrap_or(json!(null))`
3. **type_kind.rs:57** - Schema type extraction with fallback
4. **list_builder.rs:76** - Item type extraction fallback
5. **tuple_builder.rs:143** - Tuple element fallback
6. **tuple_builder.rs:167** - `tuple_examples.into_iter().next().unwrap_or(json!(null))`
7. **tuple_builder.rs:252** - Another tuple example fallback
8. **tuple_builder.rs:325** - Element fallback
9. **tuple_builder.rs:479** - Schema array fallback
10. **enum_builder.rs:423** - Enum variant example fallback
11. **enum_builder.rs:517** - Another enum variant fallback
12. **enum_builder.rs:640** - Enum example fallback
13. **array_builder.rs:80** - `let size = array_size.unwrap_or(2);`
14. **map_builder.rs:138-145** - Complex key serialization with unwrap_or_else

### .ok() conversions that hide errors:
1. **recursion_context.rs:121** - `.filter_map(|s| s.parse().ok())`
2. **type_kind.rs:56** - `.and_then(|s| s.parse().ok())`
3. **array_builder.rs:192** - `.and_then(|s| s.parse::<usize>().ok())`
4. **array_builder.rs:240** - `size_str.parse().ok()`
5. **array_builder.rs:268** - `.and_then(|s| s.parse::<usize>().ok())`

## Phase 5: Update Call Sites

Any code that calls `assemble_from_children()` needs to handle the `Result` return type:
- `protocol_enforcer.rs` line 104 - Main build_paths method
- `protocol_enforcer.rs` line 136 - ProtocolEnforcer's own assemble_from_children

## Phase 6: Builders to Migrate to New Protocol

Currently only 2 builders are migrated (`is_migrated() = true`):
- DefaultBuilder
- MapMutationBuilder

Need to eventually migrate:
- StructBuilder (currently not migrated)
- EnumBuilder (currently not migrated)
- ArrayBuilder (currently not migrated)
- ListBuilder (currently not migrated)
- SetBuilder (currently not migrated)
- TupleBuilder (currently not migrated)

## Phase 7: Clean Up Excessive Logging

Remove or downgrade to trace level:
- **50+ tracing::error! calls** across all builders
- **20+ tracing::warn! calls** that are really debug info
- All should be either removed or changed to `tracing::trace!`

Files with excessive logging:
- struct_builder.rs (30+ error traces)
- enum_builder.rs (20+ error traces)
- protocol_enforcer.rs (6 warn traces)
- map_builder.rs (already partially cleaned)

## Error Categories

### Use `Error::InvalidState` for:
- Protocol violations (missing required children in assemble_from_children)
- Methods called that shouldn't be (build_paths on migrated builders)
- Invalid builder state

### Use `Error::SchemaProcessing` for:
- Failed schema field extraction
- Missing schema when required
- Invalid schema structure
- Failed type extraction from schema

## Testing Considerations

After implementation:
1. All panics should be replaced with proper error returns
2. All placeholder/fallback values should be replaced with errors
3. Error messages should clearly indicate what went wrong and include the type name
4. The system should fail fast with clear error messages rather than producing incorrect output

## IMPLEMENTATION SUMMARY

### Immediate Critical Fixes (MUST DO):
1. **Update Error::SchemaProcessing** to have structured fields (type_name, operation, details)
2. **Fix 2 panics**: map_builder.rs:35, default_builder.rs:30
3. **Fix 2 wrong fallbacks**: map_builder.rs:118,126 (returning example_key/example_value)
4. **Update trait**: Change `assemble_from_children` to return `Result<Value>`
5. **Update 4 implementations** of assemble_from_children (Map, Default, ProtocolEnforcer + trait)
6. **Update 2 callers** in protocol_enforcer.rs

### Secondary Fixes (SHOULD DO):
1. Review and fix 14 other `json!()` fallback returns
2. Review and fix 14 `unwrap_or` patterns in critical paths
3. Review and fix 5 `.ok()` conversions that hide parse errors
4. Fix protocol_enforcer.rs unwraps at lines 67 and 90

### Cleanup (NICE TO HAVE):
1. Remove/downgrade 50+ tracing::error! calls
2. Remove/downgrade 20+ tracing::warn! calls
3. Consider migrating remaining 6 builders to new protocol

### Files That MUST Be Modified:
1. `/mcp/src/error.rs` - Enhance SchemaProcessing error
2. `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mod.rs` - Update trait
3. `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs` - Fix panic and fallbacks
4. `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/default_builder.rs` - Fix panic
5. `/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs` - Handle Results

### Expected Outcome:
- No more panics in builder code
- No more incorrect placeholder values (example_key/example_value)
- Clear error messages when things go wrong
- Proper error propagation through the builder stack
