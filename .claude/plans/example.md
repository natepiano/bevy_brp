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

### Phase 1: Add Example Enum to types.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Add after imports (line ~17)**:

```rust
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

**Reasoning**: The `Example` enum is a core type alongside `Mutability`, `PathAction`, etc., so it belongs in `types.rs` rather than a separate module. This keeps the codebase simpler.

---

### Phase 2: Update PathExample

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_example.rs`

**Changes**:

**Step 1: Add imports at the top of the file**

Add these imports to the file's import section at the top (after any `mod` declarations, grouped with existing `use` statements):

```rust
use super::types::Example;
use super::types::ExampleGroup;
```

**Step 2: Update PathExample enum definition**

```rust
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
```

**Step 3: Update for_parent() method signature**

```rust
impl PathExample {
    /// Get the example to use for parent assembly
    pub const fn for_parent(&self) -> &Example {  // Changed return type from &Value, keep const
        match self {
            Self::Simple(ex) => ex,
            Self::EnumRoot { for_parent, .. } => for_parent,
        }
    }
}
```

**Note**: The `const` qualifier is preserved because the implementation remains const-compatible (simple pattern matching returning references).

**Step 4: Update Serialize implementation**

```rust
impl Serialize for PathExample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Simple(example) => {
                let value = example.to_value();  // Convert to Value
                if value.is_null() {
                    serializer.serialize_map(Some(0))?.end()
                } else {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("example", &value)?;
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

This is the most complex phase, involving enum variant example assembly. The module builds nested examples through several layers of functions with `HashMap<MutationPathDescriptor, Example>` at the core.

#### 6.1: Function Signature Changes

There are three key functions in `enum_path_builder.rs` that need signature updates:

**1. Change build_variant_example signature (line 361)**:
```rust
// Before:
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &VariantName,
    children: &HashMap<MutationPathDescriptor, Value>,
    enum_type: &BrpTypeName,
) -> Value {

// After:
fn build_variant_example(
    signature: &VariantSignature,
    variant_name: &VariantName,
    children: &HashMap<MutationPathDescriptor, Example>,
    enum_type: &BrpTypeName,
) -> Example {
```

**2. Change build_variant_group_example signature (line ~319)**:
```rust
// Before:
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[VariantName],
    child_examples: &HashMap<MutationPathDescriptor, Value>,
    mutability: Mutability,
    ctx: &RecursionContext,
) -> std::result::Result<Option<Value>, BuilderError> {

// After:
fn build_variant_group_example(
    signature: &VariantSignature,
    variants_in_group: &[VariantName],
    child_examples: &HashMap<MutationPathDescriptor, Example>,
    mutability: Mutability,
    ctx: &RecursionContext,
) -> std::result::Result<Option<Example>, BuilderError> {
```

**3. Change build_variant_example_for_chain signature (line ~698)**:
```rust
// Before:
fn build_variant_example_for_chain(
    signature: &VariantSignature,
    variant_name: &VariantName,
    child_mutation_paths: &[MutationPathInternal],
    variant_chain: &[VariantName],
    ctx: &RecursionContext,
) -> Value {

// After:
fn build_variant_example_for_chain(
    signature: &VariantSignature,
    variant_name: &VariantName,
    child_mutation_paths: &[MutationPathInternal],
    variant_chain: &[VariantName],
    ctx: &RecursionContext,
) -> Example {
```

#### 6.1a: Update process_signature_path HashMap Parameter

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Change signature (line ~231)**:
```rust
// Before:
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    signature: &VariantSignature,
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {

// After:
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    signature: &VariantSignature,
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Example>,  // Changed from Value
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
```

**Update implementation (line ~276-278)**:
```rust
// No change needed at line ~276 - for_parent() now returns &Example after Phase 2:
let child_example = child_paths
    .first()
    .ok_or_else(|| { ... })
    .map(|p| p.example.for_parent().clone())?;  // Returns Example

// No change needed at line ~278 - type now matches HashMap<..., Example>:
child_examples.insert(child_descriptor, child_example);
```

**Reasoning**: This function bridges the recursion results (which produce `PathExample` containing `Example` after Phase 2) with the HashMap used for variant example assembly. The HashMap parameter type must match the return type of `for_parent()`.

---

#### 6.2: Update build_variant_example Implementation

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Change at lines 367-396** - All match arms need to:
1. Work with `Example` inputs from children HashMap
2. Call `.to_value()` when building JSON structures
3. Wrap final result in `Example::Json(...)` before returning
4. Use `Example::NotApplicable` for missing children (line 375 per Phase 6a.2)

**Unit variant (line 368-370)**:
```rust
// Before:
VariantSignature::Unit => {
    json!(variant_name.short_name())
}

// After:
VariantSignature::Unit => {
    Example::Json(json!(variant_name.short_name()))
}
```

**Tuple variant (lines 371-386)**:
```rust
// Before:
VariantSignature::Tuple(types) => {
    let mut tuple_values = Vec::new();
    for index in 0..types.len() {
        let descriptor = MutationPathDescriptor::from(index.to_string());
        let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
        tuple_values.push(value);
    }
    if tuple_values.len() == 1 {
        json!({ variant_name.short_name(): tuple_values[0] })
    } else {
        json!({ variant_name.short_name(): tuple_values })
    }
}

// After:
VariantSignature::Tuple(types) => {
    let mut tuple_values = Vec::new();
    for index in 0..types.len() {
        let descriptor = MutationPathDescriptor::from(index.to_string());
        let example = children.get(&descriptor).cloned().unwrap_or(Example::NotApplicable);
        tuple_values.push(example.to_value());  // Convert to Value for JSON construction
    }
    if tuple_values.len() == 1 {
        Example::Json(json!({ variant_name.short_name(): tuple_values[0] }))
    } else {
        Example::Json(json!({ variant_name.short_name(): tuple_values }))
    }
}
```

**Struct variant (lines 387-391)**:
```rust
// Before:
VariantSignature::Struct(_field_types) => {
    let field_values = support::assemble_struct_from_children(children);
    json!({ variant_name.short_name(): field_values })
}

// After:
VariantSignature::Struct(_field_types) => {
    let field_values = support::assemble_struct_from_children(children);
    Example::Json(json!({ variant_name.short_name(): field_values }))
}
```

**Note**: `assemble_struct_from_children` signature also changes (covered in Phase 7).

**Option transformation callsite (line 395)**:
```rust
// Before:
apply_option_transformation(example, variant_name, enum_type)

// After:
apply_option_transformation(example, variant_name, enum_type)  // Now accepts and returns Example
```

**Note**: This requires updating `apply_option_transformation` signature (covered in Phase 6a.1).

#### 6.3: Update build_variant_group_example Implementation

**Change implementation (lines ~327-337)**:
```rust
// Before:
let example = if matches!(
    mutability,
    Mutability::NotMutable | Mutability::PartiallyMutable
) {
    None
} else {
    Some(build_variant_example(
        signature,
        representative_variant_name,
        child_examples,
        ctx.type_name(),
    ))
};

// After:
let example = if matches!(
    mutability,
    Mutability::NotMutable | Mutability::PartiallyMutable
) {
    None
} else {
    Some(build_variant_example(
        signature,
        representative_variant_name,
        child_examples,
        ctx.type_name(),
    ))  // Now returns Example
};
```

**Reasoning**: The function already returns `Option<Example>` after signature change. The implementation mostly stays the same since it wraps the result in `Option`.

#### 6.4: Update process_signature_groups HashMap Creation and Usage

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**No signature changes needed** - this function's signature doesn't involve Example/Value types.

**Update HashMap initialization (line ~412)**:
```rust
// Before (type inferred from process_signature_path parameter):
let mut child_examples = HashMap::new();

// After (no code change, but type inference changes due to process_signature_path update):
let mut child_examples = HashMap::new();  // Now infers HashMap<MutationPathDescriptor, Example>
```

**Update HashMap passing (line ~428)** - No changes needed:
```rust
// This call already correct after process_signature_path signature change:
let child_paths = process_signature_path(
    path_kind,
    &applicable_variants,
    variant_signature,
    ctx,
    &mut child_examples,  // Type now HashMap<MutationPathDescriptor, Example>
)?;
```

**Update build_variant_group_example call (line ~441)** - No changes needed:
```rust
// This call already correct after build_variant_group_example signature change:
let example = build_variant_group_example(
    variant_signature,
    variant_names,
    &child_examples,  // Type now HashMap<MutationPathDescriptor, Example>
    mutability,
    ctx,
)?;
```

**Reasoning**: This orchestration function creates the HashMap that flows through the variant example building process. After Phase 6.1a updates `process_signature_path` parameter type, Rust's type inference automatically updates the HashMap type at line ~412. No explicit code changes needed, but the type flow must be understood.

#### 6.5: Update PathExample::EnumRoot Construction

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Location**: build_enum_root_path function, line ~812-815

**No changes needed** - already addressed by Phase 6.8:

After Phase 2 updates `PathExample::EnumRoot.for_parent` to `Example` type, and Phase 6.8 updates `default_example` parameter to `Example` type, the construction at line ~812-815 requires no changes:

```rust
PathExample::EnumRoot {
    groups: enum_examples,
    for_parent: default_example,  // Already Example type after Phase 6.8
}
```

**Reasoning**: This is not a separate update - it's the natural consequence of earlier phases. Including it here for completeness to show the data flow:
1. Phase 2: PathExample::EnumRoot.for_parent changes to Example
2. Phase 6.7: select_preferred_example returns Option<Example>
3. Phase 6.8: default_example parameter changes to Example
4. Result: Line ~814 assignment already type-correct, no code changes needed

#### 6.6: Summary of Example Flow in enum_path_builder.rs

**Data Flow**:
1. **Input**: Children have `PathExample::Simple(Example)` from recursion
2. **Extraction**: Extract `Example` from children and build `HashMap<MutationPathDescriptor, Example>`
3. **Assembly**: Pass HashMap to `build_variant_example` which returns `Example`
4. **Boundary**: Convert `Example -> Value` via `.to_value()` when creating `ExampleGroup`
5. **Storage**: `ExampleGroup.example` stores `Option<Value>` (external API format)
6. **Root assembly**: `select_preferred_example` returns `Option<Value>`, wrap in `Example::Json` for `PathExample::EnumRoot.for_parent`

**Type Conversions**:
- `Example -> Value`: Use `.to_value()` at ExampleGroup creation boundary
- `Value -> Example`: Use `Example::Json(value)` when receiving from select_preferred_example

#### 6.7: Update select_preferred_example to Return Example

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Change signature (line ~290)**:
```rust
// Before:
pub fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Value> {

// After:
pub fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Example> {
```

**Update implementation (line ~308)**:
```rust
// Before:
.and_then(|eg| eg.example.clone())

// After:
.and_then(|eg| eg.example.clone().map(Example::Json))  // Wrap Value in Example::Json
```

**Reasoning**: This function is primarily used internally (2 of 3 callsites). By returning `Option<Example>`, internal processing stays in the `Example` domain with no conversions needed. Only the single external API callsite needs conversion.

**Update callsites**:

1. **Internal: process_enum default_example (line ~790-810)** - Use directly, no change needed:
```rust
let default_example = ctx
    .find_knowledge()
    .ok()
    .flatten()
    .map(|knowledge| Example::Json(knowledge.example().clone()))
    .or_else(|| select_preferred_example(&enum_examples))  // Now returns Option<Example>
    .ok_or_else(|| { ... })?;
```

2. **Internal: process_signature_groups spawn_example (line ~660-680)** - Use directly, update fallback:
```rust
// Before:
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone())
    .or_else(|| select_preferred_example(enum_examples))
    .unwrap_or(json!(null));

// After:
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone().map(Example::Json))
    .or_else(|| select_preferred_example(enum_examples))  // Now returns Option<Example>
    .unwrap_or(Example::NotApplicable);
```

3. **External: api.rs extract_spawn_format (line ~457)** - Convert to Value at API boundary:
```rust
// Before:
PathExample::EnumRoot { groups, .. } => select_preferred_example(groups),

// After:
PathExample::EnumRoot { groups, .. } => {
    select_preferred_example(groups).map(|ex| ex.to_value())
}
```

**Note**: Use `.map()` not `.and_then()` because `to_value()` returns `Value`, not `Option<Value>`.

**Related signature cascades**: See Phase 6.8 below for documentation of `create_enum_mutation_paths` and `build_enum_root_path` parameter updates.

---

#### 6.7a: Update build_partial_root_examples spawn_example Construction

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Critical type consistency fix (lines ~540-545)**:

After Phase 6.7 changes `select_preferred_example` return type to `Option<Example>`, the spawn_example construction has a type mismatch that will cause a **compile error**:

```rust
// Before:
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone())  // Returns Option<Value>
    .or_else(|| select_preferred_example(enum_examples))  // Returns Option<Value> before Phase 6.7
    .unwrap_or(json!(null));

// After:
let spawn_example = enum_examples
    .iter()
    .find(|ex| ex.applicable_variants.contains(variant_name))
    .and_then(|ex| ex.example.clone().map(Example::Json))  // Wrap Value in Example::Json
    .or_else(|| select_preferred_example(enum_examples))  // Now returns Option<Example>
    .unwrap_or(Example::NotApplicable);  // Use Example::NotApplicable
```

**Reasoning**:
1. Line 543's `ex.example.clone()` returns `Option<Value>` (ExampleGroup.example is Value type)
2. After Phase 6.7, line 544's `select_preferred_example` returns `Option<Example>`
3. These types can't be chained with `.or_else()` - **this causes a compile error**
4. Solution: Wrap line 543's Value result in `Example::Json` for type consistency
5. Line 545: Use `Example::NotApplicable` instead of `json!(null)` for self-documentation
6. The spawn_example variable is used at lines ~590, ~602, ~615 where it's passed to `wrap_example_with_availability` (expects Example after Phase 7.4) or assigned to `example: Example` variable

**Usage sites that benefit from this fix**:
- Line ~590: `wrap_example_with_availability(example, ...)` - example is from build_variant_example_for_chain (returns Example)
- Line ~602: `let example = spawn_example` - now properly typed as Example
- Line ~615: `wrap_example_with_availability(example, ...)` - example now properly typed as Example

---

#### 6.8: Update Signature Cascades for default_example Parameter

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Two functions receive the `default_example` value from `select_preferred_example` and need signature updates:

**1. Update create_enum_mutation_paths (line ~844)**:
```rust
// Before:
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,  // Line ~847
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, RootExample>,
) -> Vec<MutationPathInternal> {

// After:
fn create_enum_mutation_paths(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Example,  // Changed from Value
    mut child_mutation_paths: Vec<MutationPathInternal>,
    partial_root_examples: HashMap<Vec<VariantName>, RootExample>,
) -> Vec<MutationPathInternal> {
```

**2. Update build_enum_root_path (line ~791)**:
```rust
// Before:
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,  // Line ~794
    enum_mutability: Mutability,
    mutability_reason: Option<NotMutableReason>,
) -> MutationPathInternal {

// After:
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Example,  // Changed from Value
    enum_mutability: Mutability,
    mutability_reason: Option<NotMutableReason>,
) -> MutationPathInternal {
```

**Update usage in build_enum_root_path (line ~814)**:
```rust
// Before:
PathExample::EnumRoot {
    groups: enum_examples,
    for_parent: default_example,  // Value type
}

// After:
PathExample::EnumRoot {
    groups: enum_examples,
    for_parent: default_example,  // Example type
}
```

**Reasoning**: After Phase 6.7 changes `select_preferred_example` to return `Option<Example>`, the `default_example` value passed through these functions is Example type. The signatures must be updated to match. The usage at line ~814 already matches the PathExample::EnumRoot.for_parent field type change from Phase 2.

**Callsite verification**:
- Line ~123: `create_enum_mutation_paths` called with `default_example: Example` ✓
- Line ~864: `build_enum_root_path` called with `default_example: Example` ✓

---

### Phase 6a: Update Option Handling and Enum Fallbacks

This phase documents the specific locations where `Example::OptionNone` and `Example::NotApplicable` replace `json!(null)` with clear semantic meaning.

#### **6a.1: option_classification.rs - Example::OptionNone**

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/option_classification.rs`

**Change at line 61-63**:
```rust
// Before:
match variant_name.short_name() {
    "None" => {
        json!(null)  // Transforms Option::None to null for BRP
    }

// After:
match variant_name.short_name() {
    "None" => {
        Example::OptionNone  // Explicit: this IS Option::None variant
    }
```

**Reasoning**: This is the ONLY location where we're explicitly handling the `None` variant of an `Option<T>` enum. Using `Example::OptionNone` makes the semantic intent crystal clear: "this value represents Option::None".

**Why distinguish OptionNone from NotApplicable if both serialize to null?**

The semantic distinction provides value during development and code maintenance:

1. **Self-documenting code**: When reading code or logs, `Example::OptionNone` clearly communicates "this is legitimately Option::None" vs `Example::NotApplicable` meaning "no example because path is not mutable"
2. **Type safety**: Pattern matching on `Example` forces explicit handling of all three cases, preventing bugs where different null semantics are conflated
3. **Debugging clarity**: When debugging intermediate values, you can see whether null came from Option::None or from a filtered-out NotMutable path
4. **Semantic correctness**: The code models the actual domain - Option::None IS different from "no data available", even if both serialize to null for BRP

At serialization time (Phase 1, line 83), both collapse to `Value::Null`, which is correct for the external API. But during internal processing, the distinction helps developers understand data flow and intent.

**Note**: This requires changing `apply_option_transformation` signature from `-> Value` to `-> Example`, and updating the one callsite at enum_path_builder.rs:395.

#### **6a.2: enum_path_builder.rs - Example::NotApplicable for Missing Children**

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Change at line 375**:
```rust
// Before:
VariantSignature::Tuple(types) => {
    let mut tuple_values = Vec::new();
    for index in 0..types.len() {
        let descriptor = MutationPathDescriptor::from(index.to_string());
        let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
        tuple_values.push(value);
    }
    // ...
}

// After:
VariantSignature::Tuple(types) => {
    let mut tuple_values = Vec::new();
    for index in 0..types.len() {
        let descriptor = MutationPathDescriptor::from(index.to_string());
        let value = children.get(&descriptor).cloned().unwrap_or(Example::NotApplicable);
        tuple_values.push(value);
    }
    // ...
}
```

**Reasoning**: When `children.get(&descriptor)` returns `None`, it means the child was filtered out by `collect_children_for_chain` (support.rs:130) because it's `NotMutable` (recursion limit, missing Reflect, etc.). This is semantically "no example available for this child", not "the value is null" or "this is Option::None".

#### **6a.3: Update Misleading Comment**

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Change at lines 349-360**:
```rust
// Before (lines 357-360):
// When children are empty (e.g., filtered `NotMutable` at recursion depth limits),
// `unwrap_or(json!(null))` provides a fallback, producing `{"Some": null}` which
// `apply_option_transformation` transforms to `null` - the correct BRP representation.

// After:
// When children are missing from the HashMap (filtered `NotMutable` at recursion depth limits),
// `unwrap_or(Example::NotApplicable)` provides a fallback representing "child unavailable".
// For Option types, this produces `{"Some": NotApplicable}` which after transformation
// and serialization becomes `null` - the correct BRP representation for unavailable children.
// This pattern works for any enum tuple variant, not just Option.
```

**Reasoning**: The original comment was misleading because it framed the fallback as Option-specific when it's actually a general mechanism for any enum tuple with filtered/unavailable children.

---

### Phase 7: Update support.rs

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs`

This module contains shared helper functions used by both `path_builder.rs` (non-enum types) and `enum_path_builder.rs` (enum types). All example-related functions need to work with `Example` internally.

#### 7.1: Update collect_children_for_chain Signature

**Change signature (line ~114)**:
```rust
// Before:
pub fn collect_children_for_chain(
    child_paths: &[&MutationPathInternal],
    ctx: &RecursionContext,
    target_chain: Option<&[VariantName]>,
) -> HashMap<MutationPathDescriptor, Value> {

// After:
pub fn collect_children_for_chain(
    child_paths: &[&MutationPathInternal],
    ctx: &RecursionContext,
    target_chain: Option<&[VariantName]>,
) -> HashMap<MutationPathDescriptor, Example> {
```

**Update map closure (line ~127)**:
```rust
// Before:
.map(|child| {
    let descriptor = child.path_kind.to_mutation_path_descriptor();
    let value = extract_child_value_for_chain(child, target_chain);
    (descriptor, value)
})

// After:
.map(|child| {
    let descriptor = child.path_kind.to_mutation_path_descriptor();
    let example = extract_child_value_for_chain(child, target_chain);  // Now returns Example
    (descriptor, example)
})
```

**Callsites to update**:
- `path_builder.rs` line ~330: Expects `HashMap<MutationPathDescriptor, Example>` ✓ (already documented in Phase 3)
- `enum_path_builder.rs` line ~705: `build_variant_example_for_chain` calls it, expects `Example` HashMap ✓ (already updated in Phase 6.1)

#### 7.2: Update extract_child_value_for_chain (Private Helper)

**Change signature (line ~71)**:
```rust
// Before:
fn extract_child_value_for_chain(
    child: &MutationPathInternal,
    child_chain: Option<&[VariantName]>,
) -> Value {

// After:
fn extract_child_value_for_chain(
    child: &MutationPathInternal,
    child_chain: Option<&[VariantName]>,
) -> Example {
```

**Update fallback closure (line ~75)**:
```rust
// Before:
let fallback = || child.example.for_parent().clone();  // Returns Value

// After:
let fallback = || child.example.for_parent().clone();  // Now returns Example after Phase 2
```

**Note**: After Phase 2, `for_parent()` returns `&Example` instead of `&Value`, so the fallback closure already returns the correct type. No code change needed at line 75.

**Update helper closure to wrap extracted values (lines ~83-88)**:
```rust
// Before:
let get_value = |root_ex: &RootExample| match root_ex {
    RootExample::Available { root_example } if !root_example.is_null() => {
        Some(root_example.clone())  // Returns Option<Value>
    }
    _ => None,
};

// After:
let get_value = |root_ex: &RootExample| match root_ex {
    RootExample::Available { root_example } if !root_example.is_null() => {
        Some(Example::Json(root_example.clone()))  // Wrap in Example::Json, returns Option<Example>
    }
    _ => None,
};
```

**Reasoning**:
- The `get_value` helper extracts `root_example: Value` from `RootExample::Available`
- This `Value` must be wrapped in `Example::Json` before returning
- The rest of the function logic (lines 77-103) remains unchanged - it already handles the `.and_then()` chains correctly
- The fallback at line 102 works unchanged because it calls the `fallback` closure which now returns `Example`

**Complete context**: This function has complex logic that:
1. Uses the fallback if no variant chain specified
2. Otherwise searches `child.partial_root_examples` for matching variant chains
3. Tries exact match first, then compatible prefix matches for nested enums
4. Falls back to `for_parent()` if nothing found
5. The only change needed is wrapping extracted `root_example` values in `Example::Json`

#### 7.3: Update assemble_struct_from_children Signature

**Change signature (line ~143)**:
```rust
// Before:
pub fn assemble_struct_from_children(
    children: &HashMap<MutationPathDescriptor, Value>,
) -> serde_json::Map<String, Value> {

// After:
pub fn assemble_struct_from_children(
    children: &HashMap<MutationPathDescriptor, Example>,
) -> serde_json::Map<String, Value> {
```

**Update loop body (line ~146)**:
```rust
// Before:
for (descriptor, example) in children {
    let field_name = (*descriptor).to_string();
    struct_obj.insert(field_name, example.clone());
}

// After:
for (descriptor, example) in children {
    let field_name = (*descriptor).to_string();
    struct_obj.insert(field_name, example.to_value());  // Convert Example -> Value
}
```

**Reasoning**: This function builds JSON objects, so it needs to convert `Example -> Value` at the boundary. The return type stays `Map<String, Value>` since it's pure JSON construction.

**Callsites to update**:
- `struct_builder.rs` line ~46: Passes `HashMap<MutationPathDescriptor, Example>` ✓ (documented in Phase 3)
- `enum_path_builder.rs` line 389: Used in `build_variant_example` Struct arm ✓ (already expects Example HashMap from Phase 6.2)

#### 7.4: Update wrap_example_with_availability Signature

**Change signature (line ~163)**:
```rust
// Before:
pub fn wrap_example_with_availability(
    example: Value,
    children: &[&MutationPathInternal],
    chain: &[VariantName],
    parent_unavailable_reason: Option<String>,
) -> RootExample {

// After:
pub fn wrap_example_with_availability(
    example: Example,
    children: &[&MutationPathInternal],
    chain: &[VariantName],
    parent_unavailable_reason: Option<String>,
) -> RootExample {
```

**Update RootExample construction**:
```rust
// Before:
RootExample::Available {
    root_example: example,  // Value
}

// After:
RootExample::Available {
    root_example: example.to_value(),  // Convert Example -> Value for external API
}
```

**Reasoning**: `RootExample` is part of external API structures (stored in `EnumPathInfo`), so it should contain `Value`. The boundary conversion happens here.

**Callsites to update**:
- `path_builder.rs` line ~372: Passes `Example` from builder ✓ (documented in Phase 3)
- `enum_path_builder.rs` line ~676 and ~689: Pass `Example` from `spawn_example` ✓ (documented in Phase 6.7)

#### 7.5: Functions That Don't Need Changes

**aggregate_mutability** - No changes needed, doesn't work with examples.

**populate_root_examples_from_partials** - No changes needed. It populates `HashMap<Vec<VariantName>, RootExample>` but doesn't manipulate the examples themselves, just transfers them from `MutationPathInternal.partial_root_examples`.

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
`Example` is a zero-cost wrapper in most cases. The `to_value()` method provides efficient conversion at boundaries, and creating `Value::Null` is a cheap enum variant construction.

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
