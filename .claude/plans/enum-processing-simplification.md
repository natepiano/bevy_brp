# Enum Processing Simplification Plan

## Goal
Eliminate duplicate `process_enum_context` functions by having `enum_path_builder` directly set `MutationPathInternal` fields instead of using a JSON wrapper that requires extraction.

## Current Problem
- Two nearly identical `process_enum_context` functions exist:
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs:469-538` (70 lines)
  - `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs:571-598` (28 lines)
- Both extract data from an `enum_root_data` JSON wrapper structure
- This creates ~100 lines of duplicate code and unnecessary complexity

## Proposed Solution
Since we control what `enum_path_builder` returns, we can eliminate the JSON wrapper entirely and have it directly populate the `MutationPathInternal` fields.

## Implementation Steps

### Step 1: Modify `build_enum_examples` return type
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Lines**: ~362-414

**Current**:
```rust
fn build_enum_examples(...) -> Result<Value> {
    // Returns wrapped JSON:
    json!({
        "enum_root_data": {
            "enum_root_examples": mutation_example,
            "enum_root_example_for_parent": default_example
        }
    })
}
```

**Change to**:
```rust
fn build_enum_examples(...) -> Result<(Vec<ExampleGroup>, Value)> {
    // Return tuple directly:
    Ok((mutation_example, default_example))
}
```

### Step 2: Update `process_enum` to handle new return type
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Lines**: ~114-131

**Current**:
```rust
let assembled_value = build_enum_examples(&variant_groups, child_examples, ctx)?;
Ok(create_result_paths(ctx, assembled_value, child_paths))
```

**Change to**:
```rust
let (enum_examples, default_example) = build_enum_examples(&variant_groups, child_examples, ctx)?;
Ok(create_result_paths(ctx, enum_examples, default_example, child_paths))
```

### Step 3: Modify `create_result_paths` to set fields directly
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Lines**: ~541-568

**Current**:
```rust
fn create_result_paths(
    ctx: &RecursionContext,
    assembled_value: Value,
    child_paths: Vec<MutationPathInternal>,
) -> Vec<MutationPathInternal> {
    let (parent_example, enum_root_examples, enum_root_example_for_parent) =
        process_enum_context(ctx, assembled_value);

    let root_mutation_path = MutationPathInternal {
        example: parent_example,
        enum_root_examples,
        enum_root_example_for_parent,
        // ... other fields
    };
}
```

**Change to**:
```rust
fn create_result_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    child_paths: Vec<MutationPathInternal>,
) -> Vec<MutationPathInternal> {
    let root_mutation_path = MutationPathInternal {
        example: match &ctx.enum_context {
            Some(EnumContext::Root) => json!(null),
            _ => concrete_example(/* ... */)
        },
        enum_root_examples: match &ctx.enum_context {
            Some(EnumContext::Root) => Some(enum_examples),
            _ => None
        },
        enum_root_example_for_parent: match &ctx.enum_context {
            Some(EnumContext::Root) => Some(default_example),
            _ => None
        },
        // ... other fields
    };
}
```

### Step 4: Remove `process_enum_context` from enum_path_builder
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Lines**: 571-598

Delete the entire function.

### Step 5: Update builder.rs to use direct fields
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`

**Line 91-92** - Remove enum context processing:
```rust
// OLD:
let (parent_example, enum_root_examples, enum_root_example_for_parent) =
    Self::process_enum_context(ctx, assembled_example);

// NEW:
let parent_example = assembled_example;
let enum_root_examples = None;  // Only enum types set this
let enum_root_example_for_parent = None;  // Only enum types set this
```

**Lines 348-352** - Simplify child example extraction:
```rust
// OLD:
let child_example = child_paths.first().map_or(json!(null), |p| {
    p.enum_root_example_for_parent
        .as_ref()
        .map_or_else(|| p.example.clone(), Clone::clone)
});

// NEW:
let child_example = child_paths.first().map_or(json!(null), |p| p.example.clone());
```

### Step 6: Remove `process_enum_context` from builder.rs
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Lines**: 469-538

Delete the entire function.

### Step 7: Clean up and test
- Remove any unused imports
- Run `cargo build && cargo +nightly fmt`
- Run `cargo nextest run` to verify tests pass

## Benefits
- Eliminates ~100 lines of duplicate code
- Removes unnecessary JSON wrapper/unwrapper pattern
- Makes data flow direct and clear
- Reduces coupling between enum processing and parent builder
- Simplifies maintenance

## Risks and Mitigation
- **Risk**: Breaking existing functionality that depends on the wrapper
  - **Mitigation**: The wrapper is only used internally between these two files
- **Risk**: Missing edge cases in enum processing
  - **Mitigation**: Existing tests should catch any regressions
- **Risk**: PartiallyMutable status handling might be affected
  - **Mitigation**: This logic remains in builder.rs unchanged (lines 110-113)

## Validation
After implementation:
1. Run all existing tests
2. Test with complex enum scenarios:
   - `Option<T>` types
   - Nested enums
   - Enums in structs
   - Enums in collections
3. Verify mutation path output is unchanged