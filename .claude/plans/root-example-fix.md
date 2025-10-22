# Root Example Collision Fix

## Problem

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

## Solution: Change HashMap to Array + Remove Signature Grouping

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

## Implementation Steps

### 1. Remove signature grouping from enum processing

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

**Remove:**
- `group_variants_by_signature()` function (lines ~197-222)
- Signature group iteration in `process_signature_groups()` (lines ~408-454)
- "Representative variant" selection logic (lines ~325-327)

**Refactor `process_signature_path()` → `process_variant_path()`:**
- **Rename function** to reflect single-variant processing (not signature groups)
- **Change parameter**: `applicable_variants: &[VariantName]` → `variant_name: &VariantName`
- **Remove lines 252-263**: Delete entire `applicable_variants` population loop for child paths
- **Simplify line 239**: Remove `.first()` conditional, directly push `variant_name` to `variant_chain`
- **New signature**:
  ```rust
  fn process_variant_path(
      path: PathKind,
      variant_name: &VariantName,      // Single variant, not array
      signature: &VariantSignature,     // Still needed for context
      ctx: &RecursionContext,
      child_examples: &mut HashMap<MutationPathDescriptor, Value>,
  ) -> Result<Vec<MutationPathInternal>, BuilderError>
  ```
- **Why keep this function**: Core recursion logic (context creation, variant chain management, child processing) remains valid for single-variant processing. Only signature-group-specific code needs removal.

**Replace with variant-by-variant processing:**
```rust
// OLD: Process by signature groups
for (variant_signature, variant_names) in variant_groups.sorted() {
    // Process all variants in group together
}

// NEW: Process each variant independently
for variant_name in all_variants.iter() {
    let variant_signature = get_signature_for_variant(variant_name);
    // Process this single variant
    // Each variant gets its own entries in mutation_paths Vec
}
```

**Function signature changes:**

Rename: `process_signature_groups()` → `process_all_variants()`

**Old signature** (line 401):
```rust
fn process_signature_groups(
    variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError>
```

**New signature**:
```rust
fn process_all_variants(
    variant_kinds: Vec<VariantKind>,  // Flat list with both name and signature
    ctx: &RecursionContext,
) -> std::result::Result<ProcessChildrenResult, BuilderError>
```

**Calling context change** (lines 98-103):

Old:
```rust
let variants_grouped_by_signature = group_variants_by_signature(ctx)?;
let (enum_examples, child_mutation_paths, partial_root_examples) =
    process_signature_groups(&variants_grouped_by_signature, ctx)?;
```

New:
```rust
let variant_kinds = extract_all_variants(ctx)?;
let (enum_examples, child_mutation_paths, partial_root_examples) =
    process_all_variants(variant_kinds, ctx)?;
```

**New helper function** (replaces `group_variants_by_signature`):

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

**Implementation inside `process_all_variants()`**:

```rust
for variant_kind in variant_kinds {
    let variant_name = variant_kind.name;
    let variant_signature = variant_kind.signature;

    // Create FRESH child_examples HashMap for each variant
    let mut child_examples = HashMap::new();

    // Process this single variant independently
    // Each variant gets its own ExampleGroup entry with single applicable_variant
}
```

**Key changes:**
- Each variant is processed independently
- No grouping by signature
- Each produces its own `MutationPathExternal` entries
- Each has its own variant-specific `root_example`

### 1b. Remove ExampleGroup and PathExample types

**Rationale**: With HashMap structure, one path key could represent multiple enum variants, requiring:
- `PathExample` enum wrapper to handle either single example OR array of grouped examples
- `ExampleGroup` struct to bundle multiple variants sharing same signature

With Vec structure, each variant gets its own entry with its own example. No grouping needed.

**Types to remove entirely:**

1. **`ExampleGroup` struct** (types.rs lines 199-211):
   ```rust
   pub struct ExampleGroup {
       pub applicable_variants: Vec<VariantName>,
       pub example: Option<Value>,
       pub signature: VariantSignature,
       pub mutability: Mutability,
   }
   ```

2. **`PathExample` enum** (types.rs lines 57-80):
   ```rust
   pub enum PathExample {
       Simple(Value),
       EnumRoot { groups: Vec<ExampleGroup> },
   }
   ```

3. **`PathExample` Serialize impl** (types.rs lines 82-112)

4. **`PathExample` Deserialize stub** (types.rs lines 114-127)

**Update `MutationPathExternal` structure:**

```rust
// OLD (lines 228-238):
pub struct MutationPathExternal {
    pub description: String,
    pub path_info: PathInfo,
    #[serde(flatten)]
    pub path_example: PathExample,  // ← Wrapper for Simple or EnumRoot
}

// NEW:
pub struct MutationPathExternal {
    pub path: MutationPath,
    pub description: String,
    #[serde(flatten)]
    pub path_info: PathInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,  // ← Direct field, no wrapper
}
```

**Update `MutationPathInternal` to store example:**

Add field to store example for transfer during conversion:
```rust
pub struct MutationPathInternal {
    pub mutation_path: MutationPath,
    pub description: String,
    pub example: Option<Value>,  // NEW: Store example here
    // ... other fields ...
}
```

**Function signature changes:**

1. **`ProcessChildrenResult` type alias** - simplify return type:
   ```rust
   // OLD:
   type ProcessChildrenResult = (
       Vec<ExampleGroup>,                      // ← Remove
       Vec<MutationPathInternal>,
       HashMap<Vec<VariantName>, Value>
   );

   // NEW:
   type ProcessChildrenResult = (
       Vec<MutationPathInternal>,              // Just paths with embedded examples
       HashMap<VariantName, Value>             // Per-variant (not per-signature-group)
   );
   ```

2. **`process_all_variants()` return type**:
   ```rust
   fn process_all_variants(
       variant_kinds: Vec<VariantKind>,
       ctx: &RecursionContext,
   ) -> Result<(Vec<MutationPathInternal>, HashMap<VariantName, Value>)>  // No ExampleGroup
   ```

3. **`build_partial_root_examples()` signature**:
   ```rust
   // OLD:
   fn build_partial_root_examples(
       variant_groups: &HashMap<VariantSignature, Vec<VariantName>>,
       examples: &[ExampleGroup],                // ← Remove parameter
       child_mutation_paths: &[MutationPathInternal],
       ctx: &RecursionContext,
   ) -> HashMap<Vec<VariantName>, Value>

   // NEW:
   fn build_partial_root_examples(
       variant_kinds: &[VariantKind],
       child_mutation_paths: &[MutationPathInternal],
       ctx: &RecursionContext,
   ) -> HashMap<VariantName, Value>  // Per-variant key
   ```

4. **Remove `select_preferred_example()` function entirely**:
   - Currently used at enum_path_builder.rs line 113
   - Purpose: Choose default example from ExampleGroups
   - Not needed: Each variant produces its own entries now

5. **Update `into_mutation_path_external()`** (mutation_path_internal.rs):
   ```rust
   // OLD:
   MutationPathExternal {
       description: self.description,
       path_info: PathInfo { ... },
       path_example: PathExample::Simple(self.example.unwrap_or(...)),
   }

   // NEW:
   MutationPathExternal {
       path: self.mutation_path,
       description: self.description,
       path_info: PathInfo { ... },
       example: self.example,  // Direct assignment
   }
   ```

**Logic changes in `process_all_variants()`:**

```rust
// Inside the variant loop:
for variant_kind in variant_kinds {
    let variant_name = variant_kind.name;
    let variant_signature = variant_kind.signature;

    // Build example for THIS variant
    let example = build_variant_example_value(
        &variant_signature,
        &variant_name,
        &child_examples,
        mutability,
        ctx,
    )?;

    // Store example in MutationPathInternal (not ExampleGroup)
    let internal_path = MutationPathInternal {
        mutation_path: MutationPath::root(),
        description: format!("Root example for {variant_name}"),
        example: Some(example),  // Direct storage
        // ... other fields ...
    };

    child_mutation_paths.push(internal_path);

    // NO MORE: examples.push(ExampleGroup { ... });
}
```

**Root path handling:**
- OLD: One root path ("") entry with `examples: Vec<ExampleGroup>`
- NEW: Multiple root path ("") entries, one per variant, each with its own `example`

**Example output:**
```json
[
  {"path": "", "example": {"Color::Srgba": [1.0, 0.0, 0.0, 1.0]}},
  {"path": "", "example": {"Color::Hsla": {"hue": 0.0, ...}}},
  {"path": ".0.alpha", "example": {"Color::Srgba": [1.0, 0.0, 0.0, 0.5]}},
  {"path": ".0.alpha", "example": {"Color::Hsla": {"hue": 0.0, ..., "alpha": 0.5}}}
]
```

### 2. Remove `applicable_variants` field from `PathInfo`

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    pub path_kind:           PathKind,
    pub type_name:           BrpTypeName,
    pub type_kind:           TypeKind,
    pub mutability:          Mutability,
    pub mutability_reason:   Option<Value>,
    pub enum_instructions:   Option<String>,
    // REMOVE: pub applicable_variants: Option<Vec<VariantName>>,  // ❌ No longer needed
    pub root_example:        Option<Value>,
}
```

Also remove from `EnumPathData`:
```rust
pub struct EnumPathData {
    pub variant_chain: Vec<VariantName>,
    // REMOVE: pub applicable_variants: Vec<VariantName>,  // ❌ No longer needed
    pub root_example: Option<Value>,
}
```

### 3. Replace `EnumVariantSignature` with `EnumVariant` knowledge type

**Location**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`

**Remove:**
```rust
EnumVariantSignature {
    enum_type: BrpTypeName,
    signature: VariantSignature,  // ❌ Too indirect
    index: usize,
}
```

**Add:**
```rust
EnumVariant {
    enum_type: BrpTypeName,
    variant_name: VariantName,  // ✅ Direct and explicit - MUST use VariantName newtype (NOT String)
}
```

**Type specification:**
- `enum_type`: `BrpTypeName` newtype
- `variant_name`: `VariantName` newtype (NOT `String`)
  - VariantName is a newtype wrapper providing type safety
  - Always use `VariantName::from("Color::Srgba")` pattern
  - Never use plain strings for variant names

**Usage pattern:**
```rust
TypeKnowledge::TreatAsRootValue {
    key: KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from("bevy_color::color::Color"),
        variant_name: VariantName::from("Color::Srgba".to_string()),
    },
    example: json!([1.0, 0.0, 0.0, 1.0]),
}
```

**Purpose**: Allows treating specific enum variants as atomic values (e.g., `Color::Srgba` as `[r, g, b, a]`) without recursing into field paths.

### 3b. Migrate existing Color variant knowledge entries

**Location**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs` lines 451-549

**Migration requirement**: All 10 Color variant entries MUST be migrated from `EnumVariantSignature` to `EnumVariant` format to preserve the existing "treat as opaque root values" behavior.

**Conversion pattern** (apply to all 10 Color variants):

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
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Srgba"),
    },
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);
```

**Deriving Variant Names**:

The variant names (e.g., `"Color::Srgba"`) are **NOT hardcoded** but dynamically derived during normal schema processing:

1. Each variant in the Color enum's `oneOf` array contains a `typePath` field (e.g., `"bevy_color::color::Color::Srgba"`)
2. The existing `variant_kind.rs::extract_variant_qualified_name()` function calls `type_parser::extract_simplified_variant_name()`
3. The parser splits the typePath at the last `::` to extract type and variant parts
4. The type part (`"bevy_color::color::Color"`) is simplified to `"Color"` (last segment)
5. The simplified type and variant are recombined: `"Color::Srgba"`
6. This is wrapped in `VariantName::from()`

**Complete list of Color variants** (as derived from schema):
- `"Color::Srgba"`
- `"Color::LinearRgba"`
- `"Color::Hsla"`
- `"Color::Hsva"`
- `"Color::Hwba"`
- `"Color::Laba"`
- `"Color::Lcha"`
- `"Color::Oklaba"`
- `"Color::Oklcha"`
- `"Color::Xyza"`

**Complete migration - all 10 Color variants:**

```rust
// 1. Srgba variant - array format [r, g, b, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Srgba"),
    },
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);

// 2. LinearRgba variant - array format [r, g, b, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::LinearRgba"),
    },
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_LINEAR_RGBA),
);

// 3. Hsla variant - array format [h, s, l, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Hsla"),
    },
    TypeKnowledge::as_root_value(json!([180.0, 0.5, 0.5, 1.0]), TYPE_BEVY_COLOR_HSLA),
);

// 4. Hsva variant - array format [h, s, v, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Hsva"),
    },
    TypeKnowledge::as_root_value(json!([240.0, 0.7, 0.9, 1.0]), TYPE_BEVY_COLOR_HSVA),
);

// 5. Hwba variant - array format [h, w, b, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Hwba"),
    },
    TypeKnowledge::as_root_value(json!([60.0, 0.2, 0.1, 1.0]), TYPE_BEVY_COLOR_HWBA),
);

// 6. Laba variant - array format [l, a, b, alpha]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Laba"),
    },
    TypeKnowledge::as_root_value(json!([0.5, 0.3, 0.2, 1.0]), TYPE_BEVY_COLOR_LABA),
);

// 7. Lcha variant - array format [l, c, h, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Lcha"),
    },
    TypeKnowledge::as_root_value(json!([0.6, 0.4, 90.0, 1.0]), TYPE_BEVY_COLOR_LCHA),
);

// 8. Oklaba variant - array format [l, a, b, alpha]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Oklaba"),
    },
    TypeKnowledge::as_root_value(json!([0.55, 0.15, 0.25, 1.0]), TYPE_BEVY_COLOR_OKLABA),
);

// 9. Oklcha variant - array format [l, c, h, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Oklcha"),
    },
    TypeKnowledge::as_root_value(json!([0.65, 0.35, 150.0, 1.0]), TYPE_BEVY_COLOR_OKLCHA),
);

// 10. Xyza variant - array format [x, y, z, a]
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Xyza"),
    },
    TypeKnowledge::as_root_value(json!([0.8, 0.7, 0.6, 1.0]), TYPE_BEVY_COLOR_XYZA),
);
```

**Migration checklist:**
- ✅ All 10 variants accounted for (Srgba, LinearRgba, Hsla, Hsva, Hwba, Laba, Lcha, Oklaba, Oklcha, Xyza)
- ✅ Each uses `KnowledgeKey::EnumVariant` (not `enum_variant_signature`)
- ✅ Each uses `VariantName::from()` with correct variant name
- ✅ All example JSON values preserved exactly
- ✅ All type constants preserved (TYPE_BEVY_COLOR_SRGBA, etc.)

**Implementation Note**: This same derivation works for the new `EnumVariant` entries - no additional logic needed. The migration simply changes the knowledge key format from `EnumVariantSignature` to `EnumVariant`, but the variant names are already being correctly derived from the schema. The constants like `TYPE_BEVY_COLOR_SRGBA` are the variant **data types** (used in `VariantSignature::Tuple`), not the source of variant names.

### 3c. Replace helper function for EnumVariant

**Location**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs` lines 119-131

**Remove old helper:**
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

**Add new helper:**
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

**Rationale**: Provides cleaner syntax for creating `EnumVariant` keys, reducing verbosity in the 10 Color variant migrations.

**Usage (simplified migration code):**
```rust
// Without helper (verbose):
map.insert(
    KnowledgeKey::EnumVariant {
        enum_type: BrpTypeName::from(TYPE_BEVY_COLOR),
        variant_name: VariantName::from("Color::Srgba"),
    },
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);

// With helper (cleaner):
map.insert(
    KnowledgeKey::enum_variant(TYPE_BEVY_COLOR, "Color::Srgba"),
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);
```

**Note**: The complete migration code above can be simplified using this helper if desired, though the explicit form is also acceptable.

### 4. Add `path` field to `MutationPathExternal`

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MutationPathExternal {
    /// The mutation path string (e.g., ".0.alpha", ".translation.x")
    ///
    /// Previously this was the HashMap key, now it's a field within the struct.
    /// Multiple entries can have the same path if they apply to different enum variants.
    ///
    /// Uses `MutationPath` newtype which serializes to String, matching the parameter
    /// name used in mutation tools.
    pub path: MutationPath,

    pub description: String,

    #[serde(flatten)]
    pub path_info: PathInfo,
}
```

### 5. Update `api.rs` to return Vec instead of HashMap

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs:47-58`

```rust
pub fn build_mutation_paths(
    type_path: &str,
    registry: &TypeRegistry,
) -> Result<Vec<MutationPathExternal>, BuilderError> {
    let internal_paths = PathBuilder::build_paths_from_type_path(type_path, registry)?;

    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            mutation_path_internal
                .clone()
                .into_mutation_path_external(registry)
        })
        .collect();

    Ok(external_paths)
}
```

**Change summary**:
- Return type: `HashMap<String, MutationPathExternal>` → `Vec<MutationPathExternal>`
- Remove tuple mapping with key (no longer needed - path becomes a field)
- Simple map and collect

### 6. Update `into_mutation_path_external` to populate path field

**Location**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

Find the `into_mutation_path_external` method and add the `path` field:

```rust
impl MutationPathInternal {
    pub fn into_mutation_path_external(self, registry: &TypeRegistry) -> MutationPathExternal {
        MutationPathExternal {
            path: self.mutation_path.clone(),  // NEW: add path field (MutationPath type)
            description: self.description,
            path_info: PathInfo {
                // ... existing fields unchanged (except remove applicable_variants) ...
            }
        }
    }
}
```

**Key points**:
- Use `self.mutation_path` directly (already exists on `MutationPathInternal`)
- No parameter addition needed - path comes from the existing field
- `path` field type is `MutationPath` (newtype that serializes to `String`)
- Also remove `applicable_variants` field population (field being removed)

### 7. Update consumers to work with array structure

**CRITICAL: Import change required first**

Before updating any Python scripts, add `Required` to the typing imports in `.claude/scripts/mutation_test_prepare.py`:

```python
# Line 23 - UPDATE FIRST:
from typing import Any, TypedDict, Required, cast  # Add Required import
```

**Rationale**: The `MutationPathData` TypedDict needs `Required[str]` for the new `path` field. Without this import, the script will fail immediately with a NameError when defining the TypedDict.

**Scripts to update**:

1. **`.claude/scripts/mutation_test_prepare.py`**
   - **Line 23**: Add `Required` to imports: `from typing import Any, TypedDict, Required, cast`
   - **Line 27-31**: Add `path: Required[str]` field to `MutationPathData` TypedDict
   - **Line 37**: Change type from `dict[str, MutationPathData]` to `list[MutationPathData]`
   - **Line 257**: Change default from `{}` to `[]`
   - **Line 337-340**: Change iteration from `mutation_paths.items()` to direct iteration over list
   - Access path via: `path_entry['path']` (direct access since it's Required)

2. **`.claude/scripts/mutation_test_process_results.py`**
   - Change: `for path, data in type_guide['mutation_paths'].items()`
   - To: `for path_entry in type_guide['mutation_paths']:`
   - Access path via: `path_entry['path']`

2. **`.claude/scripts/create_mutation_test_json_deep_comparison.py`**
   - Change: Dictionary access `mutation_paths[path_name]`
   - To: Array iteration with filter `[p for p in mutation_paths if p['path'] == path_name]`

3. **Integration tests**
   - Update any tests that expect `mutation_paths` to be a dict
   - Update to iterate array or filter by path field

4. **Tool response handlers**
   - `mcp/src/brp_tools/brp_type_guide/mod.rs` - update response serialization
   - Any code that builds the JSON response structure

## Files to Modify

### Core Implementation (Rust)

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`** ⭐ MAJOR CHANGES
   - Remove `group_variants_by_signature()` function (~25 lines)
   - Rewrite `process_signature_groups()` to process variants individually (~150 lines modified)
   - Remove signature group iteration logic
   - Remove "representative variant" selection
   - Refactor `process_signature_path()` → `process_variant_path()`:
     * Rename function to reflect single-variant processing
     * Change parameter: `applicable_variants: &[VariantName]` → `variant_name: &VariantName`
     * Remove lines 252-263: `applicable_variants` population loop
     * Simplify line 239: Remove `.first()` conditional
   - Each variant now processed independently with its own `root_example`

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`**
   - Add `path: MutationPath` field to `MutationPathExternal` struct (newtype that serializes to String)
   - Remove `applicable_variants: Option<Vec<VariantName>>` from `PathInfo` struct
   - Remove `applicable_variants: Vec<VariantName>` from `EnumPathData` struct

3. **`mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`**
   - Remove `EnumVariantSignature` variant from `KnowledgeKey` enum
   - Add `EnumVariant { enum_type: BrpTypeName, variant_name: VariantName }` variant
   - Migrate all 10 Color variant knowledge entries from `EnumVariantSignature` to `EnumVariant` format (lines 451-549) per Step 3b
   - Remove `KnowledgeKey::enum_variant_signature()` helper function

4. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs`** (lines 47-58)
   - Change return type: `HashMap<String, MutationPathExternal>` → `Vec<MutationPathExternal>`
   - Remove tuple mapping with key
   - Simplify to `.map()` and `.collect()`

5. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`**
   - Add `path: self.mutation_path.clone()` to `MutationPathExternal` construction in `into_mutation_path_external()`
   - Use existing `self.mutation_path` field (no parameter addition needed)
   - Remove `applicable_variants` population (field no longer exists)

6. **`mcp/src/brp_tools/brp_type_guide/guide.rs`**
   - Remove `HashMap` import (line 10)
   - Change `mutation_paths` field type: `HashMap<String, MutationPathExternal>` → `Vec<MutationPathExternal>` (line 47)
   - Update serde skip attribute: `HashMap::is_empty` → `Vec::is_empty` (line 46)
   - Update constructor calls: `HashMap::new()` → `Vec::new()` (lines 105, 121)
   - Change function parameters: `&HashMap<String, MutationPathExternal>` → `&[MutationPathExternal]` (lines 134, 158)
   - Update iteration: `.values()` → `.iter()` (line 138)

### Python Scripts (Dict → Array Access)

7. **`.claude/scripts/mutation_test_prepare.py`**
   - Line 23: Add `Required` import: `from typing import Any, TypedDict, Required, cast`
   - Line 27-31: Add `path: Required[str]` field to `MutationPathData` TypedDict (required field)
   - Line 37: Change type definition from `dict[str, MutationPathData]` to `list[MutationPathData]`
   - Line 257: Change `mutation_paths = type_data.get("mutation_paths") or {}` to `or []`
   - Line 337-340: Update iteration pattern:
     * OLD: `for path, path_info in mutation_paths.items():`
     * NEW: `for path_entry in mutation_paths:`
     * Access path with: `path = path_entry["path"]` (direct access, not .get())

8. **`.claude/scripts/create_mutation_test_json/read_comparison.py`**
   - Line 358: Keep `mutation_paths = type_data["mutation_paths"]` (now returns array)
   - Line 359-362: Replace dict membership check with array filter:
     ```python
     # OLD: if mutation_path not in mutation_paths: return None
     # OLD: return mutation_paths[mutation_path]
     # NEW:
     matching = [p for p in mutation_paths if p.get('path') == mutation_path]
     return matching[0] if matching else None
     ```

9. **`.claude/scripts/create_mutation_test_json/compare.py`**
   - Line 419: Update truthiness check (array is truthy if non-empty)
   - Line 422: Change `.get("mutation_paths", {})` to `.get("mutation_paths", [])`
   - Line 424: Change `isinstance(t.get("mutation_paths"), dict)` to `isinstance(t.get("mutation_paths"), list)`
   - Update `len()` call - works for both dict and list, but validate it's counting correctly

### Shell Scripts (Bash + Python/jq)

10. **`.claude/scripts/create_mutation_test_json/augment_response.sh`**
   - Lines 85-98: Complete rewrite of test_status logic for Vec structure:
     * Change empty check: `== {}` → `== []`
     * Change type check: `type == "object"` → `type == "array"`
     * Change root check: `has("")` → `[0].path == ""`
     * Change field access: `[""]` → `[0]`
   - Line 122: Update filter: `!= {}` → `!= []`
   - Line 126: Update path extraction: `// {} | keys | .[]` → `// [] | .[]`
     * Note: This counts path objects (not unique paths), so duplicates count multiple times

11. **`.claude/scripts/get_type_kind.sh`**
   - Lines 36-38, 73-74: Update Python iteration:
     ```python
     # OLD: for path, path_data in guide['mutation_paths'].items():
     # NEW: for path_entry in guide['mutation_paths']:
     #      path = path_entry['path']
     #      path_data = path_entry
     ```

12. **`.claude/scripts/get_mutation_path.sh`**
    - Lines 108-150: Update Python dict access patterns:
      ```python
      # Line 112: mutation_paths = type_data['mutation_paths']  # Now returns array
      # Line 121: for i, path in enumerate(list(mutation_paths.keys())[:20]):
      #   NEW: for i, entry in enumerate(mutation_paths[:20]):
      #        path = entry['path']

      # Line 134: if mutation_path not in mutation_paths:
      #   NEW: if not any(p['path'] == mutation_path for p in mutation_paths):

      # Line 138: matching = [p for p in mutation_paths.keys() if mutation_path in p]
      #   NEW: matching = [p['path'] for p in mutation_paths if mutation_path in p['path']]

      # Line 145: path_data = mutation_paths[mutation_path]
      #   NEW: path_data = next((p for p in mutation_paths if p['path'] == mutation_path), None)
      ```

13. **`.claude/scripts/get_mutation_path_list.sh`**
    - Lines 77-88: Update Python iteration:
      ```python
      # Line 81: mutation_paths = type_data['mutation_paths']  # Now returns array
      # Line 84: for path in mutation_paths.keys():
      #   NEW: for entry in mutation_paths:
      #        path = entry['path']
      ```

14. **`.claude/scripts/type_guide_test_extract.sh`**
    - Lines 45-51: Update jq extraction (works as-is, extracts whole array)
    - Lines 66-73: Update path validation from dict key check to array search:
      ```bash
      # OLD: jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | has($path)'
      # NEW: jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | any(.[]; .path == $path)'
      ```

### Documentation Files

15. **`.claude/commands/create_mutation_test_json.md`**
    - Line 50: Remove or fix misleading "array" reference
    - Lines 304-313: **CRITICAL** - Update `<MutationPathsExplanation/>` section:
      - Change dict access examples to array iteration examples
      - Update: `type_guide['TypeName']['mutation_paths']['.path']`
      - To: `next(p for p in type_guide['TypeName']['mutation_paths'] if p['path'] == '.path')`
    - Lines 159-161: Change "objects with path keys" to "array of objects with path field"
    - Lines 268-277: Update comparison format examples

16. **`.claude/commands/get_guide_current.md`**
    - Line 36: Update "Available paths" to reference array structure
    - Lines 34-37: Update filtering examples to use array search

17. **`.claude/commands/get_guide_baseline.md`**
    - Lines 51-54, 69: Update examples to show array structure
    - Line 102: Update JSON example from object to array

18. **`.claude/commands/get_kind_baseline.md`**
    - Line 50: Fix "array" reference to be accurate

19. **`.claude/commands/get_path_baseline.md`**
    - Lines 65-68: Update access examples

20. **`.claude/commands/compare_mutation_path.md`**
    - Lines 137-149: Update output section examples

### Integration Tests

21. **`.claude/integration_tests/type_guide.md`**
    - Line 66: Update extraction script call (script will handle array internally)
    - Line 67: Update comment about structure
    - Lines 81, 134, 197, 247: Update references to dict structure
    - Update all test assertions to expect array structure

22. **`.claude/integration_tests/data_operations.md`**
    - Lines 36-55: Update references to mutation paths discovery to reflect array structure

### Data Files (Auto-regenerated - No Manual Changes)

The following files will be automatically regenerated with the new structure:
- `.claude/transient/all_types.json`
- `.claude/transient/all_types_baseline.json`
- `.claude/transient/all_types_stats.json`
- `.claude/transient/all_types_good_*.json`
- `.claude/transient/all_types_review_failures_*.json`

## Detailed Changes by File

### Rust Implementation (4 files)

#### 1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` (Lines 228-238)

**Current:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    pub description:  String,
    pub path_info:    PathInfo,
    #[serde(flatten)]
    pub path_example: PathExample,
}
```

**New:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPathExternal {
    pub path:         MutationPath,
    pub description:  String,
    pub path_info:    PathInfo,
    #[serde(flatten)]
    pub path_example: PathExample,
}
```

#### 2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/api.rs` (Lines 28-61)

**Current:**
```rust
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<HashMap<String, MutationPathExternal>> {
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            let key = (*mutation_path_internal.mutation_path).clone();
            let mutation_path = mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry);
            (key, mutation_path)
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
                .into_mutation_path_external(&registry)
        })
        .collect();
    Ok(external_paths)
}
```

Also update `extract_spawn_format` (Lines 63-76):

**Current:**
```rust
pub fn extract_spawn_format(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value> {
    mutation_paths.get("")
}
```

**New:**
```rust
pub fn extract_spawn_format(
    mutation_paths: &[MutationPathExternal],
) -> Option<Value> {
    mutation_paths.iter().find(|path| path.path.is_empty())
}
```

#### 3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` (Lines 76-110)

**Current:**
```rust
pub fn into_mutation_path_external(
    mut self,
    registry: &HashMap<BrpTypeName, Value>,
) -> MutationPathExternal {
    MutationPathExternal {
        description,
        path_info: PathInfo { ... },
        path_example,
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
        path: self.mutation_path.clone(),
        description,
        path_info: PathInfo { ... },
        path_example,
    }
}
```

#### 4. `mcp/src/brp_tools/brp_type_guide/guide.rs` (Multiple locations)

**Line 10: Import change**
```rust
// OLD: use std::collections::HashMap;
// NEW: (remove this import - no longer needed)
```

**Line 46-47: Field declaration**
```rust
// OLD:
#[serde(skip_serializing_if = "HashMap::is_empty")]
pub mutation_paths: HashMap<String, MutationPathExternal>,

// NEW:
#[serde(skip_serializing_if = "Vec::is_empty")]
pub mutation_paths: Vec<MutationPathExternal>,
```

**Line 105, 121: Constructor calls in error builders**
```rust
// OLD: mutation_paths: HashMap::new(),
mutation_paths: Vec::new(),
```

**Line 134: Function parameter type**
```rust
// OLD:
fn generate_agent_guidance(
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Result<String>

// NEW:
fn generate_agent_guidance(
    mutation_paths: &[MutationPathExternal],
) -> Result<String>
```

**Line 137-139: Iteration pattern**
```rust
// OLD:
let has_entity = mutation_paths
    .values()
    .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));

// NEW:
let has_entity = mutation_paths
    .iter()
    .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));
```

**Line 158: Function parameter type**
```rust
// OLD:
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &HashMap<String, MutationPathExternal>,
) -> Option<Value>

// NEW:
fn extract_spawn_format_if_spawnable(
    registry_schema: &Value,
    mutation_paths: &[MutationPathExternal],
) -> Option<Value>
```

### Python Scripts (3 files)

#### 5. `.claude/scripts/mutation_test_prepare.py`

**Line 23: Add Required import for TypedDict:**
```python
# OLD: from typing import Any, TypedDict, cast
from typing import Any, TypedDict, Required, cast
```

**Lines 27-31 (Add path field as Required):**
```python
class MutationPathData(TypedDict, total=False):
    path: Required[str]  # NEW: Required field (every entry must have path)
    description: str
    example: Any  # pyright: ignore[reportExplicitAny] - arbitrary JSON value
    path_info: dict[str, str]
```

**Explanation**: `path` is marked as `Required[str]` because every mutation path entry MUST have a path field (it's required in the Rust `MutationPathExternal` struct). Other fields remain optional via `total=False`.

**Line 37:**
```python
# OLD: mutation_paths: dict[str, MutationPathData] | None
mutation_paths: list[MutationPathData] | None
```

**Line 257:**
```python
# OLD: mutation_paths = type_data.get("mutation_paths") or {}
mutation_paths = type_data.get("mutation_paths") or []
```

**Line 337-340: Update iteration pattern**
```python
# OLD:
for path, path_info in mutation_paths.items():
    # Skip non-mutable paths
    if path_info.get("path_info", {}).get("mutability") == "not_mutable":

# NEW:
for path_entry in mutation_paths:
    path = path_entry["path"]  # Access required field directly
    # Skip non-mutable paths
    if path_entry.get("path_info", {}).get("mutability") == "not_mutable":
```

**Note**: Since `path` is marked as `Required`, we access it directly with `path_entry["path"]` rather than `.get("path", "")`. This will cause type checker errors if the field is missing, which is the desired behavior.

#### 6. `.claude/scripts/create_mutation_test_json/read_comparison.py` (Lines 355-362)

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

#### 7. `.claude/scripts/create_mutation_test_json/compare.py` (Lines 419-427)

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

### Shell Scripts (5 files)

#### 8. `.claude/scripts/create_mutation_test_json/augment_response.sh`

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

**Key changes**:
- Empty check: `== {}` → `== []`
- Type check: `type == "object"` → `type == "array"`
- Root path check: `has("")` → `[0].path == ""`
- Field access: `[""]` → `[0]` (assumes root path is first when length is 1)

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

**Explanation**: The old logic used `keys` to get the path strings from the HashMap keys. The new logic iterates the array directly (each element is already a full path object). Note that this counts path **objects**, not unique path **strings** - with the new structure, duplicate path strings (e.g., `.0.alpha` for multiple Color variants) will be counted multiple times. This is the intended behavior showing total entries.

#### 9. `.claude/scripts/get_type_kind.sh` (Lines 36-38, 73-74)

**Current:**
```python
for path, path_data in guide['mutation_paths'].items():
```

**New:**
```python
for path_data in guide['mutation_paths']:
```

#### 10. `.claude/scripts/get_mutation_path.sh` (Lines 108-150)

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

#### 11. `.claude/scripts/get_mutation_path_list.sh` (Lines 77-88)

**Current:**
```python
for path in mutation_paths.keys():
```

**New:**
```python
for path_obj in mutation_paths:
    path = path_obj['path']
```

#### 12. `.claude/scripts/type_guide_test_extract.sh` (Lines 66-73)

**Current:**
```bash
jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | has($path)'
```

**New:**
```bash
jq --arg path "$FIELD_PATH" '.type_guide[$type].mutation_paths | any(.path == $path)'
```

### Documentation Files (6 files)

#### 13. `.claude/commands/create_mutation_test_json.md`

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

#### 14. `.claude/commands/get_guide_current.md` (Lines 102-105)

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

#### 15. `.claude/commands/get_guide_baseline.md` (Lines 84-87)

Same change as above - show array structure with path fields.

#### 16. `.claude/commands/get_kind_baseline.md` (Line 50)

**Current:**
```markdown
The baseline file must have the expected structure with `type_guide` array containing types with `mutation_paths`
```

**New:**
```markdown
The baseline file must have the expected structure with `type_guide` containing types with `mutation_paths` arrays
```

#### 17. `.claude/commands/get_path_baseline.md` (Lines 94-104)

Update output format to show flattened structure with path as a field.

#### 18. `.claude/commands/compare_mutation_path.md` (Lines 141-148)

Update terminology from "dict values" to "array elements" and "path objects".

### Integration Tests (2 files)

#### 19. `.claude/integration_tests/type_guide.md`

**Lines 66-67:** Add note about array format
**Lines 81-82:** Explicitly state Vec3/Quat use array format
**Lines 186-187:** Add note that object format intentionally fails
**Lines 196-197:** Emphasize array format in error validation
**Lines 238-251:** Add explicit wrong format notes

#### 20. `.claude/integration_tests/data_operations.md`

**Lines 13-14:** Add critical note about array format requirement
**Lines 37, 40, 45:** Update to reflect type_guide returns array
**Line 57:** Add success criterion for array format usage

## Testing

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

## Benefits

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

## Migration Impact

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
- **Python scripts**: 6 files need dict→array conversions (~30 lines modified)
- **Shell scripts**: 5 files need jq filter updates (~15 lines modified)
- **Documentation**: 8 files need structure examples updated
- **Tests**: 2 integration test files need assertions updated

**Timeline**:
- Core Rust implementation: 4-6 hours (enum_builder rewrite is complex)
- Consumer updates: 2-3 hours (mechanical changes, mostly straightforward)
- Testing and validation: 2-3 hours
- **Total estimate**: 8-12 hours

## Design Review Skip Notes

## DESIGN-1: Incomplete EnumVariant knowledge variant implementation - **Verdict**: REJECTED
- **Status**: REJECTED - Finding was incorrect
- **Location**: Step 3: Replace EnumVariantSignature with EnumVariant knowledge type
- **Issue**: Finding claimed the plan doesn't specify how to update knowledge lookup logic for the new EnumVariant key
- **Reasoning**: The finding misunderstood the architecture. The plan explicitly removes signature-based grouping (Step 1), making each variant process independently. The existing `find_knowledge()` method at recursion_context.rs:268-309 already handles knowledge lookups and will work with the new `EnumVariant` key structure without modification - it just matches on a different key structure (variant_name instead of signature+index). The plan's usage example (lines 201-210) demonstrates the complete picture. The suggested code location (enum_path_builder.rs:108-109) is for struct-field knowledge checks, not per-variant knowledge.
- **Decision**: The plan correctly focuses on the data structure change and knowledge key change. The lookup integration is implicit because it reuses existing infrastructure.
