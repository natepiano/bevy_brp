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

### STEP 1: Add HardCodedInfo and KnowledgeResult Structs
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Add HardCodedInfo struct:
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

Add KnowledgeResult struct:
```rust
/// Result from TypeKnowledge lookup - provides example, hard_coded info, and recursion control
#[derive(Debug, Clone)]
pub struct KnowledgeResult {
    /// Example value to use
    pub example: Value,
    /// Hard-coded information if this knowledge has it
    pub hard_coded: Option<HardCodedInfo>,
    /// Whether to stop recursion (TreatAsRootValue*) or continue (TeachAndRecurse)
    pub stop_recursion: bool,
}
```

This struct provides a clean, documented API for both path_builder and enum_builder to consume TypeKnowledge, replacing the cryptic tuple pattern in check_knowledge().

### STEP 2: Add hard_coded Field to PathInfo
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

Add to PathInfo struct at the end (after `root_example` field):
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub hard_coded: Option<HardCodedInfo>,
```

### STEP 2a: Add hard_coded Field to MutationPathInternal
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

Add to MutationPathInternal struct:
```rust
pub hard_coded: Option<HardCodedInfo>,
```

This field stores the hard_coded info during path building before it's transferred to PathInfo in the final external representation.

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

### STEP 6a: Update find_knowledge() to Return KnowledgeResult
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

Change the return type and extract all data from TypeKnowledge:

```rust
pub fn find_knowledge(
    &self,
) -> std::result::Result<Option<KnowledgeResult>, BuilderError> {
    // All existing lookup logic stays the same (struct fields, enum variants, exact types)
    // ...existing code that finds knowledge...

    // At the end, instead of returning &'static TypeKnowledge, extract the data:
    Ok(knowledge_ref.map(|knowledge| {
        let example = knowledge.example().clone();
        let (hard_coded, stop_recursion) = match knowledge {
            TypeKnowledge::TreatAsRootValue { .. } => (None, true),
            TypeKnowledge::TreatAsRootValueWithOriginal { original_value, note, .. } => {
                (Some(HardCodedInfo {
                    original_value: original_value.clone(),
                    note: note.clone(),
                }), true)
            }
            TypeKnowledge::TeachAndRecurse { .. } => (None, false),
        };

        KnowledgeResult {
            example,
            hard_coded,
            stop_recursion,
        }
    }))
}
```

**Why this change:**
- Single source of truth for extracting data from TypeKnowledge
- Both path_builder and enum_builder get the same data from the same method
- Returns owned data (no `&'static` lifetime leak)
- Clearly documents the three possible behaviors via `stop_recursion` flag

### STEP 6b: Add Shared Helper to support.rs
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs`

Add function to create MutationPathInternal from KnowledgeResult:

```rust
use super::types::{HardCodedInfo, KnowledgeResult};

/// Create MutationPathInternal from TypeKnowledge result
///
/// Provides single canonical way to create paths from knowledge, ensuring
/// both path_builder and enum_builder handle knowledge consistently.
///
/// Used by:
/// - path_builder's check_knowledge() for TreatAsRootValue types
/// - enum_builder's build_enum_root_path() when knowledge is found
pub fn create_path_from_knowledge(
    ctx: &RecursionContext,
    knowledge: KnowledgeResult,
    mutability: Mutability,
    mutability_reason: Option<Value>,
) -> MutationPathInternal {
    MutationPathInternal {
        mutation_path: ctx.mutation_path.clone(),
        example: PathExample::Simple(knowledge.example),
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutability,
        mutability_reason,
        enum_path_data: None,
        depth: *ctx.depth,
        partial_root_examples: None,
        hard_coded: knowledge.hard_coded,
    }
}
```

**Import additions needed:**
```rust
use super::types::PathExample;
```

### STEP 7: Add Imports
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Add imports near line 52 with other type imports:
```rust
use super::support;
use super::types::{HardCodedInfo, KnowledgeResult};
```

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

Add import with other type imports:
```rust
use super::types::HardCodedInfo;
```

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

Add import with other type imports:
```rust
use super::types::{HardCodedInfo, KnowledgeResult};
```

### STEP 8: Update check_knowledge() to Use KnowledgeResult
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Simplify check_knowledge() to use KnowledgeResult and shared helper:

```rust
fn check_knowledge(
    ctx: &RecursionContext,
) -> (
    Option<std::result::Result<Vec<MutationPathInternal>, BuilderError>>,
    Option<Value>,
) {
    match ctx.find_knowledge() {
        Ok(Some(knowledge_result)) => {
            // Check if we should stop recursion (TreatAsRootValue variants)
            if knowledge_result.stop_recursion {
                // Use shared helper to create MutationPathInternal
                let path = support::create_path_from_knowledge(
                    ctx,
                    knowledge_result,
                    Mutability::Mutable,
                    None,  // mutability_reason
                );
                return (Some(Ok(vec![path])), None);
            }

            // TeachAndRecurse: continue processing, provide example for assembly
            (None, Some(knowledge_result.example))
        }
        Ok(None) => {
            // No knowledge found - continue with normal processing
            (None, None)
        }
        Err(e) => {
            // Propagate error from find_knowledge()
            (Some(Err(e)), None)
        }
    }
}
```

**Key improvements:**
1. No more cryptic tuple handling - KnowledgeResult makes intent clear
2. Uses `stop_recursion` flag instead of matching on TypeKnowledge variants
3. Uses shared `support::create_path_from_knowledge()` helper - single creation point
4. Simpler control flow - if stop, use helper and return; else continue with example
5. Hard-coded info automatically included via the helper

### STEP 9: Update enum_builder to Use KnowledgeResult and Shared Helper
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs`

Update the knowledge lookup and path creation to use the shared pattern:

**Current code (lines 105-119):**
```rust
let default_example = ctx
    .find_knowledge()
    .ok()
    .flatten()
    .map(|knowledge| knowledge.example().clone())
    .or_else(|| select_preferred_example(&enum_examples))
    .ok_or_else(...)?;
```

**New code:**
```rust
// Check for knowledge first
let knowledge_result = ctx.find_knowledge().ok().flatten();

// If we have TreatAsRootValue knowledge, stop processing and return just the root path
if let Some(ref kr) = knowledge_result {
    if kr.stop_recursion {
        // Use shared helper to create path from knowledge
        let root_path = support::create_path_from_knowledge(
            ctx,
            kr.clone(),
            Mutability::Mutable,
            None,
        );
        // Return only the root path - no variant processing needed
        return Ok(vec![root_path]);
    }
}

// Otherwise, get example (from knowledge or variant selection)
let default_example = knowledge_result
    .as_ref()
    .map(|kr| kr.example.clone())
    .or_else(|| select_preferred_example(&enum_examples))
    .ok_or_else(|| {
        BuilderError::SystemError(Report::new(Error::InvalidState(format!(
            "Enum {} has no valid example: no struct field knowledge and no mutable variants",
            ctx.type_name()
        ))))
    })?;

// Extract hard_coded if knowledge provided it (for enum root path)
let hard_coded = knowledge_result.and_then(|kr| kr.hard_coded);
```

**Update build_enum_root_path signature and call:**
```rust
// Pass hard_coded to build_enum_root_path
let mut root_mutation_path = build_enum_root_path(
    ctx,
    enum_examples,
    default_example,
    enum_mutability,
    mutability_reason,
    hard_coded,  // NEW PARAMETER
);
```

**Update build_enum_root_path function (around line 680):**
```rust
fn build_enum_root_path(
    ctx: &RecursionContext,
    enum_examples: Vec<ExampleGroup>,
    default_example: Value,
    enum_mutability: Mutability,
    mutability_reason: Option<Value>,
    hard_coded: Option<HardCodedInfo>,  // NEW PARAMETER
) -> MutationPathInternal {
    // ... existing code ...

    MutationPathInternal {
        mutation_path: ctx.mutation_path.clone(),
        example: PathExample::EnumRoot {
            groups: enum_examples,
            for_parent: default_example,
        },
        type_name: ctx.type_name().display_name(),
        path_kind: ctx.path_kind.clone(),
        mutability: enum_mutability,
        mutability_reason,
        enum_path_data,
        depth: *ctx.depth,
        partial_root_examples: None,
        hard_coded,  // NEW FIELD
    }
}
```

**Add import to enum_path_builder.rs:**
```rust
use super::super::support;
use super::super::types::{HardCodedInfo, KnowledgeResult};
```

**Why these changes:**
- enum_builder now respects `stop_recursion` flag from TypeKnowledge
- Uses same shared helper as path_builder for consistency
- Properly initializes `hard_coded` field in MutationPathInternal
- If TreatAsRootValue knowledge found, returns early without processing variants

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
) -> MutationPathInternal  // Note: returns MutationPathInternal, not MutationPath
```

Update MutationPathInternal construction (around line 450):
```rust
MutationPathInternal {
    mutation_path: ctx.mutation_path.clone(),
    example,
    type_name: ctx.type_name().display_name(),
    path_kind: ctx.path_kind.clone(),
    mutability: status,
    mutability_reason,
    enum_path_data,
    depth: *ctx.depth,
    partial_root_examples,
    hard_coded,  // NEW FIELD - store parameter in struct
}
```

**Note**: PathInfo is constructed separately in `mutation_path_internal.rs` (see STEP 10a below).

### STEP 10a: Update PathInfo Construction in into_mutation_path_external()
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs`

Update the PathInfo construction in the `into_mutation_path_external()` method to include the `hard_coded` field:

```rust
MutationPathExternal {
    description,
    path_info: PathInfo {
        path_kind: self.path_kind,
        type_name: self.type_name,
        type_kind,
        mutability: self.mutability,
        mutability_reason: self.mutability_reason,
        enum_instructions,
        applicable_variants,
        root_example,
        hard_coded: self.hard_coded.clone(),  // NEW FIELD - transfer from MutationPathInternal
    },
    path_example,
}
```

This transfers the `hard_coded` info from `MutationPathInternal` to the final `PathInfo` in the external representation.

### STEP 11: Update All Call Sites of build_mutation_path_internal()
**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`

Update exactly **3 call sites** to add `None` for the new `hard_coded` parameter:

1. **Line 522**: `build_final_result()` - PathAction::Create branch
2. **Line 536**: `build_final_result()` - PathAction::Skip branch
3. **Line 555**: `build_not_mutable_path()`

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

1. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` - Add HardCodedInfo struct, add KnowledgeResult struct, add hard_coded field to PathInfo
2. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_path_internal.rs` - Add hard_coded field to MutationPathInternal, add import for HardCodedInfo, update PathInfo construction in into_mutation_path_external()
3. `mcp/src/brp_tools/brp_type_guide/type_knowledge.rs` - Add TreatAsRootValueWithOriginal variant, add constructor, update example() and get_simplified_name() methods, update all 10 color variant entries
4. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs` - Update find_knowledge() to return Result<Option<KnowledgeResult>, BuilderError> instead of Result<Option<&'static TypeKnowledge>, BuilderError>
5. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/support.rs` - Add create_path_from_knowledge() shared helper function for creating MutationPathInternal from KnowledgeResult
6. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs` - Add imports for HardCodedInfo and KnowledgeResult, simplify check_knowledge() to use KnowledgeResult and shared helper, update build_mutation_path_internal() signature, update 4 call sites
7. `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_builder/enum_path_builder.rs` - Add imports, update to use KnowledgeResult, respect stop_recursion flag, use shared helper from support.rs, update build_enum_root_path() signature and call

## Implementation Notes

- **Shared Abstraction**: Both path_builder and enum_builder use the same pattern for TypeKnowledge handling:
  - Call `ctx.find_knowledge()` which returns `Result<Option<KnowledgeResult>, BuilderError>`
  - Check `knowledge_result.stop_recursion` flag to determine behavior
  - Use `support::create_path_from_knowledge()` helper for consistent MutationPathInternal creation
  - This ensures both call sites handle knowledge identically and eliminates duplicate logic
- **Clone Cost**: The original_value and note are cloned from static TypeKnowledge when building paths. This is acceptable because:
  - Only 10 color variants use this feature
  - Values are small (JSON objects with 4 fields each)
  - Static storage means one-time memory cost in HashMap
  - Extraction happens once during path building via KnowledgeResult
- **API Simplification**: KnowledgeResult provides a clean API that replaces the cryptic tuple return pattern in check_knowledge(), making the control flow clear and self-documenting
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
