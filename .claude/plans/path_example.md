# PathExample: Making Illegal States Unrepresentable

## Problem Statement

The current `MutationPathInternal` structure uses multiple optional fields to represent examples, creating states that should be impossible:

```rust
pub struct MutationPathInternal {
    pub example: Value,                          // Always null for enums
    pub enum_example_groups: Option<Vec<ExampleGroup>>,     // Only Some for enum roots
    pub enum_example_for_parent: Option<Value>,  // Only Some for enum roots
}
```

**Invalid States This Allows:**
1. `example: json!(42), enum_example_groups: Some(...)` - contradictory
2. `example: json!(null), enum_example_groups: None` - ambiguous (enum or not-mutable?)
3. Forgetting to check `enum_example_for_parent` when extracting values

**Recent Bug:** In `builder.rs:530`, code used `child.example.clone()` for enum children, getting `null` instead of the actual variant value from `enum_example_for_parent`. This bug class is **architectural** - the data structure makes it easy to forget the enum special case.

## Proposed Solution

Replace the three-field approach with a single enum that makes the distinction explicit and compiler-enforced:

```rust
/// Example value for a mutation path
///
/// This enum ensures we cannot accidentally use the wrong example format for a path.
/// Enum roots MUST use `EnumRoot` variant, non-enum paths MUST use `Simple` variant.
#[derive(Debug, Clone)]
pub enum PathExample {
    /// Simple value example used by non-enum types
    ///
    /// Examples:
    /// - Structs: `{"field1": value1, "field2": value2}`
    /// - Primitives: `42`, `"text"`, `true`
    /// - Arrays: `[1, 2, 3]`
    /// - Option::None: `null` (special case for Option enum)
    Simple(Value),

    /// Enum root with variant groups and parent assembly value
    ///
    /// Only used for enum root paths (where `enum_example_groups` would be `Some`).
    /// The `for_parent` field provides the simplified example that parent types
    /// use when assembling their own examples.
    EnumRoot {
        /// All variant groups for this enum (the `examples` array in JSON output)
        groups: Vec<ExampleGroup>,
        /// Simplified example for parent assembly (replaces `enum_example_for_parent`)
        for_parent: Value,
    },
}

impl PathExample {
    /// Get the value to use for parent assembly
    ///
    /// For `Simple`, returns the value directly.
    /// For `EnumRoot`, returns the `for_parent` field.
    pub fn for_parent(&self) -> &Value {
        match self {
            Self::Simple(val) => val,
            Self::EnumRoot { for_parent, .. } => for_parent,
        }
    }

    /// Check if this is an enum root
    pub const fn is_enum_root(&self) -> bool {
        matches!(self, Self::EnumRoot { .. })
    }

    /// Get the simple value, or None if this is an enum root
    pub fn as_simple(&self) -> Option<&Value> {
        match self {
            Self::Simple(val) => Some(val),
            Self::EnumRoot { .. } => None,
        }
    }

    /// Get the enum groups, or None if this is a simple value
    pub fn as_enum_groups(&self) -> Option<&[ExampleGroup]> {
        match self {
            Self::Simple(_) => None,
            Self::EnumRoot { groups, .. } => Some(groups),
        }
    }
}
```

## Updated `MutationPathInternal`

```rust
pub struct MutationPathInternal {
    /// Example value - now type-safe!
    pub example: PathExample,

    // REMOVED: enum_example_groups (now in PathExample::EnumRoot)
    // REMOVED: enum_example_for_parent (now in PathExample::EnumRoot)

    /// Path for mutation
    pub full_mutation_path: FullMutationPath,
    /// Type information
    pub type_name: BrpTypeName,
    /// Context describing mutation kind
    pub path_kind: PathKind,
    /// Mutation status
    pub mutation_status: MutationStatus,
    /// Reason if not mutable
    pub mutation_status_reason: Option<Value>,
    /// Enum-specific path data
    pub enum_path_data: Option<EnumPathData>,
    /// Recursion depth
    pub depth: usize,
    /// Partial root examples for variant chains
    pub partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
}
```

## Migration Impact

### Files Requiring Changes

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`**
   - Add `PathExample` enum
   - Update `MutationPathInternal` structure
   - Update `MutationPath::from_mutation_path_internal()` to pattern match on `PathExample`

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`**
   - Update enum root path creation to use `PathExample::EnumRoot`
   - Update `build_variant_example()` to return appropriate format
   - Lines ~1010-1020: Change from setting three fields to creating `PathExample::EnumRoot`

3. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`**
   - Update `process_child()` (lines 432-437): Use `example.for_parent()` method
   - Update `assemble_partial_root_examples()` (lines 532-536): Use `example.for_parent()` method
   - Update `build_mutation_path_internal()` to accept `PathExample` instead of `Value`
   - Update `build_not_mutable_path()` to use `PathExample::Simple(json!(null))`

4. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`**
   - Update any code that constructs `MutationPathInternal` to use `PathExample`

### Code Changes - Before/After

#### Before (Error-Prone)
```rust
// In builder.rs - easy to forget enum_example_for_parent
let fallback_example = child.example.clone();  // BUG: null for enums!

// In enum_path_builder.rs - three separate assignments
MutationPathInternal {
    example: json!(null),
    enum_example_groups: Some(enum_examples),
    enum_example_for_parent: Some(simple_example),
    // ...
}
```

#### After (Compiler-Enforced)
```rust
// In builder.rs - compiler forces us to handle both cases
let fallback_example = child.example.for_parent().clone();  // Works for all types!

// Or with explicit pattern matching for clarity:
let fallback_example = match &child.example {
    PathExample::Simple(val) => val.clone(),
    PathExample::EnumRoot { for_parent, .. } => for_parent.clone(),
};

// In enum_path_builder.rs - single atomic construction
MutationPathInternal {
    example: PathExample::EnumRoot {
        groups: enum_examples,
        for_parent: simple_example,
    },
    // ...
}
```

## Benefits

### 1. Compiler-Enforced Correctness
Pattern matching forces handling of both cases. The bug we just fixed (using `example` instead of `enum_example_for_parent`) becomes **impossible** because the compiler requires explicit handling.

### 2. Self-Documenting Code
```rust
// OLD: What does this mean?
if path.enum_example_groups.is_some() { ... }

// NEW: Crystal clear
if path.example.is_enum_root() { ... }
// or
match &path.example {
    PathExample::EnumRoot { groups, .. } => { /* handle enum */ },
    PathExample::Simple(val) => { /* handle simple */ },
}
```

### 3. Reduced Cognitive Load
Developers no longer need to remember:
- "enum paths have `example: null`"
- "use `enum_example_for_parent` for parent assembly"
- "check `enum_example_groups.is_some()` to detect enum roots"

The type system encodes this knowledge.

### 4. Fewer Edge Cases
Invalid state combinations are impossible by construction:
- Can't have `example: 42` with `enum_example_groups: Some(...)`
- Can't forget to set `enum_example_for_parent` when creating enum roots
- Can't misinterpret `example: null` (is it enum? not-mutable? Option::None?)

## Implementation Strategy

### Phase 1: Add New Type (Non-Breaking)
1. Add `PathExample` enum to `types.rs`
2. Add it alongside existing fields temporarily
3. Run tests to ensure it compiles

### Phase 2: Update Constructors
1. Change `enum_path_builder.rs` to construct `PathExample::EnumRoot`
2. Change `builder.rs` to construct `PathExample::Simple`
3. Keep old fields populated for compatibility

### Phase 3: Update Consumers
1. Update all code that reads `example`/`enum_example_groups`/`enum_example_for_parent`
2. Replace with `PathExample` pattern matching or helper methods
3. Verify all call sites handle both variants

### Phase 4: Remove Old Fields
1. Delete `enum_example_groups` and `enum_example_for_parent` from `MutationPathInternal`
2. Run full test suite
3. Fix any compilation errors (should be none if Phase 3 was thorough)

### Phase 5: Simplify
1. Look for repeated patterns that can use helper methods
2. Add convenience methods to `PathExample` as needed
3. Update documentation

## Testing Strategy

### Unit Tests
```rust
#[test]
fn path_example_simple_for_parent() {
    let example = PathExample::Simple(json!({"x": 10}));
    assert_eq!(example.for_parent(), &json!({"x": 10}));
}

#[test]
fn path_example_enum_for_parent() {
    let example = PathExample::EnumRoot {
        groups: vec![/* ... */],
        for_parent: json!({"Variant": "value"}),
    };
    assert_eq!(example.for_parent(), &json!({"Variant": "value"}));
}

#[test]
fn path_example_type_checks() {
    let simple = PathExample::Simple(json!(42));
    assert!(!simple.is_enum_root());
    assert!(simple.as_simple().is_some());
    assert!(simple.as_enum_groups().is_none());

    let enum_root = PathExample::EnumRoot {
        groups: vec![],
        for_parent: json!(null),
    };
    assert!(enum_root.is_enum_root());
    assert!(enum_root.as_simple().is_none());
    assert!(enum_root.as_enum_groups().is_some());
}
```

### Integration Tests
1. Run existing mutation test suite with new structure
2. Verify `TestComplexComponent` works correctly (the bug we just fixed)
3. Verify all enum types produce correct `examples` arrays in JSON output
4. Verify non-enum types produce correct `example` values

### Regression Prevention
Add test specifically for the bug we just fixed:
```rust
#[test]
fn struct_with_enum_field_assembles_correctly() {
    // TestComplexComponent has SimpleNestedEnum field
    // Verify that the root_example for nested paths contains
    // the actual variant value, not null
    let type_guide = generate_type_guide("extras_plugin::TestComplexComponent");

    let mode_field_paths = type_guide.mutation_paths
        .iter()
        .filter(|(path, _)| path.starts_with(".mode."))
        .collect::<Vec<_>>();

    for (path, data) in mode_field_paths {
        if let Some(root_example) = &data.path_info.root_example {
            let mode_value = root_example.get("mode");
            // Should be "None" string or {"Nested": ...}, NOT null
            assert_ne!(mode_value, Some(&json!(null)),
                "Bug regression: mode field is null in root_example for path {}", path);
        }
    }
}
```

## Future Improvements

Once `PathExample` is in place, consider:

1. **Enum for mutation status + example**
   ```rust
   pub enum MutationResult {
       Mutable { example: PathExample },
       PartiallyMutable { example: PathExample, reason: Value },
       NotMutable { reason: Value },
   }
   ```

2. **Separate types for root vs child paths**
   - Root paths need `partial_root_examples`
   - Child paths don't
   - Could use type parameters or separate structs

3. **Builder pattern for `MutationPathInternal`**
   - Enforce invariants at construction time
   - Make it impossible to create invalid paths

## Conclusion

This refactoring eliminates an entire class of bugs by making invalid states unrepresentable. The initial migration effort is modest, but the long-term benefits are substantial:

- **Fewer bugs**: Compiler prevents the mistake we just fixed and many others
- **Better documentation**: Code clearly expresses intent through types
- **Easier maintenance**: New developers can't accidentally misuse the API
- **Reduced testing burden**: Invalid states don't need test coverage

This is a textbook example of Rust's type system working for us instead of against us.
