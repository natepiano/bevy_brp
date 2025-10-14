# PathExample: Making Illegal States Unrepresentable

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

### Step 1: Add PathExample Type Definition ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: SAFE (Additive change - compiles independently)
**Build Status**: ✅ Will compile successfully

**Objective**: Add the new `PathExample` enum type to make enum vs non-enum examples type-safe

**Changes**:
- Add `PathExample` enum with `Simple` and `EnumRoot` variants
- Add `for_parent()` helper method
- Place just before `MutationPathInternal` struct definition

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Implementation Details**: See "Code Changes - types.rs PathExample Definition" section below

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Validation**: Confirm the build succeeds with no errors

---

### Step 2: Core Type System Migration ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: ATOMIC GROUP - CRITICAL (Must be done together)
**Build Status**: ✅ Will compile successfully after all changes

**Objective**: Update the core `MutationPathInternal` struct and all code that constructs it

**Changes**:
1. **types.rs**: Update `MutationPathInternal` struct
   - Change `example` field from `Value` to `PathExample`
   - Remove `enum_example_groups` field
   - Remove `enum_example_for_parent` field

2. **builder.rs**: Update `build_mutation_path_internal()` signature
   - Change parameter `example: Value` to `example: PathExample`

3. **builder.rs**: Update all 4 call sites:
   - `build_final_result` Create mode (line ~572): Wrap with `PathExample::Simple()`
   - `build_final_result` Skip mode (line ~586): Wrap with `PathExample::Simple()`
   - `build_not_mutable_path` (line ~607): Wrap with `PathExample::Simple()`
   - `check_knowledge` TreatAsRootValue (line ~124): Wrap with `PathExample::Simple()`

4. **enum_path_builder.rs**: Update `build_enum_root_path()` struct initialization
   - Replace three-field pattern with `PathExample::EnumRoot { groups, for_parent }`
   - Remove `enum_example_groups` and `enum_example_for_parent` field assignments

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Implementation Details**: See sections below:
- "Code Changes - build_mutation_path_internal Call Sites"
- "Code Changes - enum_path_builder.rs Struct Initialization"

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Validation**: Confirm the build succeeds - all struct construction sites now use the new field structure

**⚠️ CRITICAL**: This step must be completed atomically. Do not commit partial changes - all construction sites must be updated together.

---

### Step 3: Update Field Access Patterns ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: ATOMIC GROUP - CRITICAL (Must be done together)
**Build Status**: ✅ Will compile successfully after all changes
**Dependencies**: Requires Step 2

**Objective**: Update all code that reads from the example field to use the new `for_parent()` method

**Changes**:
1. **builder.rs**: Update `process_child()` (lines 432-437)
   - Replace nested `map_or_else` with `p.example.for_parent().clone()`

2. **builder.rs**: Update `assemble_partial_root_examples()` (lines 529-536)
   - Replace `enum_example_for_parent` access with `child.example.for_parent().clone()`

3. **enum_path_builder.rs**: Update `process_signature_path()` (lines 570-580)
   - Replace `enum_example_for_parent` access with `p.example.for_parent().clone()`

4. **enum_path_builder.rs**: Update `extract_child_fallback_value()` (lines 811-818)
   - Replace `enum_example_for_parent` access with `child.example.for_parent().clone()`

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Implementation Details**: See sections below:
- "Code Changes - builder.rs Call Sites"
- "Code Changes - enum_path_builder.rs Call Sites"

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Validation**: Confirm the build succeeds - all field access now uses the `for_parent()` helper method

---

### Step 4: Update Serialization Logic ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: ATOMIC GROUP (Breaking change)
**Build Status**: ✅ Will compile successfully
**Dependencies**: Requires Step 2

**Objective**: Update the JSON serialization logic to pattern match on the new `PathExample` type

**Changes**:
- **types.rs**: Update `from_mutation_path_internal()` (lines 357-391)
  - Replace `path.enum_example_groups.as_ref().map_or_else(...)` patterns
  - Use explicit `match &path.example` with `PathExample::EnumRoot` and `PathExample::Simple` branches

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Implementation Details**: See "Code Changes - types.rs Conversion Logic" section below

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Validation**: Confirm the build succeeds and serialization logic correctly handles both enum variants

---

### Step 5: Verify path_builder.rs Changes ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: VALIDATION (Conditional - may not need changes)
**Build Status**: ✅ Should already compile
**Dependencies**: Requires Steps 2-4

**Objective**: Check if `path_builder.rs` has any direct `MutationPathInternal` construction that needs updating

**Changes**:
- Search for any `MutationPathInternal { ... }` construction in `path_builder.rs`
- If found, update to use `PathExample` enum instead of separate fields
- If none found, mark step as complete

**Files Modified**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` (if needed)

**Build Command**:
```bash
cargo build && cargo +nightly fmt
```

**Validation**: Confirm the build succeeds with no warnings

---

### Step 6: Final Validation ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: VALIDATION
**Build Status**: ✅ Should compile successfully
**Dependencies**: Requires all previous steps

**Objective**: Perform comprehensive validation of the completed refactoring

**Validation Steps**:
1. Run full build in release mode:
   ```bash
   cargo build --release && cargo +nightly fmt
   ```

2. Verify success criteria:
   - ✅ No compilation errors
   - ✅ No warnings about unused fields
   - ✅ All enum cases handled exhaustively
   - ✅ Code formatted with nightly rustfmt

3. Review the changes:
   - Confirm `MutationPathInternal` no longer has `enum_example_groups` or `enum_example_for_parent`
   - Confirm all construction sites use `PathExample::Simple()` or `PathExample::EnumRoot {}`
   - Confirm all access sites use `for_parent()` method or pattern matching

**Build Command**:
```bash
cargo build --release && cargo +nightly fmt
```

**Success Criteria**:
- All builds succeed with zero errors
- All code properly formatted
- Type system now prevents the enum field access bug class
- Invalid state combinations are impossible by construction

---

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
    ///
    /// This is the ONLY helper method provided. All other usage should use explicit
    /// pattern matching to maintain type safety and force exhaustive handling of both cases.
    pub fn for_parent(&self) -> &Value {
        match self {
            Self::Simple(val) => val,
            Self::EnumRoot { for_parent, .. } => for_parent,
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
   - Add `PathExample` enum (place just before `MutationPathInternal` at line ~167)
     - Requires existing imports: `Value` (already imported), `Vec`, `ExampleGroup` (defined later in same file)
     - Use `pub` visibility to match other types in this module
     - See "Code Changes - types.rs PathExample Definition" section for complete code
   - Update `MutationPathInternal` structure (lines 167-215)
     - Change `example` field type from `Value` to `PathExample`
     - Remove `enum_example_groups` field
     - Remove `enum_example_for_parent` field
   - Update `MutationPath::from_mutation_path_internal()` to pattern match on `PathExample` (lines 357-391)
     - Replace `path.enum_example_groups.as_ref().map_or_else(...)` patterns with explicit `match &path.example`
     - See detailed before/after in "Code Changes - types.rs Conversion Logic" section below

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`**
   - Update `build_enum_root_path()` (lines 1037-1051): Replace three-field initialization with `PathExample::EnumRoot`
   - Update `process_signature_path()` (lines 570-580): Use `example.for_parent()` method
   - Update `extract_child_fallback_value()` (lines 814-817): Use `example.for_parent()` method
   - See detailed changes in:
     * "Code Changes - enum_path_builder.rs Struct Initialization" section (for build_enum_root_path)
     * "Code Changes - enum_path_builder.rs Call Sites" section (for process_signature_path and extract_child_fallback_value)

3. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`**
   - Update `process_child()` (lines 167-171): Use `example.for_parent()` method
   - Update `assemble_partial_root_examples()` (lines 266-270): Use `example.for_parent()` method
   - Update `build_mutation_path_internal()` signature (line 186): Change `example: Value` to `example: PathExample`
   - Update all `build_mutation_path_internal()` call sites:
     * Line 306: `build_final_result` - wrap `example_to_use` with `PathExample::Simple(example_to_use)`
     * Line 320: `build_final_result` Skip mode - wrap `example_to_use` with `PathExample::Simple(example_to_use)`
     * Line 339: `build_not_mutable_path` - wrap `json!(null)` with `PathExample::Simple(json!(null))`
     * Line 377: `check_knowledge` TreatAsRootValue - wrap `example` with `PathExample::Simple(example)`
   - See detailed call site changes in "Code Changes - build_mutation_path_internal Call Sites" section below

4. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`**
   - Update any code that constructs `MutationPathInternal` to use `PathExample`

### Code Changes - types.rs PathExample Definition

Add this enum definition just before the `MutationPathInternal` struct (at line ~167):

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
    ///
    /// This is the ONLY helper method provided. All other usage should use explicit
    /// pattern matching to maintain type safety and force exhaustive handling of both cases.
    pub fn for_parent(&self) -> &Value {
        match self {
            Self::Simple(val) => val,
            Self::EnumRoot { for_parent, .. } => for_parent,
        }
    }
}
```

**Placement notes:**
- Insert just before `MutationPathInternal` struct (around line 167)
- All required types are already imported or defined in this file:
  - `Value` from `serde_json` (line 9)
  - `Vec` from std prelude
  - `ExampleGroup` defined later in same file (line 273)
- Use `pub` visibility to match other public types in this module
- The `#[derive(Debug, Clone)]` is sufficient - no Serialize/Deserialize needed for internal type

### Code Changes - types.rs Conversion Logic

#### Before (from_mutation_path_internal - lines 357-391)
```rust
let (examples, example) = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        // PartiallyMutable enums: show examples array with per-variant status
        // PartiallyMutable non-enums: check for Default trait
        path.enum_example_groups.as_ref().map_or_else(
            || {
                let example = if has_default_for_root {
                    Some(json!({}))
                } else {
                    None
                };
                (vec![], example)
            },
            |enum_examples| (enum_examples.clone(), None), // Enum: use examples array
        )
    }
    MutationStatus::NotMutable => {
        // NotMutable: no example at all (not even null)
        (vec![], None)
    }
    MutationStatus::Mutable => {
        path.enum_example_groups.as_ref().map_or_else(
            || {
                // Mutable paths: use the example value
                // This includes enum children (with embedded `applicable_variants`) and
                // regular values
                (vec![], Some(path.example.clone()))
            },
            |enum_examples| {
                // Enum root: use the examples array
                (enum_examples.clone(), None)
            },
        )
    }
};
```

#### After (with PathExample pattern matching)
```rust
let (examples, example) = match path.mutation_status {
    MutationStatus::PartiallyMutable => {
        match &path.example {
            PathExample::EnumRoot { groups, .. } => {
                // Enum: use examples array with per-variant status
                (groups.clone(), None)
            }
            PathExample::Simple(_) => {
                // Non-enum: check for Default trait
                let example = if has_default_for_root {
                    Some(json!({}))
                } else {
                    None
                };
                (vec![], example)
            }
        }
    }
    MutationStatus::NotMutable => {
        // NotMutable: no example at all (not even null)
        (vec![], None)
    }
    MutationStatus::Mutable => {
        match &path.example {
            PathExample::EnumRoot { groups, .. } => {
                // Enum root: use the examples array
                (groups.clone(), None)
            }
            PathExample::Simple(val) => {
                // Mutable paths: use the example value
                (vec![], Some(val.clone()))
            }
        }
    }
};
```

### Code Changes - build_mutation_path_internal Call Sites

The signature change from `example: Value` to `example: PathExample` affects 4 call sites in builder.rs:

#### Before (line 186 signature)
```rust
fn build_mutation_path_internal(
    ctx: &RecursionContext,
    example: Value,  // OLD: takes raw Value
    status: MutationStatus,
    mutation_status_reason: Option<Value>,
    partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
) -> MutationPathInternal
```

#### After (line 186 signature)
```rust
fn build_mutation_path_internal(
    ctx: &RecursionContext,
    example: PathExample,  // NEW: takes PathExample
    status: MutationStatus,
    mutation_status_reason: Option<Value>,
    partial_root_examples: Option<BTreeMap<Vec<VariantName>, Value>>,
) -> MutationPathInternal
```

#### Call Site Changes

**1. build_final_result - Create mode (line 306)**
```rust
// Before
Self::build_mutation_path_internal(
    ctx,
    example_to_use,  // Value
    parent_status,
    mutation_status_reason,
    partial_root_examples,
)

// After
Self::build_mutation_path_internal(
    ctx,
    PathExample::Simple(example_to_use),  // Wrap in PathExample::Simple
    parent_status,
    mutation_status_reason,
    partial_root_examples,
)
```

**2. build_final_result - Skip mode (line 320)**
```rust
// Before
Self::build_mutation_path_internal(
    ctx,
    example_to_use,  // Value
    parent_status,
    mutation_status_reason,
    partial_root_examples,
)

// After
Self::build_mutation_path_internal(
    ctx,
    PathExample::Simple(example_to_use),  // Wrap in PathExample::Simple
    parent_status,
    mutation_status_reason,
    partial_root_examples,
)
```

**3. build_not_mutable_path (line 339)**
```rust
// Before
Self::build_mutation_path_internal(
    ctx,
    json!(null),  // Value
    MutationStatus::NotMutable,
    Option::<Value>::from(&reason),
    None,
)

// After
Self::build_mutation_path_internal(
    ctx,
    PathExample::Simple(json!(null)),  // Wrap in PathExample::Simple
    MutationStatus::NotMutable,
    Option::<Value>::from(&reason),
    None,
)
```

**4. check_knowledge - TreatAsRootValue case (line 377)**
```rust
// Before
Some(Ok(vec![Self::build_mutation_path_internal(
    ctx,
    example,  // Value from knowledge
    MutationStatus::Mutable,
    None,
    None,
)]))

// After
Some(Ok(vec![Self::build_mutation_path_internal(
    ctx,
    PathExample::Simple(example),  // Wrap in PathExample::Simple
    MutationStatus::Mutable,
    None,
    None,
)]))
```

**Note**: enum_path_builder.rs also constructs `MutationPathInternal` directly (not via build_mutation_path_internal), which requires creating `PathExample::EnumRoot` - see below.

### Code Changes - builder.rs Call Sites

Beyond the `build_mutation_path_internal` call sites shown above, `builder.rs` has two functions that directly access the enum_example_for_parent field and need updates.

#### 1. `process_child()` (lines 432-437)

This function processes each child during recursion and extracts its example value for parent assembly.

**Before:**
```rust
// Extract child's example - handle both simple and enum root cases
let child_example = child_paths.first().map_or(json!(null), |p| {
    p.enum_example_for_parent
        .as_ref()
        .map_or_else(|| p.example.clone(), Clone::clone)
});
```

**After:**
```rust
// Extract child's example - handle both simple and enum root cases
let child_example = child_paths.first().map_or(json!(null), |p| {
    p.example.for_parent().clone()
});
```

**Why this simplifies:** The `for_parent()` method encapsulates the enum vs non-enum logic, collapsing the nested `map_or_else` into a simple method call.

#### 2. `assemble_partial_root_examples()` (lines 529-536)

This function assembles partial root examples for variant chains by collecting child values.

**Before:**
```rust
// No variant-specific value, use regular example
// For enum children, use enum_example_for_parent instead of example
// (enum paths always have example: null)
let fallback_example = child
    .enum_example_for_parent
    .as_ref()
    .map_or_else(|| child.example.clone(), Clone::clone);
examples_for_chain.insert(descriptor, fallback_example);
```

**After:**
```rust
// No variant-specific value, use regular example
// Use for_parent() which handles both Simple and EnumRoot variants
let fallback_example = child.example.for_parent().clone();
examples_for_chain.insert(descriptor, fallback_example);
```

**Why this is better:** The comment explaining the enum special case can be simplified because the type system now enforces it. The code becomes more maintainable and the intent is clearer.

### Code Changes - enum_path_builder.rs Struct Initialization

The most critical change: replacing three separate fields with a single `PathExample::EnumRoot` variant in `build_enum_root_path()`.

#### Before (lines 1037-1051)
```rust
// Direct field assignment - enums ALWAYS generate examples arrays
MutationPathInternal {
    full_mutation_path: ctx.full_mutation_path.clone(),
    example: json!(null), /* Enums always use null for the example field -
                           * they use Vec<ExampleGroup> */
    enum_example_groups: Some(enum_examples),
    enum_example_for_parent: Some(default_example),
    type_name: ctx.type_name().display_name(),
    path_kind: ctx.path_kind.clone(),
    mutation_status: enum_mutation_status,
    mutation_status_reason,
    enum_path_data: enum_data,
    depth: *ctx.depth,
    partial_root_examples: None,
}
```

#### After (with PathExample::EnumRoot)
```rust
// Direct field assignment - enums ALWAYS generate examples arrays
MutationPathInternal {
    full_mutation_path: ctx.full_mutation_path.clone(),
    example: PathExample::EnumRoot {
        groups: enum_examples,      // Was: enum_example_groups
        for_parent: default_example, // Was: enum_example_for_parent
    },
    // REMOVED: enum_example_groups field
    // REMOVED: enum_example_for_parent field
    type_name: ctx.type_name().display_name(),
    path_kind: ctx.path_kind.clone(),
    mutation_status: enum_mutation_status,
    mutation_status_reason,
    enum_path_data: enum_data,
    depth: *ctx.depth,
    partial_root_examples: None,
}
```

**Key points:**
- The `groups` and `for_parent` data are **not lost** - they're now **inside** the `PathExample::EnumRoot` variant
- The three-field pattern is replaced by a single field containing an enum variant
- The old `enum_example_groups` and `enum_example_for_parent` fields must be removed from the struct definition in types.rs

### Code Changes - enum_path_builder.rs Call Sites

**CORRECTION:** The plan previously mentioned updating `build_variant_example()`, but this is **INCORRECT**.
That function never accesses `MutationPathInternal` or `enum_example_for_parent` - it only works with
pre-extracted `HashMap<MutationPathDescriptor, Value>` data and requires **NO CHANGES**.

The actual call sites requiring updates are:

#### 1. `process_signature_path()` (lines 570-580)

This function recursively processes child paths and extracts their examples to populate the `child_examples` HashMap.

**Before:**
```rust
p.enum_example_for_parent.as_ref().map_or_else(
    || {
        // For non-enum children, use example
        tracing::debug!("Using example (no enum_example_for_parent)");
        p.example.clone()
    },
    |enum_example| {
        tracing::debug!("Using enum_example_for_parent: {enum_example:?}");
        enum_example.clone()
    },
)
```

**After (Simple version):**
```rust
p.example.for_parent().clone()
```

**After (With debug logging):**
```rust
match &p.example {
    PathExample::Simple(val) => {
        tracing::debug!("Using example (Simple variant)");
        val.clone()
    }
    PathExample::EnumRoot { for_parent, .. } => {
        tracing::debug!("Using enum_example_for_parent: {for_parent:?}");
        for_parent.clone()
    }
}
```

**Why this simplifies:** The `for_parent()` method already handles both cases (Simple and EnumRoot variants),
so the conditional logic collapses to a single method call.

#### 2. `extract_child_fallback_value()` (lines 811-818)

Standalone helper function used by `extract_child_value_for_chain()` to get fallback values when building
partial root examples.

**Before:**
```rust
fn extract_child_fallback_value(child: &MutationPathInternal) -> Value {
    // For enum children, use `enum_example_for_parent` instead of `example`
    // because enum paths always have `example: null`
    child.enum_example_for_parent.as_ref().map_or_else(
        || child.example.clone(), // Non-enum child: use regular example
        Clone::clone,             // Enum child: use selected variant example
    )
}
```

**After:**
```rust
fn extract_child_fallback_value(child: &MutationPathInternal) -> Value {
    // Use for_parent() method which handles both Simple and EnumRoot variants
    child.example.for_parent().clone()
}
```

**Why this is better:** The entire function collapses to a single line because `PathExample::for_parent()`
encapsulates the branching logic that was previously explicit.

#### Why These Are The Only Sites

These are the only two locations in `enum_path_builder.rs` that:
1. Have a `MutationPathInternal` reference
2. Need to extract the "for parent" value from it
3. Currently use `.enum_example_for_parent` field access

All other usages are either:
- Field assignments in struct construction (line 1043 - will be removed with struct change)
- Debug logging of field presence (line 566 - can be updated to check variant instead)
- Comments and documentation (no code changes needed)
- Functions like `build_variant_example()` that work with already-extracted values (no changes needed)

### Code Changes - builder.rs Examples

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

// NEW: Crystal clear - explicit pattern matching
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
