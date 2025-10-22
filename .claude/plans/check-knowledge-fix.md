# Check Knowledge Function Complexity Fix

## Problem

The `check_knowledge` function in `path_builder.rs:577-620` has a semantically opaque return type that encodes multiple distinct control flow decisions in an unnamed tuple.

### Current Return Type
```rust
(
    Option<Result<Vec<MutationPathInternal>, BuilderError>>,  // First element
    Option<Value>,                                             // Second element
)
```

### Four Distinct Semantic Outcomes

1. **Complete immediately** (TreatAsRootValue): `(Some(Ok(vec![...])), None)`
   - Don't recurse into children, return these paths now

2. **Use example and recurse** (TeachAndRecurse): `(None, Some(example))`
   - Continue processing children but use this hardcoded example

3. **No knowledge found**: `(None, None)`
   - Continue with normal processing, assemble example from children

4. **Error**: `(Some(Err(e)), None)`
   - Propagate error from `find_knowledge()`

### Problems with Current Approach

1. **Implicit semantics**: Comment on line 607 admits: *"this is not obvious by the current return type"*

2. **Enum builder doesn't distinguish**: In `enum_path_builder.rs:105-119`, the code treats `TreatAsRootValue` and `TeachAndRecurse` identically:
   ```rust
   let default_example = ctx
       .find_knowledge()
       .ok()
       .flatten()
       .map(|knowledge| knowledge.example().clone())  // ← Just extracts example!
   ```
   This loses the distinction between "stop here" vs "recurse with this example"

3. **Tuple forces mental mapping**: Developers must remember:
   - `(Some(Ok(_)), None)` = early return
   - `(None, Some(_))` = continue with example
   - `(None, None)` = continue without example
   - `(Some(Err(_)), None)` = error

## Solution

### Replace tuple with explicit `KnowledgeAction` enum

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

```rust
/// Action to take based on type knowledge lookup
pub enum KnowledgeAction {
    /// Return immediately with complete paths (TreatAsRootValue knowledge)
    /// Don't recurse into children
    Complete(Vec<MutationPathInternal>),

    /// Use this example but continue recursing children (TeachAndRecurse knowledge)
    UseExampleAndRecurse(Value),

    /// No hardcoded knowledge found - proceed with normal assembly from children
    NoHardcodedKnowledge,
}

pub type KnowledgeCheckResult = std::result::Result<KnowledgeAction, BuilderError>;
```

### Update `check_knowledge` function

```rust
fn check_knowledge(ctx: &RecursionContext) -> KnowledgeCheckResult {
    match ctx.find_knowledge()? {
        Some(TypeKnowledge::TreatAsRootValue { example, .. }) => {
            // Return immediately without recursing
            Ok(KnowledgeAction::Complete(vec![
                Self::build_mutation_path_internal(
                    ctx,
                    PathExample::Simple(example),
                    Mutability::Mutable,
                    None,
                    None,
                )
            ]))
        }
        Some(TypeKnowledge::TeachAndRecurse { example }) => {
            // Use this example but continue recursing children
            Ok(KnowledgeAction::UseExampleAndRecurse(example))
        }
        None => {
            // No knowledge - proceed with normal processing
            Ok(KnowledgeAction::NoHardcodedKnowledge)
        }
    }
}
```

### Update call sites in `path_builder.rs`

**In `build_paths` (around line 92-95):**

```rust
// OLD:
let (knowledge_result, knowledge_example) = Self::check_knowledge(ctx);
if let Some(result) = knowledge_result {
    return result;
}

// NEW:
let knowledge_example = match Self::check_knowledge(ctx)? {
    KnowledgeAction::Complete(paths) => return Ok(paths),
    KnowledgeAction::UseExampleAndRecurse(example) => Some(example),
    KnowledgeAction::NoHardcodedKnowledge => None,
};
```

### Update enum builder in `enum_path_builder.rs`

**Fix lines 105-119 to properly handle TreatAsRootValue:**

```rust
// OLD - treats both knowledge types the same:
let default_example = ctx
    .find_knowledge()
    .ok()
    .flatten()
    .map(|knowledge| knowledge.example().clone())
    .or_else(|| select_preferred_example(&enum_examples))

// NEW - respects TreatAsRootValue by returning early:
let default_example = match ctx.find_knowledge()? {
    Some(TypeKnowledge::TreatAsRootValue { example, .. }) => {
        // This enum itself is opaque - return immediately without processing variants
        return Ok(vec![build_enum_root_path_from_knowledge(ctx, example)]);
    }
    Some(TypeKnowledge::TeachAndRecurse { example }) => {
        // Use this example but still process variants
        Some(example)
    }
    None => {
        // Use preferred example from processed variants
        select_preferred_example(&enum_examples)
    }
}.ok_or_else(|| ...)?;
```

## Alternative Considered: Use `TypeKnowledge` Directly

**Why not just match on `Result<Option<TypeKnowledge>>`?**

```rust
// This would work:
match ctx.find_knowledge()? {
    Some(TypeKnowledge::TreatAsRootValue { ... }) => ...,
    Some(TypeKnowledge::TeachAndRecurse { ... }) => ...,
    None => ...,
}
```

**Downsides:**
1. `TypeKnowledge` represents **static facts** (storage domain), not **control flow decisions** (builder domain)
2. No explicit name for the "no knowledge" case - it's just `None`
3. Less clear what action should be taken for each variant
4. Doesn't convey the "NoHardcodedKnowledge" vs "UseExampleAndRecurse" distinction as clearly

**Benefits of `KnowledgeAction`:**
1. Explicitly names the three control flow paths
2. Self-documenting: variant names say what to do
3. Ergonomic with `?` operator
4. Clear separation between knowledge domain and builder actions

## Files to Modify

1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`
   - Add `KnowledgeAction` enum and type alias
   - Update `check_knowledge` function signature and implementation
   - Update call site in `build_paths` method

2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`
   - Update `process_enum` to handle `TreatAsRootValue` correctly (return early)
   - Update default_example selection logic to distinguish knowledge types

## Testing

Test scenarios:
1. **Enum with TreatAsRootValue knowledge**: Should return early, not process variants
2. **Enum with TeachAndRecurse knowledge**: Should process variants but use knowledge example as default
3. **Non-enum with TreatAsRootValue**: Should return single path without recursing
4. **Non-enum with TeachAndRecurse**: Should recurse into children but use knowledge example
5. **Type without knowledge**: Should recurse and assemble example from children

## Benefits

- **Type safety**: Compiler enforces handling all cases
- **Self-documenting**: Names clearly express intent
- **Fixes enum builder bug**: Now properly distinguishes TreatAsRootValue vs TeachAndRecurse
- **Better maintainability**: Future developers immediately understand control flow
- **Testability**: Each variant can be tested independently

## Open Question

Should `TypeKnowledge` itself be used instead of introducing `KnowledgeAction`? The distinction is:
- `TypeKnowledge` = "what we know about this type" (knowledge domain)
- `KnowledgeAction` = "what the builder should do" (control flow domain)

Current recommendation: Keep the separation for clarity and to maintain the architectural boundary between knowledge storage and builder behavior.
