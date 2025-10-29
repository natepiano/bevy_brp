# Plan: Replace Value with Self-Documenting Example Enum

## Problem Statement

Currently, mutation path examples use `serde_json::Value` directly, with `json!(null)` serving as a sentinel value for multiple semantically different cases:

1. **Option::None** - A legitimate serializable value that BRP interprets as `None`
2. **Not Applicable** - "There is no example because this path isn't mutable"
3. **Actual null** - A regular JSON null value

This overloading makes the code hard to understand and maintain. When you see `json!(null)`, you can't immediately tell which of these three meanings applies.

## Solution

Introduce a self-documenting `Example` enum that wraps `Value` with explicit semantic meaning:

```rust
/// Self-documenting wrapper for example values in mutation paths
#[derive(Debug, Clone)]
pub enum Example {
    /// A regular JSON value (object, array, string, number, bool, or null-as-data)
    Json(Value),

    /// Explicit representation of Option::None (serializes to null for BRP)
    /// Documents that null is intentional and meaningful
    OptionNone,

    /// No example exists (for NotMutable paths)
    /// Documents that we're not providing an example (not that the example is null)
    NotApplicable,
}
```

## Design Principles

### 1. **Core Building Code Works with Example**
- All mutation path builders (path_builder.rs, enum_path_builder.rs) work with `Example`
- All storage structures (`PathExample`, `child_examples` HashMap) use `Example`
- This is where self-documentation provides the most value

### 2. **Type Kind Builders Work with Value**
- Builders like `StructMutationBuilder`, `TupleMutationBuilder` are fundamentally JSON assemblers
- They work with `serde_json::Value` for JSON construction operations
- Conversion happens at the boundary when calling `assemble_from_children()`

### 3. **Serialization Structures Use Value**
- `ExampleGroup` keeps `Option<Value>` for serialization simplicity
- At serialization time, semantic distinctions have already been collapsed
- Conversion happens when creating `ExampleGroup` instances

### 4. **Explicit Conversion Boundaries**
- Use `Example::to_value()` when passing to builders or serialization
- Use `Example::from_value()` or `Example::Json(val)` when wrapping results
- Each conversion point is self-documenting

## Implementation Plan

### Phase 1: Define Core Types

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/example.rs` (new file)

```rust
use serde_json::Value;

/// Self-documenting wrapper for example values in mutation paths
#[derive(Debug, Clone, PartialEq)]
pub enum Example {
    /// A regular JSON value
    Json(Value),

    /// Explicit Option::None (serializes to null)
    OptionNone,

    /// No example available (for NotMutable paths)
    NotApplicable,
}

impl Example {
    /// Convert to Value for JSON operations (assembly, serialization)
    pub fn to_value(&self) -> Value {
        match self {
            Self::Json(v) => v.clone(),
            Self::OptionNone | Self::NotApplicable => Value::Null,
        }
    }

    /// Borrow as Value reference (for zero-copy operations)
    pub fn as_value(&self) -> &Value {
        // Cache a static null value to return references
        static NULL: Value = Value::Null;
        match self {
            Self::Json(v) => v,
            Self::OptionNone | Self::NotApplicable => &NULL,
        }
    }

    /// Check if this represents a null-equivalent value
    pub const fn is_null_equivalent(&self) -> bool {
        matches!(self, Self::OptionNone | Self::NotApplicable)
    }
}

impl From<Value> for Example {
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}

impl From<Example> for Value {
    fn from(example: Example) -> Self {
        example.to_value()
    }
}
```

**Action**: Create this new module and export it from mutation_path_builder/mod.rs

---

### Phase 2: Update PathExample

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_example.rs`

**Changes**:

```rust
use super::example::Example;  // Add import
use super::types::ExampleGroup;

#[derive(Debug, Clone)]
pub enum PathExample {
    /// Simple example value (changed from Value to Example)
    Simple(Example),

    /// Enum root with variant groups
    EnumRoot {
        groups: Vec<ExampleGroup>,
        /// Simplified example for parent assembly (changed from Value to Example)
        for_parent: Example,
    },
}

impl PathExample {
    /// Get the example to use for parent assembly
    pub fn for_parent(&self) -> &Example {  // Changed return type
        match self {
            Self::Simple(ex) => ex,
            Self::EnumRoot { for_parent, .. } => for_parent,
        }
    }
}

// Update Serialize implementation
impl Serialize for PathExample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Simple(example) => {
                let value = example.as_value();  // Convert to Value
                if value.is_null() {
                    serializer.serialize_map(Some(0))?.end()
                } else {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("example", value)?;
                    map.end()
                }
            }
            Self::EnumRoot { groups, .. } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("examples", groups)?;
                map.end()
            }
        }
    }
}
```

**Reasoning**: `PathExample` is the primary storage structure for examples, so it should use the self-documenting `Example` type. Serialization converts to `Value` at the boundary.

**Migration Impact**: The change to `for_parent()` return type from `&Value` to `&Example` affects several callsites. Each must be updated to handle the new type:

**Affected callsites and migration patterns:**

1. **path_builder.rs line 393** (in `process_child` function):
   ```rust
   // Before:
   let child_example = child_paths
       .first()
       .map_or(json!(null), |p| p.example.for_parent().clone());

   // After:
   let child_example = child_paths
       .first()
       .map_or(Example::NotApplicable, |p| p.example.for_parent().clone());
   ```
   *Pattern: Change default from `json!(null)` to `Example::NotApplicable`*

2. **enum_path_builder.rs line 276** (variant group assembly):
   ```rust
   // Before:
   .map(|p| p.example.for_parent().clone())

   // After:
   .map(|p| p.example.for_parent().clone())  // No change - already returns Example
   ```
   *Pattern: No change needed at callsite - Example works directly*

3. **support.rs line 71-104** (`extract_child_value_for_chain` function):
   ```rust
   // Before:
   let fallback = || child.example.for_parent().clone();

   // After:
   let fallback = || child.example.for_parent().to_value();
   ```
   *Pattern: Add `.to_value()` to convert Example to Value for return type*

**Note**: Most callers that store the result in `HashMap<..., Example>` require no changes. Only callers that expect `Value` need explicit `.to_value()` conversion.

---

### Phase 3: Update path_builder.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

**Changes**:

1. **Update ChildProcessingResult (line 57-64)**:
```rust
struct ChildProcessingResult {
    all_paths: Vec<MutationPathInternal>,
    paths_to_expose: Vec<MutationPathInternal>,
    child_examples: HashMap<MutationPathDescriptor, Example>,  // Changed from Value
}
```

2. **Update assembly call (line 106-108)**:
```rust
// Convert Examples to Values for builder assembly
let child_values: HashMap<_, _> = child_examples
    .iter()
    .map(|(k, ex)| (k.clone(), ex.to_value()))
    .collect();

let assembled_value = self
    .inner
    .assemble_from_children(ctx, child_values)?;

// Wrap result in Example
let assembled_example = Example::Json(assembled_value);
```

3. **Update partial root examples assembly (line 117)**:
```rust
// Convert to values for assembly
let child_values: HashMap<_, _> = direct_children
    .iter()
    .filter_map(|p| {
        let descriptor = p.path_kind.to_mutation_path_descriptor();
        child_examples.get(&descriptor)
            .map(|ex| (descriptor, ex.to_value()))
    })
    .collect();

let partial_example = self.inner
    .assemble_from_children(ctx, child_values)?;
// Use directly in partial root examples (converted later)
```

4. **Update knowledge example handling (line 119-121)**:
```rust
// Use knowledge example if available (already wrapped in Example)
let final_example = knowledge_example
    .map_or(assembled_example, |ex| ex);
```

5. **Update example building for mutation status (line 129-151)**:
```rust
let example_to_use: Example = match parent_status {
    Mutability::NotMutable => Example::NotApplicable,  // Self-documenting!
    Mutability::PartiallyMutable => {
        // Build partial example with only mutable children
        let mutable_child_values: HashMap<_, _> = child_examples
            .iter()
            .filter(|(descriptor, _)| {
                all_paths.iter().any(|p| {
                    p.path_kind.to_mutation_path_descriptor() == **descriptor
                        && matches!(p.mutability, Mutability::Mutable)
                })
            })
            .map(|(k, ex)| (k.clone(), ex.to_value()))
            .collect();

        let assembled = self.inner
            .assemble_from_children(ctx, mutable_child_values)
            .unwrap_or_else(|_| json!(null));

        Example::Json(assembled)
    }
    Mutability::Mutable => final_example,
};
```

6. **Update process_child return type (line 352-396)**:
```rust
fn process_child(
    descriptor: &MutationPathDescriptor,
    child_ctx: &RecursionContext,
) -> Result<(Vec<MutationPathInternal>, Example)> {  // Changed return type
    // ... existing logic ...

    // Extract child's example
    let child_example = child_paths
        .first()
        .map_or(Example::NotApplicable, |p| p.example.for_parent().clone());

    Ok((child_paths, child_example))
}
```

**HashMap Insertion/Extraction Strategy**: With `child_examples` now typed as `HashMap<MutationPathDescriptor, Example>`, the insertion and extraction logic becomes straightforward:

**Insertion (line 334)**:
```rust
// Before:
let (child_paths, child_example) = Self::process_child(&child_key, &child_ctx)?;
child_examples.insert(child_key, child_example);  // child_example was Value

// After:
let (child_paths, child_example) = Self::process_child(&child_key, &child_ctx)?;
child_examples.insert(child_key, child_example);  // child_example is now Example
// No changes needed at insertion site - process_child now returns Example
```

**Extraction for assembly (lines 217-221, already shown above in point 2)**:
```rust
// Convert HashMap<..., Example> to HashMap<..., Value> when calling builders:
let child_values: HashMap<_, _> = child_examples
    .iter()
    .map(|(k, ex)| (k.clone(), ex.to_value()))
    .collect();
```

**Key filtering (lines 113-115)**: No changes needed - `contains_key()` only uses keys, not values:
```rust
// This code works unchanged:
if child_examples.contains_key(&descriptor) { ... }
```

7. **Update build_not_mutable_path (line 556-567)**:
```rust
fn build_not_mutable_path(
    ctx: &RecursionContext,
    reason: NotMutableReason,
) -> MutationPathInternal {
    Self::build_mutation_path_internal(
        ctx,
        PathExample::Simple(Example::NotApplicable),  // Self-documenting!
        Mutability::NotMutable,
        Some(reason),
        None,
    )
}
```

8. **Update check_knowledge (line 582-625)**:
```rust
fn check_knowledge(
    ctx: &RecursionContext,
) -> (
    Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>>,
    Option<Example>,  // Changed from Option<Value>
) {
    let knowledge_result = ctx.find_knowledge();
    match knowledge_result {
        Ok(Some(knowledge)) => {
            let value = knowledge.example().clone();  // This is Value from TypeKnowledge
            let example = Example::Json(value);  // Wrap in Example

            if matches!(knowledge, TypeKnowledge::TreatAsRootValue { .. }) {
                return (
                    Some(Ok(vec![Self::build_mutation_path_internal(
                        ctx,
                        PathExample::Simple(example),
                        Mutability::Mutable,
                        None,
                        None,
                    )])),
                    None,
                );
            }

            (None, Some(example))
        }
        Ok(None) => (None, None),
        Err(e) => (Some(Err(e)), None),
    }
}
```

---

### Phase 4: Update mutation_path_internal.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

**Changes to resolve_path_example (line 169-187)**:
```rust
fn resolve_path_example(&mut self, has_default_for_root: bool) -> PathExample {
    match self.mutability {
        Mutability::NotMutable => {
            PathExample::Simple(Example::NotApplicable)  // Self-documenting!
        }
        Mutability::PartiallyMutable => match &self.example {
            PathExample::EnumRoot { .. } => self.example.clone(),
            PathExample::Simple(_) => {
                if has_default_for_root {
                    PathExample::Simple(Example::Json(json!({})))
                } else {
                    PathExample::Simple(Example::NotApplicable)
                }
            }
        },
        Mutability::Mutable => {
            // Clone the example for Mutable case
            self.example.clone()
        }
    }
}
```

---

### Phase 5: Update api.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`

**Changes to extract_spawn_format (line 67-76)**:
```rust
pub fn extract_spawn_format(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value> {
    mutation_paths
        .get("")
        .and_then(|root_path| match &root_path.path_example {
            PathExample::Simple(example) => {
                match example {
                    Example::Json(val) => Some(val.clone()),
                    Example::OptionNone => Some(Value::Null),
                    Example::NotApplicable => None,  // No spawn format
                }
            }
            PathExample::EnumRoot { groups, .. } => select_preferred_example(groups),
        })
}
```

**Reasoning**: Make the distinction explicit: `Json` types have spawn formats, `OptionNone` serializes to null, `NotApplicable` means no spawn format exists.

---

### Phase 6: Update enum_path_builder.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Key Changes**:

1. **Process children and create example groups**: When building `ExampleGroup` instances (line ~446), convert `Example -> Value`:
```rust
examples.push(ExampleGroup {
    applicable_variants,
    signature: variant_signature.clone(),
    example: Some(assembled_example.to_value()),  // Convert Example -> Value
    mutability: group_status,
});
```

2. **Build EnumRoot PathExample**: When creating the root path (line ~793-843):
```rust
let for_parent_example = select_preferred_example(&enum_examples)
    .map_or(Example::NotApplicable, Example::Json);  // Wrap in Example

PathExample::EnumRoot {
    groups: enum_examples,
    for_parent: for_parent_example,
}
```

3. **Handle child examples**: Anywhere the code collects child examples, ensure they're `Example` type and convert to `Value` when creating `ExampleGroup`.

**Note**: The enum builder is complex with many assembly points. Each location that creates an example Value should wrap it in `Example::Json()`. Each location that passes examples to builders should convert via `.to_value()`.

---

### Phase 7: Update support.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs`

**Changes**: Any functions that work with examples should accept/return `Example` instead of `Value`. Key functions:

1. **collect_children_for_chain**: Return type changes to `HashMap<..., Example>` or convert at call sites
2. **populate_root_examples_from_partials**: May need updates if it manipulates examples
3. Any utility functions that work with example values

---

### Phase 8: TypeKindBuilder Trait (Boundary Definition)

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/type_kind_builders/type_kind_builder.rs`

**Keep unchanged**:
```rust
pub trait TypeKindBuilder {
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        child_examples: HashMap<MutationPathDescriptor, Value>,  // Keep as Value
    ) -> Result<Value>;  // Keep as Value
}
```

**Reasoning**: Type kind builders are JSON assemblers. They fundamentally work with `serde_json::Value` for JSON construction. The conversion boundary is at the *call sites* in path_builder.rs and enum_path_builder.rs.

---

## Testing Strategy

### 1. Compilation Checks
After each phase, ensure the code compiles. Use:
```bash
cargo build
```

### 2. Type Safety Verification
The new design should catch errors at compile time:
- Can't accidentally pass `Value` where `Example` is expected
- Can't accidentally return wrong example type
- Pattern matching forces handling all `Example` variants

### 3. Integration Tests
Run existing integration tests:
```bash
cargo nextest run
```

All tests should pass without behavioral changes - we're only improving self-documentation, not changing logic.

### 4. Manual Inspection
Review call sites to verify:
- `Example::NotApplicable` appears for NotMutable paths
- `Example::OptionNone` appears for Option::None cases
- `Example::Json(...)` wraps all other values
- Conversions happen at clear boundaries

---

## Benefits

### 1. **Self-Documenting Code**
```rust
// Before
PathExample::Simple(json!(null))  // What does this null mean?

// After
PathExample::Simple(Example::NotApplicable)  // Clearly "no example because not mutable"
PathExample::Simple(Example::OptionNone)     // Clearly "this is Option::None"
```

### 2. **Type Safety**
The compiler enforces that all variants are handled, preventing bugs where null values are misinterpreted.

### 3. **Clear Boundaries**
Every `.to_value()` call marks a boundary where semantic meaning collapses to JSON. This makes the architecture easier to understand.

### 4. **Minimal Runtime Cost**
`Example` is a zero-cost wrapper in most cases. Explicit `as_value()` and `to_value()` methods provide efficient conversion at boundaries.

---

## Potential Challenges

### 1. **ExampleGroup Considerations**
We keep `ExampleGroup.example: Option<Value>` for simplicity, but we could change to `Option<Example>` if semantic distinction at that level becomes valuable.

### 2. **Builder Assembly Boundary**
Must be careful to convert `Example -> Value` when calling builders and `Value -> Example` when storing results. Missing conversions will cause compile errors (which is good!).

### 3. **Enum Builder Complexity**
enum_path_builder.rs is complex with many assembly points. Each needs careful review to ensure proper Example usage.

---

## Migration Order

1. ✅ Define `Example` enum with conversion methods
2. ✅ Update `PathExample` to use `Example`
3. ✅ Update path_builder.rs storage and assembly
4. ✅ Update mutation_path_internal.rs
5. ✅ Update api.rs extraction logic
6. ✅ Update enum_path_builder.rs (most complex)
7. ✅ Update support.rs utilities
8. ✅ Run tests and fix any remaining issues

---

## Open Questions

### Should Option::None be represented?
Currently, `Option<T>` types use regular values for `Some` variants. The `None` variant is typically represented as a unit enum variant. Do we need `Example::OptionNone` at all, or can we rely on enum variant representation?

**Answer**: Keep `Example::OptionNone` for cases where we need to explicitly represent a None value in JSON (e.g., when Option implements Default and None serializes to null).

### Should ExampleGroup use Example?
Currently planned to keep `Option<Value>` for simplicity. Revisit if we need semantic distinction at serialization level.

**Decision**: Keep `Option<Value>` unless strong need emerges. Conversion at boundary is clean and simple.

---

## Success Criteria

- [ ] All code compiles without warnings
- [ ] All existing tests pass
- [ ] No behavioral changes (this is purely a refactor)
- [ ] Code is more readable with self-documenting `Example` variants
- [ ] Clear conversion boundaries between `Example` and `Value`
- [ ] grep for `json!(null)` in mutation path builder code shows zero or near-zero results
