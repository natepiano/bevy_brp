# Root Example Collision Fix

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
   For Python scripts:
   ```bash
   python -m py_compile <file>
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

---

## PROBLEM OVERVIEW

### The Core Issue

When multiple enum variants share the same field names, they create mutation paths with identical keys that collide in the HashMap, silently overwriting all but the last variant.

### Example with `Color` enum

All 10 color variants have an `alpha` field:
- `Xyza` → creates `.0.alpha` path
- `Hsla` → creates `.0.alpha` path
- `Srgba` → creates `.0.alpha` path
- etc.

Currently in `api.rs:47-58`, these paths are collected into a `HashMap<String, MutationPathExternal>` where the key is the mutation_path string. **Only the last variant wins** - all others are silently overwritten.

### Current Behavior
```json
{
  "mutation_paths": {
    ".0.alpha": {
      "applicable_variants": ["Color::Xyza"],  // Only shows last variant!
      "root_example": {"Xyza": {"alpha": 1.0, "x": 1.0, "y": 1.0, "z": 1.0}}
    }
  }
}
```

### Why This Happens

In `api.rs:47-58`:
```rust
let external_paths = internal_paths
    .iter()
    .map(|mutation_path_internal| {
        let key = (*mutation_path_internal.mutation_path).clone();  // ".0.alpha"
        let mutation_path = mutation_path_internal
            .clone()
            .into_mutation_path_external(&registry);
        (key, mutation_path)  // All variants create same key!
    })
    .collect();  // Last entry wins, others silently discarded
```

## SOLUTION: Change HashMap to Array + Remove Signature Grouping

This fix involves two complementary changes:

1. **Change data structure from HashMap to Vec** - allows duplicate path keys
2. **Remove signature grouping** - process each variant independently for explicit, self-contained entries

### Why Remove Signature Grouping?

**Current grouping approach:**
- Groups variants by signature (e.g., `Color::Xyza(XyzColorSpace)` and `Color::Hsla(HslaColorSpace)` might group together if they have similar signatures)
- Creates ONE entry per path per signature group
- Uses `applicable_variants: ["VariantA", "VariantB"]` to show which variants share the path
- Picks a "representative variant" for `root_example`
- User must infer: "I can also use VariantB since it's listed in `applicable_variants`"

**Problems with grouping:**
- **Implicit > Explicit**: Requires inference about which variants work
- **Doesn't prevent collisions**: Different signatures with same field names still collide in HashMap
- **Complex code**: ~200 lines of grouping logic in `enum_path_builder.rs`
- **Unnecessary with Vec**: Grouping was an optimization for HashMap - Vec makes it obsolete

**New variant-by-variant approach:**
- Process each variant independently (no grouping)
- Each variant gets its own entry with explicit `root_example`
- Remove `applicable_variants` field entirely
- Clear 1:1 mapping: one entry = one variant = one root_example

### Current Structure (HashMap with Grouping)
```rust
HashMap<String, MutationPathExternal>

// JSON output:
{
  "mutation_paths": {
    ".0.alpha": {
      "description": "Mutate the 'alpha' field...",
      "applicable_variants": ["Color::Xyza", "Color::Hsla", "Color::Srgba"],  // Grouped!
      "root_example": {"Xyza": {"alpha": 1.0, "x": 1.0, "y": 1.0, "z": 1.0}}  // Representative
    }
  }
}
```

### New Structure (Vec without Grouping)
```rust
Vec<MutationPathExternal>

// JSON output:
{
  "mutation_paths": [
    {
      "path": ".0.alpha",
      "description": "Mutate the 'alpha' field in Xyza variant",
      "root_example": {"Xyza": {"alpha": 1.0, "x": 1.0, "y": 1.0, "z": 1.0}}
    },
    {
      "path": ".0.alpha",  // Same path, different variant - explicit!
      "description": "Mutate the 'alpha' field in Hsla variant",
      "root_example": {"Hsla": {"alpha": 1.0, "hue": 1.0, "saturation": 1.0, "lightness": 1.0}}
    },
    {
      "path": ".0.alpha",  // Third variant with same path
      "description": "Mutate the 'alpha' field in Srgba variant",
      "root_example": {"Srgba": {"red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0}}
    },
    {
      "path": ".0.hue",
      "description": "Mutate the 'hue' field in Hsla variant",
      "root_example": {"Hsla": {"alpha": 1.0, "hue": 1.0, "saturation": 1.0, "lightness": 1.0}}
    }
  ]
}
```

---

## INTERACTIVE IMPLEMENTATION SEQUENCE

### STEP 1: Core Type System Refactoring ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: ATOMIC GROUP - CRITICAL
**Build Status**: ⚠️ Will not compile until Step 3 complete

**Objective**: Remove ExampleGroup and PathExample wrapper types, add direct path and example fields to MutationPathExternal

#### Why This Change

With HashMap structure, one path key could represent multiple enum variants, requiring:
- `PathExample` enum wrapper to handle either single example OR array of grouped examples
- `ExampleGroup` struct to bundle multiple variants sharing same signature

With Vec structure, each variant gets its own entry with its own example. No grouping needed.

#### Files to Modify

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`**

#### Detailed Changes

##### 1. Remove ExampleGroup struct (lines 199-211)

**Current:**
```rust
pub struct ExampleGroup {
    pub applicable_variants: Vec<VariantName>,
    pub example: Option<Value>,
    pub signature: VariantSignature,
    pub mutability: Mutability,
}
```

**Action**: Delete this entire struct

##### 2. Remove PathExample enum (lines 57-80)

**Current:**
```rust
pub enum PathExample {
    Simple(Value),
    EnumRoot { groups: Vec<ExampleGroup> },
}
```

**Action**: Delete this entire enum

##### 3. Remove PathExample Serialize impl (lines 82-112)

**Action**: Delete the entire Serialize implementation for PathExample

##### 4. Remove PathExample Deserialize stub (lines 114-127)

**Action**: Delete the entire Deserialize implementation for PathExample

##### 5. Update MutationPathExternal (lines 228-238)

**Current:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    pub description:  String,
    pub path_info:    PathInfo,
    #[serde(flatten)]
    pub path_example: PathExample,  // ← Wrapper removed
}
```

**New:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    pub path:         MutationPath,  // NEW: mutation path as a field
    pub description:  String,
    #[serde(flatten)]
    pub path_info:    PathInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:      Option<Value>,  // NEW: direct field, no wrapper
}
```

##### 6. Update MutationPathInternal - add example field

**Current:** (no example field)

**New:** Add field to store example for transfer during conversion:
```rust
pub struct MutationPathInternal {
    pub mutation_path: MutationPath,
    pub description: String,
    pub example: Option<Value>,  // NEW: Store example here
    // ... other fields ...
}
```

##### 7. Remove applicable_variants from PathInfo

**Current:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    pub path_kind:           PathKind,
    pub type_name:           BrpTypeName,
    pub type_kind:           TypeKind,
    pub mutability:          Mutability,
    pub mutability_reason:   Option<Value>,
    pub enum_instructions:   Option<String>,
    pub applicable_variants: Option<Vec<VariantName>>,  // ❌ Remove this
    pub root_example:        Option<Value>,
}
```

**New:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    pub path_kind:           PathKind,
    pub type_name:           BrpTypeName,
    pub type_kind:           TypeKind,
    pub mutability:          Mutability,
    pub mutability_reason:   Option<Value>,
    pub enum_instructions:   Option<String>,
    // applicable_variants field removed
    pub root_example:        Option<Value>,
}
```

##### 8. Remove applicable_variants from EnumPathData

**Current:**
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    pub applicable_variants: Vec<VariantName>,  // ❌ Remove this
    pub root_example: Option<Value>,
}
```

**New:**
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    // applicable_variants field removed
    pub root_example: Option<Value>,
}
```

##### 9. Update ProcessChildrenResult type alias

**Current:**
```rust
type ProcessChildrenResult = (
    Vec<ExampleGroup>,                      // ← Remove
    Vec<MutationPathInternal>,
    HashMap<Vec<VariantName>, Value>
);
```

**New:**
```rust
type ProcessChildrenResult = (
    Vec<MutationPathInternal>,              // Just paths with embedded examples
    HashMap<VariantName, Value>             // Per-variant (not per-signature-group)
);
```

#### Build Command

```bash
cargo build  # Will fail - part of atomic group
```

**Expected**: Compilation errors due to missing types/fields that will be fixed in Steps 2-3

---

### STEP 2: API Layer Changes ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: ATOMIC GROUP - CRITICAL
**Build Status**: ⚠️ Will not compile until Step 3 complete
**Dependencies**: Requires Step 1

**Objective**: Change API return type from HashMap to Vec, update path extraction logic

#### Files to Modify

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`**
2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`**

#### Detailed Changes

##### 1. Update build_mutation_paths in api.rs (Lines 28-61)

**Current:**
```rust
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<HashMap<String, MutationPathExternal>> {
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            let key = (*mutation_path_internal.mutation_path).clone();  // Extract key
            let mutation_path = mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry);
            (key, mutation_path)  // Tuple for HashMap
        })
        .collect();
    Ok(external_paths)
}
```

**New:**
```rust
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<Vec<MutationPathExternal>> {
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry)  // No tuple needed
        })
        .collect();
    Ok(external_paths)
}
```

**Key changes:**
- Return type: `HashMap<String, MutationPathExternal>` → `Vec<MutationPathExternal>`
- Remove tuple mapping with key (no longer needed - path becomes a field)
- Simple map and collect

##### 2. Update extract_spawn_format in api.rs (Lines 63-76)

**Current:**
```rust
pub fn extract_spawn_format(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value> {
    mutation_paths.get("")  // Get root path from HashMap key
}
```

**New:**
```rust
pub fn extract_spawn_format(
    mutation_paths: &[MutationPathExternal],
) -> Option<Value> {
    mutation_paths
        .iter()
        .find(|path| path.path.is_empty())  // Find root path by field
        .and_then(|path| path.example.clone())
}
```

**Key changes:**
- Parameter type: `&HashMap<String, MutationPathExternal>` → `&[MutationPathExternal]`
- Logic: HashMap key lookup → array iteration with field check

##### 3. Update into_mutation_path_external in mutation_path_internal.rs (Lines 76-110)

**Current:**
```rust
pub fn into_mutation_path_external(
    mut self,
    registry: &HashMap<BrpTypeName, Value>,
) -> MutationPathExternal {
    MutationPathExternal {
        description,
        path_info: PathInfo { ... },
        path_example,  // PathExample wrapper
    }
}
```

**New:**
```rust
pub fn into_mutation_path_external(
    mut self,
    registry: &HashMap<BrpTypeName, Value>,
) -> MutationPathExternal {
    MutationPathExternal {
        path: self.mutation_path.clone(),  // NEW: populate path field
        description,
        path_info: PathInfo {
            // ... existing fields (except remove applicable_variants) ...
        },
        example: self.example,  // NEW: direct assignment from internal
    }
}
```

**Key changes:**
- Add `path: self.mutation_path.clone()` (use existing field)
- Replace `path_example: PathExample::Simple(...)` with `example: self.example`
- Remove `applicable_variants` field population (field no longer exists)

#### Build Command

```bash
cargo build  # Will fail - part of atomic group
```

**Expected**: Compilation errors in enum_path_builder.rs that will be fixed in Step 3

---

### STEP 3: Enum Processing Refactor ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: ATOMIC GROUP - CRITICAL
**Build Status**: ✅ Compiles successfully after this step
**Dependencies**: Requires Steps 1-2

**Objective**: Remove signature grouping logic, process each variant independently

#### Files to Modify

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`** ⭐ MAJOR CHANGES

#### Detailed Changes

This is the most complex refactor. We're removing ~200 lines of grouping logic and replacing it with straightforward variant-by-variant processing.

##### 1. Remove group_variants_by_signature function

**Current:** (lines ~197-222) - entire function that groups variants by signature

**Action**: Delete this entire function (~25 lines)

##### 2. Replace with extract_all_variants helper

**New function to add:**
```rust
/// Extract all variants from schema as a flat list
fn extract_all_variants(
    ctx: &RecursionContext,
) -> Result<Vec<VariantKind>> {
    let schema = ctx.require_registry_schema()?;

    let one_of_array = schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Report::new(Error::InvalidState(format!(
                "Enum type {} missing oneOf field in schema",
                ctx.type_name()
            )))
        })?;

    one_of_array
        .iter()
        .map(|v| VariantKind::from_schema_variant(v, &ctx.registry, ctx.type_name()))
        .collect::<Result<Vec<_>>>()
}
```

##### 3. Rename and refactor process_signature_groups → process_all_variants

**Old signature** (line 401):
```rust
fn process_signature_groups(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError>
```

**New signature:**
```rust
fn process_all_variants(
    variant_kinds: Vec<VariantKind>,  // Flat list with both name and signature
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError>
```

**Implementation changes inside process_all_variants:**

**Old logic:** (lines ~408-454)
```rust
for (variant_signature, variant_names) in variant_groups.sorted() {
    // Process all variants in group together
    // Build one ExampleGroup for the group
}
```

**New logic:**
```rust
let mut child_mutation_paths = Vec::new();
let mut partial_root_examples = HashMap::new();

for variant_kind in variant_kinds {
    let variant_name = variant_kind.name;
    let variant_signature = variant_kind.signature;

    // Create FRESH child_examples HashMap for each variant
    let mut child_examples = HashMap::new();

    // Process this single variant independently
    let variant_paths = process_variant_path(
        PathKind::EnumRoot,
        &variant_name,           // Single variant, not array
        &variant_signature,
        ctx,
        &mut child_examples,
    )?;

    child_mutation_paths.extend(variant_paths);

    // Build example for THIS variant
    let example = build_variant_example_value(
        &variant_signature,
        &variant_name,
        &child_examples,
        mutability,
        ctx,
    )?;

    // Store in partial_root_examples (per-variant key)
    partial_root_examples.insert(variant_name.clone(), example);

    // NO MORE: examples.push(ExampleGroup { ... });
}

Ok((child_mutation_paths, partial_root_examples))
```

##### 4. Refactor process_signature_path → process_variant_path

**Current function name**: `process_signature_path` (reflects signature-group processing)

**New function name**: `process_variant_path` (reflects single-variant processing)

**Old signature:**
```rust
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],  // Array of variants in signature group
    signature: &VariantSignature,
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> Result<Vec<MutationPathInternal>, BuilderError>
```

**New signature:**
```rust
fn process_variant_path(
    path: PathKind,
    variant_name: &VariantName,      // Single variant, not array
    signature: &VariantSignature,     // Still needed for context
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> Result<Vec<MutationPathInternal>, BuilderError>
```

**Changes inside process_variant_path:**

**Line 239 - Simplify variant_chain construction:**

Old:
```rust
let variant_chain = if let Some(first_variant) = applicable_variants.first() {
    let mut chain = parent_data.variant_chain.clone();
    chain.push(first_variant.clone());
    chain
} else {
    parent_data.variant_chain.clone()
};
```

New:
```rust
let mut variant_chain = parent_data.variant_chain.clone();
variant_chain.push(variant_name.clone());
```

**Lines 252-263 - Remove applicable_variants population loop:**

Old:
```rust
for (mutation_path, path_info) in &mut child_paths {
    if let Some(enum_path_data) = &mut path_info.enum_path_data {
        // Populate applicable_variants for this signature group
        enum_path_data.applicable_variants = applicable_variants.to_vec();
    }
}
```

New: (delete this entire loop - field no longer exists)

##### 5. Update calling context in build_enum_paths (lines 98-103)

**Old:**
```rust
let variants_grouped_by_signature = group_variants_by_signature(ctx)?;
let (enum_examples, child_mutation_paths, partial_root_examples) =
    process_signature_groups(&variants_grouped_by_signature, ctx)?;
```

**New:**
```rust
let variant_kinds = extract_all_variants(ctx)?;
let (child_mutation_paths, partial_root_examples) =
    process_all_variants(variant_kinds, ctx)?;
```

**Note**: `enum_examples` is removed from the return tuple (no more ExampleGroup)

##### 6. Remove select_preferred_example function

**Current:** (line ~113) - function used to choose default example from ExampleGroups

**Action**: Delete this entire function - not needed when each variant produces its own entries

##### 7. Update build_partial_root_examples signature

**Old:**
```rust
fn build_partial_root_examples(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    examples: &[ExampleGroup],                // ← Remove parameter
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<Vec<VariantName>, Value>  // Grouped key
```

**New:**
```rust
fn build_partial_root_examples(
    variant_kinds: &[VariantKind],
    child_mutation_paths: &[MutationPathInternal],
    ctx: &RecursionContext,
) -> HashMap<VariantName, Value>  // Per-variant key
```

**Implementation**: This function is now simpler - just iterate variant_kinds and build examples per-variant (no grouping logic).

##### 8. Root path handling changes

**Old approach:**
- One root path ("") entry with `PathExample::EnumRoot { groups: Vec<ExampleGroup> }`
- One entry with multiple grouped variants

**New approach:**
- Multiple root path ("") entries, one per variant
- Each with its own `example` field
- Each self-contained

**Example output:**
```json
[
  {"path": "", "example": {"Color::Srgba": [1.0, 0.0, 0.0, 1.0]}},
  {"path": "", "example": {"Color::Hsla": {"hue": 0.0, ...}}},
  {"path": ".0.alpha", "example": {"Color::Srgba": [1.0, 0.0, 0.0, 0.5]}},
  {"path": ".0.alpha", "example": {"Color::Hsla": {"hue": 0.0, ..., "alpha": 0.5}}}
]
```

#### Build Command

```bash
cargo build && cargo +nightly fmt
```

**Expected**: ✅ Compilation succeeds - atomic group complete!

---

### STEP 4: Type Knowledge Migration ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: SAFE
**Build Status**: ✅ Compiles successfully
**Dependencies**: Requires Step 3

**Objective**: Replace EnumVariantSignature with EnumVariant knowledge type

#### Why This Change

The old `EnumVariantSignature` key used signature + index (indirect matching). The new `EnumVariant` key uses variant_name (direct and explicit), matching our new per-variant processing approach.

#### Files to Modify

1. **`mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`**

#### Detailed Changes

##### 1. Remove EnumVariantSignature variant from KnowledgeKey enum

**Current:**
```rust
pub enum KnowledgeKey {
    // ... other variants ...
    EnumVariantSignature {
        enum_type: BrpTypeName,
        signature: VariantSignature,  // ❌ Too indirect
        index: usize,
    },
}
```

**Action**: Delete this entire variant

##### 2. Add EnumVariant variant to KnowledgeKey enum

**New:**
```rust
pub enum KnowledgeKey {
    // ... other variants ...
    EnumVariant {
        enum_type: BrpTypeName,
        variant_name: VariantName,  // ✅ Direct and explicit - MUST use VariantName newtype
    },
}
```

**Type specification:**
- `enum_type`: `BrpTypeName` newtype
- `variant_name`: `VariantName` newtype (NOT `String`)
  - VariantName is a newtype wrapper providing type safety
  - Always use `VariantName::from("Color::Srgba")` pattern
  - Never use plain strings for variant names

##### 3. Remove enum_variant_signature helper function (lines 119-131)

**Current:**
```rust
/// Create an enum variant signature match key
pub fn enum_variant_signature(
    enum_type: impl Into<BrpTypeName>,
    signature: VariantSignature,
    index: usize,
) -> Self {
    Self::EnumVariantSignature {
        enum_type: enum_type.into(),
        signature,
        index,
    }
}
```

**Action**: Delete this entire function

##### 4. Add enum_variant helper function

**New:**
```rust
/// Create an enum variant match key
pub fn enum_variant(
    enum_type: impl Into<BrpTypeName>,
    variant_name: impl Into<VariantName>,
) -> Self {
    Self::EnumVariant {
        enum_type: enum_type.into(),
        variant_name: variant_name.into(),
    }
}
```

**Purpose**: Provides cleaner syntax for creating `EnumVariant` keys, reducing verbosity

##### 5. Migrate all 10 Color variant knowledge entries (lines 451-549)

**Migration requirement**: All 10 Color variant entries MUST be migrated from `EnumVariantSignature` to `EnumVariant` format to preserve the existing "treat as opaque root values" behavior.

**Deriving Variant Names**: The variant names (e.g., `"Color::Srgba"`) are dynamically derived during normal schema processing - NOT hardcoded.

**Complete list of Color variants** (as derived from schema):
- `"Color::Srgba"`, `"Color::LinearRgba"`, `"Color::Hsla"`, `"Color::Hsva"`, `"Color::Hwba"`
- `"Color::Laba"`, `"Color::Lcha"`, `"Color::Oklaba"`, `"Color::Oklcha"`, `"Color::Xyza"`

**Conversion pattern** (apply to all 10):

Before (signature-based):
```rust
map.insert(
    KnowledgeKey::enum_variant_signature(
        TYPE_BEVY_COLOR,
        VariantSignature::Tuple(vec![BrpTypeName::from(TYPE_BEVY_COLOR_SRGBA)]),
        0,
    ),
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);
```

After (variant-based):
```rust
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Srgba"),
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);
```

**Complete migration code for all 10 variants:**

```rust
// 1. Srgba variant - array format [r, g, b, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Srgba"),
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);

// 2. LinearRgba variant - array format [r, g, b, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::LinearRgba"),
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_LINEAR_RGBA),
);

// 3. Hsla variant - array format [h, s, l, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Hsla"),
    TypeKnowledge::as_root_value(json!([180.0, 0.5, 0.5, 1.0]), TYPE_BEVY_COLOR_HSLA),
);

// 4. Hsva variant - array format [h, s, v, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Hsva"),
    TypeKnowledge::as_root_value(json!([240.0, 0.7, 0.9, 1.0]), TYPE_BEVY_COLOR_HSVA),
);

// 5. Hwba variant - array format [h, w, b, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Hwba"),
    TypeKnowledge::as_root_value(json!([60.0, 0.2, 0.1, 1.0]), TYPE_BEVY_COLOR_HWBA),
);

// 6. Laba variant - array format [l, a, b, alpha]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Laba"),
    TypeKnowledge::as_root_value(json!([0.5, 0.3, 0.2, 1.0]), TYPE_BEVY_COLOR_LABA),
);

// 7. Lcha variant - array format [l, c, h, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Lcha"),
    TypeKnowledge::as_root_value(json!([0.6, 0.4, 90.0, 1.0]), TYPE_BEVY_COLOR_LCHA),
);

// 8. Oklaba variant - array format [l, a, b, alpha]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Oklaba"),
    TypeKnowledge::as_root_value(json!([0.55, 0.15, 0.25, 1.0]), TYPE_BEVY_COLOR_OKLABA),
);

// 9. Oklcha variant - array format [l, c, h, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Oklcha"),
    TypeKnowledge::as_root_value(json!([0.65, 0.35, 150.0, 1.0]), TYPE_BEVY_COLOR_OKLCHA),
);

// 10. Xyza variant - array format [x, y, z, a]
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Xyza"),
    TypeKnowledge::as_root_value(json!([0.8, 0.7, 0.6, 1.0]), TYPE_BEVY_COLOR_XYZA),
);
```

**Migration checklist:**
- ✅ All 10 variants accounted for (Srgba, LinearRgba, Hsla, Hsva, Hwba, Laba, Lcha, Oklaba, Oklcha, Xyza)
- ✅ Each uses `KnowledgeKey::enum_variant` (not `enum_variant_signature`)
- ✅ Each uses `VariantName::from()` with correct variant name
- ✅ All example JSON values preserved exactly
- ✅ All type constants preserved (TYPE_BEVY_COLOR_SRGBA, etc.)

#### Build Command

```bash
cargo build && cargo +nightly fmt
```

**Expected**: ✅ Compilation succeeds

---

### STEP 5: Guide Module Updates ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: SAFE
**Build Status**: ✅ Compiles successfully
**Dependencies**: Requires Steps 1-3

**Objective**: Update guide module to use Vec instead of HashMap

#### Files to Modify

1. **`mcp/src/brp_tools/brp_type_guide/guide.rs`**

#### Detailed Changes

##### 1. Remove HashMap import (line 10)

**Current:**
```rust
use std::collections::HashMap;
```

**New:**
```rust
// (remove this import - no longer needed)
```

##### 2. Update mutation_paths field (lines 46-47)

**Current:**
```rust
#[serde(skip_serializing_if = "HashMap::is_empty")]
pub mutation_paths: HashMap<String, MutationPathExternal>,
```

**New:**
```rust
#[serde(skip_serializing_if = "Vec::is_empty")]
pub mutation_paths: Vec<MutationPathExternal>,
```

##### 3. Update constructor calls (lines 105, 121)

**Current:**
```rust
mutation_paths: HashMap::new(),
```

**New:**
```rust
mutation_paths: Vec::new(),
```

##### 4. Update generate_agent_guidance parameter (line 134)

**Current:**
```rust
fn generate_agent_guidance(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Result<String>
```

**New:**
```rust
fn generate_agent_guidance(
    mutation_paths: &[MutationPathExternal],
) -> Result<String>
```

##### 5. Update iteration in generate_agent_guidance (lines 137-139)

**Current:**
```rust
let has_entity = mutation_paths
    .values()
    .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));
```

**New:**
```rust
let has_entity = mutation_paths
    .iter()
    .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));
```

##### 6. Update extract_spawn_format_if_spawnable parameter (line 158)

**Current:**
```rust
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value>
```

**New:**
```rust
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &[MutationPathExternal],
) -> Option<Value>
```

#### Build Command

```bash
cargo build && cargo +nightly fmt
```

**Expected**: ✅ Compilation succeeds

---

### STEP 6: Python Script Updates ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: SAFE - Rust must be complete
**Build Status**: ✅ No compilation (Python)
**Dependencies**: Requires Step 5 (Rust changes complete)

**Objective**: Update Python scripts to work with array structure

**CRITICAL**: Add `Required` import FIRST before any other changes

#### Files to Modify

1. **`.claude/scripts/mutation_test_prepare.py`**
2. **`.claude/scripts/create_mutation_test_json/read_comparison.py`**
3. **`.claude/scripts/create_mutation_test_json/compare.py`**

#### Detailed Changes

##### 1. mutation_test_prepare.py

**Line 23: Add Required import for TypedDict**

**Current:**
```python
from typing import Any, TypedDict, cast
```

**New:**
```python
from typing import Any, TypedDict, Required, cast
```

**Rationale**: The `MutationPathData` TypedDict needs `Required[str]` for the new `path` field. Without this import, the script will fail immediately with a NameError.

**Lines 27-31: Add path field as Required**

**Current:**
```python
class MutationPathData(TypedDict, total=False):
    description: str
    example: Any  # pyright: ignore[reportExplicitAny]
    path_info: dict[str, str]
```

**New:**
```python
class MutationPathData(TypedDict, total=False):
    path: Required[str]  # NEW: Required field (every entry must have path)
    description: str
    example: Any  # pyright: ignore[reportExplicitAny]
    path_info: dict[str, str]
```

**Explanation**: `path` is marked as `Required[str]` because every mutation path entry MUST have a path field (it's required in the Rust `MutationPathExternal` struct). Other fields remain optional via `total=False`.

**Line 37: Change type annotation**

**Current:**
```python
mutation_paths: dict[str, MutationPathData] | None
```

**New:**
```python
mutation_paths: list[MutationPathData] | None
```

**Line 257: Change default value**

**Current:**
```python
mutation_paths = type_data.get("mutation_paths") or {}
```

**New:**
```python
mutation_paths = type_data.get("mutation_paths") or []
```

**Lines 337-340: Update iteration pattern**

**Current:**
```python
for path, path_info in mutation_paths.items():
    # Skip non-mutable paths
    if path_info.get("path_info", {}).get("mutability") == "not_mutable":
```

**New:**
```python
for path_entry in mutation_paths:
    path = path_entry["path"]  # Access required field directly
    # Skip non-mutable paths
    if path_entry.get("path_info", {}).get("mutability") == "not_mutable":
```

**Note**: Since `path` is marked as `Required`, we access it directly with `path_entry["path"]` rather than `.get("path", "")`. This will cause type checker errors if the field is missing, which is the desired behavior.

##### 2. read_comparison.py (Lines 355-362)

**Current:**
```python
mutation_paths = type_data["mutation_paths"]
if mutation_path not in mutation_paths:
    return None
return mutation_paths[mutation_path]
```

**New:**
```python
mutation_paths = type_data["mutation_paths"]
if not isinstance(mutation_paths, list):
    return None
for path_data in mutation_paths:
    if isinstance(path_data, dict) and path_data.get("path") == mutation_path:
        return path_data
return None
```

##### 3. compare.py (Lines 419-427)

**Current:**
```python
total_paths = sum(
    len(cast(dict[str, JsonValue], t.get("mutation_paths", {})))
    if isinstance(t.get("mutation_paths"), dict)
    else 0
    for t in data.values()
    if isinstance(t, dict)
)
```

**New:**
```python
total_paths = sum(
    len(cast(list[JsonValue], t.get("mutation_paths", [])))
    if isinstance(t.get("mutation_paths"), list)
    else 0
    for t in data.values()
    if isinstance(t, dict)
)
```

#### Build Command

```bash
python -m py_compile .claude/scripts/mutation_test_prepare.py
python -m py_compile .claude/scripts/create_mutation_test_json/read_comparison.py
python -m py_compile .claude/scripts/create_mutation_test_json/compare.py
```

**Expected**: ✅ No syntax errors

---

### STEP 7: Shell Script Updates ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: SAFE - Rust must be complete
**Build Status**: ✅ No compilation (Bash)
**Dependencies**: Requires Step 5 (Rust changes complete)

**Objective**: Update shell scripts and jq filters for array structure

#### Files to Modify

1. **`.claude/scripts/create_mutation_test_json/augment_response.sh`**
2. **`.claude/scripts/get_type_kind.sh`**
3. **`.claude/scripts/get_mutation_path.sh`**
4. **`.claude/scripts/get_mutation_path_list.sh`**
5. **`.claude/scripts/type_guide_test_extract.sh`**

#### Detailed Changes

##### 1. augment_response.sh (Lines 85-98, 122, 126)

**Lines 85-98: Complete rewrite of test_status logic**

OLD logic (HashMap-based):
```bash
if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == {}) then
    "passed"
elif (($entry.value.mutation_paths | type == "object") and ($entry.value.mutation_paths | length == 1) and ($entry.value.mutation_paths | has(""))) then
    if (($entry.value.mutation_paths[""].path_info.mutability // "") == "not_mutable") then
        "passed"
    elif ($entry.value.mutation_paths[""].example == {} and ($entry.value.mutation_paths[""].examples == null or $entry.value.mutation_paths[""].examples == [])) then
        "passed"
    else
        "untested"
    end
else
    "untested"
end
```

NEW logic (Vec-based):
```bash
if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == []) then
    "passed"
elif (($entry.value.mutation_paths | type == "array") and ($entry.value.mutation_paths | length == 1) and ($entry.value.mutation_paths[0].path == "")) then
    # Single entry with empty path (root only)
    if (($entry.value.mutation_paths[0].path_info.mutability // "") == "not_mutable") then
        "passed"
    elif ($entry.value.mutation_paths[0].example == {} and ($entry.value.mutation_paths[0].examples == null or $entry.value.mutation_paths[0].examples == [])) then
        "passed"
    else
        "untested"
    end
else
    "untested"
end
```

**Key changes:**
- Empty check: `== {}` → `== []`
- Type check: `type == "object"` → `type == "array"`
- Root path check: `has("")` → `[0].path == ""`
- Field access: `[""]` → `[0]`

**Line 122: Update types_with_mutations filter**
```bash
# OLD: select(.value.mutation_paths != null and .value.mutation_paths != {})
# NEW:
select(.value.mutation_paths != null and .value.mutation_paths != [])
```

**Line 126: Update total_mutation_paths calculation**
```bash
# OLD: .value.mutation_paths // {} | keys | .[]
# NEW: .value.mutation_paths // [] | .[]
```

**Explanation**: The new logic iterates the array directly (counts path objects, not unique path strings). Duplicate path strings will be counted multiple times - this is intended behavior.

##### 2. get_type_kind.sh (Lines 36-38, 73-74)

**Current:**
```python
for path, path_data in guide['mutation_paths'].items():
```

**New:**
```python
for path_data in guide['mutation_paths']:
    # If you need the path string, access it:
    # path = path_data['path']
```

##### 3. get_mutation_path.sh (Lines 108-150)

**Line 121:**
```python
# OLD: for i, path in enumerate(list(mutation_paths.keys())[:20]):
for i, path_obj in enumerate(mutation_paths[:20]):
    path = path_obj['path']
```

**Lines 134-145:**
```python
# OLD: if mutation_path not in mutation_paths:
path_data = None
for path_obj in mutation_paths:
    if path_obj['path'] == mutation_path:
        path_data = path_obj
        break

if path_data is None:
    # ... error handling ...
    matching = [path_obj['path'] for path_obj in mutation_paths if mutation_path in path_obj['path']]
```

##### 4. get_mutation_path_list.sh (Lines 77-88)

**Current:**
```python
for path in mutation_paths.keys():
```

**New:**
```python
for path_obj in mutation_paths:
    path = path_obj['path']
```

##### 5. type_guide_test_extract.sh (Lines 66-73)

**Current:**
```bash
jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | has($path)'
```

**New:**
```bash
jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | any(.path == $path)'
```

#### Build Command

```bash
shellcheck .claude/scripts/create_mutation_test_json/augment_response.sh  # Optional
shellcheck .claude/scripts/get_type_kind.sh  # Optional
shellcheck .claude/scripts/get_mutation_path.sh  # Optional
shellcheck .claude/scripts/get_mutation_path_list.sh  # Optional
shellcheck .claude/scripts/type_guide_test_extract.sh  # Optional
```

**Expected**: ✅ No syntax errors (shellcheck is optional)

---

### STEP 8: Documentation Updates ⏳ PENDING

**Status**: ⏳ PENDING
**Type**: SAFE - No dependencies
**Build Status**: ✅ No compilation
**Dependencies**: None (can be done anytime after Step 5)

**Objective**: Update all documentation to reflect array structure

#### Files to Modify

1. **`.claude/commands/create_mutation_test_json.md`**
2. **`.claude/commands/get_guide_current.md`**
3. **`.claude/commands/get_guide_baseline.md`**
4. **`.claude/commands/get_kind_baseline.md`**
5. **`.claude/commands/get_path_baseline.md`**
6. **`.claude/commands/compare_mutation_path.md`**
7. **`.claude/integration_tests/type_guide.md`**
8. **`.claude/integration_tests/data_operations.md`**

#### Detailed Changes

##### 1. create_mutation_test_json.md

**Lines 159-161:**
```markdown
# OLD: Complete mutation_paths as objects with path keys and example values
Complete mutation_paths as arrays of objects containing path and example data
```

**Lines 304-313 (CRITICAL section):**
```markdown
<MutationPathsExplanation>
**Understanding Mutation Paths Structure**

Mutation paths are stored as an array of objects, NOT a dictionary:
- **Structure**: `mutation_paths` is an array where each element has a `path` field
- **Example path**: `.image_mode.0.center_scale_mode`
- **Access**: `[obj for obj in type_guide['TypeName']['mutation_paths'] if obj['path'] == '.image_mode.0.center_scale_mode'][0]`
- **Alternative**: Use helper functions to find paths by string key

Path notation patterns: `.field.0` (variant), `.field[0]` (array), `.field.0.nested` (nested in variant)
</MutationPathsExplanation>
```

##### 2. get_guide_current.md (Lines 102-105)

**Current:**
```json
"mutation_paths": {
  "": { /* root mutation */ },
  ".field": { /* field mutations */ }
}
```

**New:**
```json
"mutation_paths": [
  { "path": "", /* root mutation */ },
  { "path": ".field", /* field mutations */ }
]
```

##### 3. get_guide_baseline.md (Lines 84-87)

Same change as above - show array structure with path fields.

##### 4. get_kind_baseline.md (Line 50)

**Current:**
```markdown
The baseline file must have the expected structure with `type_guide` array containing types with `mutation_paths`
```

**New:**
```markdown
The baseline file must have the expected structure with `type_guide` containing types with `mutation_paths` arrays
```

##### 5. get_path_baseline.md (Lines 94-104)

Update output format to show flattened structure with path as a field.

##### 6. compare_mutation_path.md (Lines 141-148)

Update terminology from "dict values" to "array elements" and "path objects".

##### 7. type_guide.md (integration test)

**Lines 66-67:** Add note about array format
**Lines 81-82:** Explicitly state Vec3/Quat use array format
**Lines 186-187:** Add note that object format intentionally fails
**Lines 196-197:** Emphasize array format in error validation
**Lines 238-251:** Add explicit wrong format notes

##### 8. data_operations.md (integration test)

**Lines 13-14:** Add critical note about array format requirement
**Lines 37, 40, 45:** Update to reflect type_guide returns array
**Line 57:** Add success criterion for array format usage

#### Build Command

N/A (documentation - no build required)

**Expected**: Documentation accurately reflects new array structure

---

### FINAL STEP: Complete Validation ⏳ PENDING

**Status**: ⏳ PENDING
**Dependencies**: All previous steps complete

#### Validation Checklist

Run the following validation steps:

- [ ] Run `cargo build && cargo +nightly fmt`
- [ ] Run `cargo nextest run`
- [ ] Test Color enum type guide output (verify 10 `.0.alpha` entries)
- [ ] Verify no HashMap collisions
- [ ] Verify each variant has its own root_example
- [ ] Run integration tests from `.claude/integration_tests/type_guide.md`
- [ ] Test Python scripts with new JSON structure
- [ ] Test shell scripts with new JSON structure

#### Success Criteria

- All tests pass
- Color enum shows 10 separate entries for `.0.alpha` (one per variant)
- Each entry has variant-specific `root_example`
- No `applicable_variants` field present anywhere
- Scripts successfully process array structure

#### Expected Color Enum Output

```json
{
  "mutation_paths": [
    {
      "path": ".0.alpha",
      "description": "Mutate the 'alpha' field in Xyza variant",
      "root_example": {"Xyza": {"alpha": 1.0, "x": 0.8, "y": 0.7, "z": 0.6}}
    },
    {
      "path": ".0.alpha",
      "description": "Mutate the 'alpha' field in Hsla variant",
      "root_example": {"Hsla": {"alpha": 1.0, "hue": 180.0, "saturation": 0.5, "lightness": 0.5}}
    },
    // ... 8 more variants with `.0.alpha` path
  ]
}
```

---

## BENEFITS

### Code Simplification
- **Remove ~200 lines of grouping logic**: Eliminate `group_variants_by_signature()`, signature iteration, representative variant selection, and `applicable_variants` merging
- **Simpler knowledge system**: Replace indirect `EnumVariantSignature` with explicit `EnumVariant` type
- **Remove field**: Eliminate `applicable_variants` from `PathInfo` and `EnumPathData` structs
- **Straightforward processing**: Each variant processed independently - no coordination between variants needed

### Semantic Clarity
- **Explicit over implicit**: Each entry shows exactly one variant's `root_example` - no inference required
- **Self-contained entries**: Every entry has complete information (path + specific root_example)
- **Clear 1:1 mapping**: One entry = one variant = one root_example (no ambiguity)
- **Natural duplicate handling**: Vec inherently supports duplicate paths - embrace it rather than fight it

### Agent Experience
- **Better discoverability**: Agents see all variants explicitly listed with their exact examples
- **No mental model complexity**: Don't need to understand "applicable_variants" concept
- **Easy filtering**: `filter(path == ".0.alpha")` returns all variants with that path
- **Correct examples always**: Each `root_example` matches the variant it's for

### Technical Benefits
- **HashMap collision fix**: Vec allows duplicate path keys naturally
- **Simpler data flow**: No signature grouping = no grouping state to track
- **Easier maintenance**: Less complex code = fewer edge cases = fewer bugs
- **Consistent approach**: If we accept Vec with duplicates, removing grouping is the logical conclusion

---

## TESTING

### Test with `Color` Enum

The `Color` enum has 10 variants, each with different color space fields but all sharing an `alpha` field.

**Expected behavior after changes:**

1. **Verify duplicate paths preserved**
   - Array should contain 10 separate `.0.alpha` entries (one per variant)
   - Each `.0.alpha` entry has a different variant-specific `root_example`
   - Variants like `Hsla` should have entries for `.0.hue`, `.0.saturation`, `.0.lightness`
   - No `applicable_variants` field present in any entry

2. **Verify each entry is self-contained**
   - Each entry has exactly ONE variant's `root_example` (not grouped)
   - `root_example` for `.0.alpha` in `Xyza` variant shows `{"Xyza": {"alpha": 1.0, "x": ..., "y": ..., "z": ...}}`
   - `root_example` for `.0.alpha` in `Hsla` variant shows `{"Hsla": {"alpha": 1.0, "hue": ..., "saturation": ..., "lightness": ...}}`
   - Each `root_example` is specific to the variant it represents

3. **Test with `EnumVariant` knowledge**
   - Add knowledge entry for `Color::Srgba` with `TreatAsRootValue`
   - Verify `Srgba` variant produces ONLY root path (no field paths like `.0.red`, `.0.alpha`)
   - Verify other variants (without knowledge) still produce all field paths
   - Demonstrates per-variant knowledge override capability

4. **Verify no signature grouping artifacts**
   - No "representative variant" selection
   - No variants grouped together in same entry
   - Each variant independently discoverable

5. **Verify scripts work with new structure**
   - Update and run `mutation_test_prepare.py`
   - Update and run `create_mutation_test_json/read_comparison.py`
   - Update and run `create_mutation_test_json/compare.py`
   - Run integration tests from `.claude/integration_tests/type_guide.md`

---

## MIGRATION IMPACT

**Breaking changes**:
1. **Data structure**: `mutation_paths` changes from `HashMap<String, MutationPathExternal>` to `Vec<MutationPathExternal>`
2. **Field removal**: `applicable_variants` field removed from all output (no longer present)
3. **Semantic change**: Each entry now represents ONE variant (not grouped by signature)
4. **Knowledge type**: `EnumVariantSignature` knowledge entries must be migrated to `EnumVariant` format

**API consumers affected**:
- Any code accessing `mutation_paths` as a dict/object must change to array iteration
- Any code relying on `applicable_variants` field must be redesigned
- Any hardcoded `EnumVariantSignature` knowledge entries must be converted

**Internal impact**:
- **Rust code**: ~200 lines removed (enum grouping logic), ~50 lines modified (types, api, internals)
- **Python scripts**: 3 files need dict→array conversions (~30 lines modified)
- **Shell scripts**: 5 files need jq filter updates (~15 lines modified)
- **Documentation**: 8 files need structure examples updated
- **Tests**: 2 integration test files need assertions updated

**Timeline**:
- Core Rust implementation: 4-6 hours (enum_builder rewrite is complex)
- Consumer updates: 2-3 hours (mechanical changes, mostly straightforward)
- Testing and validation: 2-3 hours
- **Total estimate**: 8-12 hours

---

## DESIGN REVIEW SKIP NOTES

### DESIGN-1: Incomplete EnumVariant knowledge variant implementation - **Verdict**: REJECTED
- **Status**: REJECTED - Finding was incorrect
- **Location**: Step 4: Replace EnumVariantSignature with EnumVariant knowledge type
- **Issue**: Finding claimed the plan doesn't specify how to update knowledge lookup logic for the new EnumVariant key
- **Reasoning**: The finding misunderstood the architecture. The plan explicitly removes signature-based grouping (Step 1), making each variant process independently. The existing `find_knowledge()` method at recursion_context.rs:268-309 already handles knowledge lookups and will work with the new `EnumVariant` key structure without modification - it just matches on a different key structure (variant_name instead of signature+index). The plan's usage example demonstrates the complete picture. The suggested code location is for struct-field knowledge checks, not per-variant knowledge.
- **Decision**: The plan correctly focuses on the data structure change and knowledge key change. The lookup integration is implicit because it reuses existing infrastructure.
