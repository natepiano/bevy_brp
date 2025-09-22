# Plan 2: Add Variant Chain Lookup for Correct Root Examples

## Goal
Build on Plan 1's refactoring to provide correct, complete root examples for each mutation path. This will fix the issue where mutating nested enum fields requires multiple steps with incorrect intermediate examples.

## Prerequisite
Plan 1 must be complete - we need ALL variant examples available during recursion.

## The Problem (Recap)

Currently for `.middle_struct.nested_enum.name` in `TestVariantChainEnum`:

```json
"enum_variant_path": [
  {
    "path": "",
    "variant_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": { "VariantA": 1000000 }  // ❌ Wrong variant!
        }
      }
    }
  },
  {
    "path": ".middle_struct.nested_enum",
    "variant_example": { "VariantB": { "name": "...", "value": 3.14 } }
  }
]
```

## The Solution

Provide a single, correct root example:

```json
"variant_example": {
  "WithMiddleStruct": {
    "middle_struct": {
      "nested_enum": {
        "VariantB": {  // ✅ Correct variant!
          "name": "Hello, World!",
          "value": 3.14
        }
      }
    }
  }
}
```

## Architecture

### 1. Track Variant Chains

Add to `MutationPathInternal`:
```rust
pub struct MutationPathInternal {
    // ... existing fields ...

    /// The specific variant chain for this path
    /// E.g., ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantB"]
    pub variant_chain: Vec<String>,

    /// Complete root example with correct variant chain
    pub root_variant_example: Option<Value>,

    // Keep temporarily for migration verification
    pub enum_variant_path: Vec<VariantPath>,
}
```

### 2. Build Variant-to-Example Mapping

During enum assembly, maintain:
```rust
/// Maps variant chains to complete root examples
type VariantChainMap = HashMap<Vec<String>, Value>;
```

Example entries:
- `["WithMiddleStruct", "VariantA"]` → Complete root with VariantA
- `["WithMiddleStruct", "VariantB"]` → Complete root with VariantB
- `["WithMiddleStruct", "VariantC"]` → Complete root with VariantC
- `["Empty"]` → Just the Empty variant

### 3. Propagate Variant Context

During recursion in `builder.rs`:

```rust
fn process_child(...) {
    // Track which variant this child is part of
    if let Some(current_variant) = get_current_variant() {
        child_ctx.variant_chain.push(current_variant);
    }

    // After recursion, child paths have complete variant chains
    let child_paths = recurse_mutation_paths(...);

    // Each path knows its exact variant requirements
}
```

### 4. Assign Root Examples

At the root level assembly:

```rust
fn finalize_mutation_paths(paths: Vec<MutationPathInternal>, variant_map: VariantChainMap) {
    for path in &mut paths {
        // Use the variant chain to lookup the correct root example
        if !path.variant_chain.is_empty() {
            path.root_variant_example = variant_map.get(&path.variant_chain).cloned();
        }
    }
}
```

## Implementation Steps

### Phase 1: Add Variant Chain Tracking

1. Add `variant_chain: Vec<String>` to `MutationPathInternal`
2. Update `RecursionContext` to track current variant
3. Propagate variant information during recursion

### Phase 2: Build Variant Mapping

1. In `enum_builder::assemble_from_children()`:
   - Build examples for each variant combination
   - Create `VariantChainMap` with all combinations
   - Pass this map up through recursion

2. Store mapping at root level for lookup

### Phase 3: Assign Root Examples

1. After collecting all mutation paths
2. For each path with a variant chain
3. Lookup the corresponding root example
4. Attach it to the mutation path

### Phase 4: Update Output Format

Modify `path_info` to include the complete root example:

```rust
"path_info": {
    "mutation_status": "mutable",
    "path_kind": "StructField",
    "type": "String",
    "type_kind": "Value",
    "enum_instructions": "To mutate `.middle_struct.nested_enum.name`, use the provided root_variant_example",
    "root_variant_example": { /* complete example */ }
}
```

## Migration Strategy

1. **Keep Both Systems**: Maintain `enum_variant_path` alongside new `variant_chain`
2. **Verify Correctness**: Compare that variant chains match enum_variant_path entries
3. **Gradual Transition**: Update consumers to use new format
4. **Remove Old System**: Once verified, remove `enum_variant_path`

## Success Criteria

### For `.middle_struct.nested_enum.name`:

1. **Single Mutation**: Only one mutation needed (not two)
2. **Correct Example**: Root example has `VariantB` (not `VariantA`)
3. **Clear Instructions**: Simple message, not complex array
4. **All Paths Work**: Every mutation path has its correct root example

### Testing

1. **Verify Variant Chains**: Each path's variant chain matches its requirements
2. **Lookup Validation**: Correct root examples are assigned
3. **Mutation Test**: Actually perform mutations using the new examples
4. **Coverage**: Test deeply nested enums (3+ levels)

## Example: Complete Transformation

### Before (Current)
```json
{
  "path": ".middle_struct.nested_enum.name",
  "example": "Hello, World!",
  "enum_variant_path": [/* 2-step array */],
  "path_info": { /* complex instructions */ }
}
```

### After (With Plan 2)
```json
{
  "path": ".middle_struct.nested_enum.name",
  "example": "Hello, World!",
  "variant_chain": ["TestVariantChainEnum::WithMiddleStruct", "BottomEnum::VariantB"],
  "path_info": {
    "enum_instructions": "Use the root_variant_example to enable this mutation",
    "root_variant_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": {
            "VariantB": {
              "name": "Hello, World!",
              "value": 3.14
            }
          },
          "some_field": "test",
          "some_value": 42.5
        }
      }
    }
  }
}
```

## Risks

1. **Memory Usage**: Storing all variant combinations could be expensive for deeply nested enums
2. **Complexity**: Tracking variant chains through recursion adds complexity
3. **Breaking Change**: Output format changes may break existing consumers

## Open Questions

1. Should we compress/optimize the variant map for large enums?
2. How do we handle Option<T> which has special formatting rules?
3. Should root_variant_example be at top level or in path_info?