# Fix PathRequirement Context Examples

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

**PREREQUISITE**: `plan-variant-chain-infrastructure.md` has been completed successfully. The variant_chain infrastructure provides `ctx.variant_chain` and has already solved 2/3 of the PathRequirement issues:
- ✅ **Complete variant_path chains**: Working correctly
- ✅ **Correct descriptions**: Generated properly from variant chains
- ❌ **Complete example structure**: Still shows local values instead of nested structure

### Step 1: Add Complete Example Construction Helper ⏳ PENDING
**Objective**: Build complete nested PathRequirement examples from variant_chain + schemas
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Change Type**: Additive
**Build Command**: `cargo build && cargo +nightly fmt`

**Key Changes:**
- Add helper method to construct complete examples from `ctx.variant_chain`
- Replace `example.clone()` in PathRequirement construction (line 500)
- Handle path parsing, variant signature lookup, and structure navigation

### Step 2: Add Parent Wrapping Coordination ⏳ PENDING
**Objective**: Coordinate PathRequirement example wrapping during recursive pop-back
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Change Type**: Additive
**Dependencies**: Requires Step 1
**Build Command**: `cargo build && cargo +nightly fmt && cargo nextest run`

**Key Changes:**
- Add wrapping logic after `assemble_from_children` in `build_paths` method
- Coordinate multi-level wrapping across recursion levels
- Handle timing and error cases properly

## Problem Statement

**STATUS UPDATE**: After implementing variant_chain infrastructure, we have achieved significant progress:

### What's Now Working ✅

For `.nested_config.0` path, we're getting:
```json
{
  "path_requirement": {
    "description": "To use this mutation path, root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional",  // ✅ FIXED: Complete description
    "example": 1000000,  // ❌ REMAINING ISSUE: Just the raw value!
    "variant_path": [  // ✅ FIXED: Complete variant chain
      {"path": "", "variant": "TestEnumWithSerDe::Nested"},
      {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
    ]
  }
}
```

### What Should Happen (from reference JSON lines 98-110)

```json
{
  "path_requirement": {
    "description": "To use this mutation path, the root must be set to TestEnumWithSerDe::Nested and .nested_config must be set to NestedConfigEnum::Conditional",
    "example": {
      "Nested": {
        "nested_config": {"Conditional": 1000000},
        "other_field": "Hello, World!"
      }
    },
    "variant_path": [
      {"path": "", "variant": "TestEnumWithSerDe::Nested"},
      {"path": ".nested_config", "variant": "NestedConfigEnum::Conditional"}
    ]
  }
}
```

### The Remaining Issue

**Only one issue remains**: Wrong example format - shows just `1000000` instead of the complete nested structure needed to access that path.

## Core Issue (Updated)

The variant_chain infrastructure solved the ancestry tracking, but PathRequirement examples are still built with only local context:

**Current PathRequirement construction (builder.rs:500):**
```rust
example: example.clone(), // Just uses local value (1000000)
```

**What we need:**
The complete nested structure showing how to reach the mutation path through all required enum variants.

## Solution: Complete Example Construction from variant_chain

**Approach**: Use the available `ctx.variant_chain` + registry schemas to build complete nested examples during PathRequirement construction. This can be done either:

1. **Direct construction**: Build complete example from variant_chain when creating PathRequirement
2. **Parent wrapping**: During recursive pop-back, parents wrap children's PathRequirement examples

We'll use the parent wrapping approach as it leverages the already-assembled complete parent examples.

## Implementation

The implementation leverages the existing variant_chain infrastructure and adds parent wrapping logic during recursive pop-back.

### Core Approach

1. **PathRequirement creation** continues to use `ctx.variant_chain` (already working)
2. **Parent wrapping logic** added after `assemble_from_children` to build complete examples
3. **Multi-level coordination** handles nested enum structures

### Key Algorithm

**Parent Wrapping Process:**
1. Parent completes assembly from all children → has complete example
2. Parent identifies children with PathRequirements needing wrapping
3. Parent builds complete examples by replacing specific fields with child's PathRequirement example
4. Process repeats recursively up the stack

**Example for `.nested_config.0`:**
1. `.nested_config.0` creates PathRequirement with `example: 1000000`
2. `.nested_config` assembles `{"Conditional": 1000000}`, wraps child's example
3. Root assembles complete structure, wraps child's example again
4. Final result: `{"Nested": {"nested_config": {"Conditional": 1000000}, "other_field": "Hello, World!"}}`

This approach leverages already-assembled parent examples instead of trying to construct complete examples from scratch using variant_chain traversal.