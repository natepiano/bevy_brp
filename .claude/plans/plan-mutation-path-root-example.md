# Plan: Simplify Nested Enum Mutations with Direct Root Examples

## Problem Statement

Currently, nested enum mutation paths require complex multi-step instructions that are difficult for AI agents to follow correctly. Looking at `TestVariantChainEnum.json`, we can see the core issue:

### Current Complex Multi-Step Approach

For the mutation path `.middle_struct.nested_enum.name` (lines 191-228), the system provides:

```json
{
  "path_info": {
    "enum_instructions": "`.middle_struct.nested_enum.name` mutation path requires 2 variant selections. Follow the instructions in variant_path array to set each variant in order.",
    "enum_variant_path": [
      {
        "instructions": "Mutate 'root' mutation 'path' to the 'TestVariantChainEnum::WithMiddleStruct' variant using 'variant_example'",
        "path": "",
        "variant_example": {
          "WithMiddleStruct": {
            "middle_struct": {
              "nested_enum": {
                "VariantA": 1000000  // ❌ WRONG VARIANT!
              }
            }
          }
        }
      },
      {
        "instructions": "Mutate '.middle_struct.nested_enum' mutation 'path' to the 'BottomEnum::VariantB' variant using 'variant_example'",
        "path": ".middle_struct.nested_enum",
        "variant_example": {
          "VariantB": {
            "name": "Hello, World!",
            "value": 3.1415927410125732
          }
        }
      }
    ]
  }
}
```

### The Critical Problems

1. **Wrong Variant in Root Example**: The root example shows `VariantA: 1000000` (line 204) but the mutation path `.name` only exists in `VariantB`. This is fundamentally incorrect.

2. **Complex Multi-Step Process**: Agents must follow a 2-step process:
   - First set the root to `WithMiddleStruct` variant (but with wrong nested enum)
   - Then set the nested enum to the correct `VariantB`

3. **Cognitive Overhead**: The `enum_variant_path` array format requires agents to understand and execute multiple mutations in sequence, increasing failure rates.

### What Should Happen Instead

For `.middle_struct.nested_enum.name`, the system should provide a **single, correct root example**:

```json
{
  "path_info": {
    "enum_instructions": "Use the provided root_example to enable this mutation path",
    "root_example": {
      "WithMiddleStruct": {
        "middle_struct": {
          "nested_enum": {
            "VariantB": {  // ✅ CORRECT VARIANT!
              "name": "Hello, World!",
              "value": 3.1415927410125732
            }
          },
          "some_field": "Hello, World!",
          "some_value": 3.1415927410125732
        }
      }
    }
  }
}
```

This allows agents to:
1. **Single Mutation**: Set the root to the provided example (one operation instead of two)
2. **Correct Structure**: The example has the right variant chain built-in
3. **Direct Path Usage**: Then directly mutate `.middle_struct.nested_enum.name` as intended

## Goal

Replace the complex `enum_variant_path` array approach with a simple `root_example` that provides the complete, correct structure needed for each mutation path.
