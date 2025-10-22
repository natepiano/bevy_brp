# Color Knowledge Enhancement: Show Original Field Structure

## Goal

Add `hard_coded` field to type guide output showing the original field-based structure alongside the hard-coded array format, so agents can see field names are available.

## Architecture Decision: New TypeKnowledge Variant

**Use Option B: Add new variant rather than enhancing existing one**

### Current TypeKnowledge
```rust
pub enum TypeKnowledge {
    TeachAndRecurse { example: Value },
    TreatAsRootValue { example: Value, simplified_type: String },
}
```

### Enhanced TypeKnowledge
```rust
pub enum TypeKnowledge {
    TeachAndRecurse { example: Value },
    TreatAsRootValue { example: Value, simplified_type: String },
    TreatAsRootValueWithOriginal {  // NEW VARIANT
        example: Value,
        simplified_type: String,
        original_value: Value,      // Required: field-based structure
        note: String,               // Required: explanation
    },
}
```

**Why new variant vs enhancing TreatAsRootValue:**
- Type safety: original_value and note are required, not Optional
- Clarity: Match expressions explicitly show which case provides original
- No Option pollution: No `if original_value.is_some()` checks needed
- Semantics: Name clearly indicates enhanced behavior

## Propagation Path

**Very direct - single function call:**

```
1. TypeKnowledge stored in BRP_TYPE_KNOWLEDGE HashMap
2. check_knowledge() looks up and matches on TypeKnowledge variant
3. For TreatAsRootValueWithOriginal: extract original_value + note
4. Create HardCodedInfo struct
5. Pass to build_mutation_path_internal() as new parameter
6. build_mutation_path_internal() includes it in PathInfo
```

## Implementation Steps

### STEP 1: Add HardCodedInfo Struct
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

```rust
/// Information about hard-coded value replacement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardCodedInfo {
    /// Original field-based structure before replacement
    pub original_value: Value,
    /// Explanation of why value was replaced
    pub note: String,
}
```

### STEP 2: Add hard_coded Field to PathInfo
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Add to PathInfo struct:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub hard_coded: Option<HardCodedInfo>,
```

### STEP 3: Add TreatAsRootValueWithOriginal Variant
**File**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`

Add to TypeKnowledge enum:
```rust
TreatAsRootValueWithOriginal {
    example: Value,
    simplified_type: String,
    original_value: Value,
    note: String,
},
```

### STEP 4: Update TypeKnowledge Methods
**File**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`

Update `example()` method:
```rust
pub const fn example(&self) -> &Value {
    match self {
        Self::TeachAndRecurse { example }
        | Self::TreatAsRootValue { example, .. }
        | Self::TreatAsRootValueWithOriginal { example, .. } => example,
    }
}
```

Update `get_simplified_name()` method:
```rust
if let Some(Self::TreatAsRootValue { simplified_type, .. }
         | Self::TreatAsRootValueWithOriginal { simplified_type, .. })
    = BRP_TYPE_KNOWLEDGE.get(&knowledge_key)
{
    Some(BrpTypeName::from(simplified_type.clone()))
}
```

### STEP 5: Add New Constructor
**File**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`

```rust
impl TypeKnowledge {
    // Existing constructors unchanged
    pub const fn new(example: Value) -> Self { ... }
    pub fn as_root_value(example: Value, simplified_type: impl Into<String>) -> Self { ... }

    // NEW: For types showing both formats
    pub fn as_root_value_with_original(
        example: Value,
        original_value: Value,
        simplified_type: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self::TreatAsRootValueWithOriginal {
            example,
            simplified_type: simplified_type.into(),
            original_value,
            note: note.into(),
        }
    }
}
```

### STEP 6: Update All 10 Color Variant Knowledge Entries
**File**: `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs`

Replace each color variant entry. Example for Srgba:

```rust
// OLD:
map.insert(
    KnowledgeKey::enum_variant_signature(
        TYPE_BEVY_COLOR,
        VariantSignature::Tuple(vec![BrpTypeName::from(TYPE_BEVY_COLOR_SRGBA)]),
        0,
    ),
    TypeKnowledge::as_root_value(json!([1.0, 0.0, 0.0, 1.0]), TYPE_BEVY_COLOR_SRGBA),
);

// NEW:
map.insert(
    KnowledgeKey::enum_variant_signature(
        TYPE_BEVY_COLOR,
        VariantSignature::Tuple(vec![BrpTypeName::from(TYPE_BEVY_COLOR_SRGBA)]),
        0,
    ),
    TypeKnowledge::as_root_value_with_original(
        json!([1.0, 0.0, 0.0, 1.0]),  // Hard-coded array
        json!({"red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0}),  // Original structure
        TYPE_BEVY_COLOR_SRGBA,
        "Example replaced with hard-coded value to reduce duplicate mutation path examples"
    ),
);
```

**Field mappings for all 10 variants:**
- **Srgba**: `{"red", "green", "blue", "alpha"}`
- **LinearRgba**: `{"red", "green", "blue", "alpha"}`
- **Hsla**: `{"hue", "saturation", "lightness", "alpha"}`
- **Hsva**: `{"hue", "saturation", "value", "alpha"}`
- **Hwba**: `{"hue", "whiteness", "blackness", "alpha"}`
- **Laba**: `{"lightness", "a", "b", "alpha"}`
- **Lcha**: `{"lightness", "chroma", "hue", "alpha"}`
- **Oklaba**: `{"lightness", "a", "b", "alpha"}`
- **Oklcha**: `{"lightness", "chroma", "hue", "alpha"}`
- **Xyza**: `{"x", "y", "z", "alpha"}`

### STEP 7: Add Import for HardCodedInfo
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Add import near line 52 with other type imports:
```rust
use super::types::HardCodedInfo;
```

### STEP 8: Update matches! Macro
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Update line 591 to include new variant:
```rust
// OLD:
if matches!(knowledge, TypeKnowledge::TreatAsRootValue { .. }) {

// NEW:
if matches!(knowledge, TypeKnowledge::TreatAsRootValue { .. }
                      | TypeKnowledge::TreatAsRootValueWithOriginal { .. }) {
```

### STEP 9: Update check_knowledge() to Extract HardCodedInfo
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Replace the existing `if matches!` pattern with a nested match to handle all three TypeKnowledge variants explicitly:

```rust
fn check_knowledge(
    ctx: &RecursionContext,
) -> (
    Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>>,
    Option<Value>,
) {
    let knowledge_result = ctx.find_knowledge();
    match knowledge_result {
        Ok(Some(knowledge)) => {
            let example = knowledge.example().clone();

            // Handle all three TypeKnowledge variants explicitly
            match knowledge {
                TypeKnowledge::TreatAsRootValue { .. } => {
                    // Simple case: no original to show
                    return (
                        Some(Ok(vec![Self::build_mutation_path_internal(
                            ctx,
                            PathExample::Simple(example),
                            Mutability::Mutable,
                            None,
                            None,  // No hard_coded info
                        )])),
                        None,
                    );
                }
                TypeKnowledge::TreatAsRootValueWithOriginal {
                    original_value,
                    note,
                    ..
                } => {
                    // Enhanced case: show original structure
                    let hard_coded = Some(HardCodedInfo {
                        original_value: original_value.clone(),
                        note: note.clone(),
                    });

                    return (
                        Some(Ok(vec![Self::build_mutation_path_internal(
                            ctx,
                            PathExample::Simple(example),
                            Mutability::Mutable,
                            None,
                            hard_coded,  // Include hard_coded info
                        )])),
                        None,
                    );
                }
                TypeKnowledge::TeachAndRecurse { .. } => {
                    // CRITICAL: Preserve existing behavior from line 604
                    // Return None for early return (continue processing),
                    // Some(example) to provide example for mutation path assembly
                    // This allows recursion to continue while using the provided example
                    (None, Some(example))
                }
            }
        }
        Ok(None) => {
            // Continue with normal processing, no hard coded mutation knowledge found
            (None, None)
        }
        Err(e) => {
            // Propagate error from find_knowledge()
            (Some(Err(e)), None)
        }
    }
}
```

**Key changes from current code:**
1. Replace `if matches!` pattern (line 591) with nested `match knowledge`
2. Explicitly handle all three variants instead of fall-through behavior
3. TeachAndRecurse returns `(None, Some(example))` - same as current line 604
4. Both TreatAsRootValue variants return early with mutation paths
5. Outer match structure preserved for Ok(None) and Err cases

### STEP 10: Add hard_coded Parameter to build_mutation_path_internal()
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Update signature:
```rust
fn build_mutation_path_internal(
    ctx: &RecursionContext,
    path_example: PathExample,
    mutability: Mutability,
    mutability_reason: Option<Value>,
    hard_coded: Option<HardCodedInfo>,  // NEW PARAMETER
) -> MutationPath
```

Update PathInfo construction:
```rust
let path_info = PathInfo {
    path_kind,
    type_name: ctx.type_name(),
    type_kind: ctx.type_kind(),
    mutability,
    mutability_reason,
    enum_instructions,
    applicable_variants,
    root_example,
    hard_coded,  // NEW FIELD
};
```

### STEP 11: Update All Call Sites of build_mutation_path_internal()
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Update exactly **4 call sites** to add `None` for the new `hard_coded` parameter:

1. **Line ~522**: `build_final_result()` - PathAction::Create branch
2. **Line ~536**: `build_final_result()` - PathAction::Skip branch
3. **Line ~555**: `build_not_mutable_path()`
4. **Line ~593**: `check_knowledge()` - TreatAsRootValue branch (this one may pass Some(HardCodedInfo) - see STEP 9)

Note: Line numbers are approximate; search for `build_mutation_path_internal(` to find exact locations.

**Only the calls from check_knowledge() (STEP 9) pass Some(HardCodedInfo) for the new variant.** All other calls pass `None`.

## Expected Output

After implementation, color type guide will show:

```json
{
  "": {
    "description": "Replace the entire Color enum",
    "examples": [
      {
        "applicable_variants": ["Color::Srgba"],
        "example": {"Srgba": [1.0, 0.0, 0.0, 1.0]},
        "mutability": "mutable",
        "signature": {"Tuple": ["bevy_color::srgba::Srgba"]}
      }
    ],
    "path_info": {
      "path_kind": "RootValue",
      "type": "bevy_color::color::Color",
      "type_kind": "Enum",
      "mutability": "mutable"
    }
  },
  ".0": {
    "description": "Mutate element 0 of Color struct",
    "example": [1.0, 0.0, 0.0, 1.0],
    "path_info": {
      "applicable_variants": ["Color::Srgba"],
      "enum_instructions": "First, set the root mutation path to 'root_example'...",
      "path_kind": "IndexedElement",
      "type": "bevy_color::srgba::Srgba",
      "type_kind": "Struct",
      "mutability": "mutable",
      "root_example": {"Srgba": [1.0, 0.0, 0.0, 1.0]},
      "hard_coded": {
        "original_value": {"red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0},
        "note": "Example replaced with hard-coded value to reduce duplicate mutation path examples"
      }
    }
  }
}
```

## Benefits

✅ Agents see field names are available for object-based mutations
✅ Clear explanation of why compact array format is preferred
✅ No recursion penalty (both values pre-defined in knowledge)
✅ Type-safe (required fields, not Optional)
✅ Backward compatible (existing TreatAsRootValue unchanged)
✅ Only applied to Color variants that benefit from it

## Files Modified

1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Add HardCodedInfo struct, add hard_coded field to PathInfo
2. `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs` - Add TreatAsRootValueWithOriginal variant, add constructor, update example() and get_simplified_name() methods, update all 10 color variant entries
3. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` - Add import for HardCodedInfo, update matches! macro, update check_knowledge(), update build_mutation_path_internal() signature, update 4 call sites

## Implementation Notes

- **Clone Cost**: The original_value and note are cloned from static TypeKnowledge when building paths. This is acceptable because:
  - Only 10 color variants use this feature
  - Values are small (JSON objects with 4 fields each)
  - Static storage means one-time memory cost in HashMap
- **Match Completeness**: All pattern matches on TypeKnowledge variants have been identified and updated
- **Ordering**: hard_coded field appears only on variant element paths (`.0`), not on root Color enum path, because root uses PathExample::EnumRoot which follows a different code path

## Testing

After implementation:
1. Run `cargo build` - verify no compilation errors
2. Run `cargo install --path mcp`
3. Reconnect MCP: `/mcp reconnect brp`
4. Run type guide on Color: `mcp__brp__brp_type_guide(["bevy_color::color::Color"])`
5. **Verify `.0` path has `hard_coded` field** with:
   - `original_value` showing object format with field names
   - `note` explaining the replacement
6. **Test all 10 color variants** to ensure each shows correct field names:
   - Srgba: red, green, blue, alpha
   - LinearRgba: red, green, blue, alpha
   - Hsla: hue, saturation, lightness, alpha
   - Hsva: hue, saturation, value, alpha
   - Hwba: hue, whiteness, blackness, alpha
   - Laba: lightness, a, b, alpha
   - Lcha: lightness, chroma, hue, alpha
   - Oklaba: lightness, a, b, alpha
   - Oklcha: lightness, chroma, hue, alpha
   - Xyza: x, y, z, alpha
7. **Verify backward compatibility**: Check a non-color type with TreatAsRootValue (e.g., Entity) has NO `hard_coded` field
8. Run mutation test on a type containing BackgroundColor to ensure mutations still work
