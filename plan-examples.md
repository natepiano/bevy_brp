# Plan: Fix Enum Examples Structure

## Design Review Skip Notes

### DESIGN-1: Plan-introduced duplication in assemble_from_children implementations - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Builder Changes - Concrete Implementations
- **Issue**: The plan creates 8 nearly identical assemble_from_children implementations across different builders, all following the same pattern: iterate children, call concrete_value(), build JSON structure. This violates DRY principle and creates maintenance burden.
- **Reasoning**: Investigation found this is appropriate architectural consistency, not problematic duplication. Each builder has genuinely different requirements (Structs build objects from named fields, Tuples handle ordered elements with special unwrapping, Maps convert keys and check complexity, etc.). The structural similarity reflects good trait design where each type implements a common interface while handling its specific requirements.
- **Decision**: User elected to skip this recommendation

### DESIGN-2: Complex state management with EnumContext tracking through recursion - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: EnumContext Addition
- **Issue**: The EnumContext tracking with variant_chain becomes complex to manage through deep recursion. The nested enum with Root/Child variants and variant_chain Vec adds cognitive overhead and potential for state inconsistencies.
- **Reasoning**: While adding EnumContext to RecursionContext does add complexity, this is ESSENTIAL complexity required for correct enum handling. The EnumContext is necessary to: (1) Signal when to return examples array vs concrete value, (2) Track which variants apply to child paths (e.g., .mode.0 only for "Special" variant), (3) Handle nested enum chains properly (e.g., "Nested.Conditional"). Without this state tracking, the enum builder cannot determine the correct output format. The investigation incorrectly claimed this doesn't exist when it's a core part of the plan.
- **Critical Note**: This complexity is unavoidable - the Case Analysis demonstrates that EnumContext must exist for the system to work correctly
- **Decision**: User confirmed this is essential complexity that must exist

### DESIGN-3: Inconsistent API where builders return MutationExample but immediately need conversion - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Keep Original assemble_from_children Signature
- **Issue**: The API design requires builders to return MutationExample but then immediately call concrete_value() in most cases. This suggests the abstraction might be at the wrong level - builders could return the appropriate type directly.
- **Existing Implementation**: The plan has already been rewritten to address this issue. MutationExample is now an internal implementation detail of the enum builder only. All other builders continue using their existing Value signatures (see "Section: Keep Original assemble_from_children Signature").
- **Plan Section**: Section 3. Builder Changes clearly states "All builders EXCEPT enum builder keep their current signature"
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### TYPE-SYSTEM-1: Magic string field names violate type safety - **Verdict**: MODIFIED
- **Status**: SKIPPED
- **Location**: Section: new_enum_builder.rs Implementation
- **Issue**: Plan uses magic strings 'enum_root_data', 'examples', 'default' for JSON communication between enum builder and ProtocolEnforcer, violating 'No Magic Values' principle.
- **Reasoning**: While the finding identified a valid type safety concern, the magic strings only appear in 2 places total in the plan, creating a simple internal protocol between tightly coupled components. Adding typed structures would introduce more complexity than the problem warrants. This is pragmatic internal communication, not a public API.
- **Decision**: User elected to skip this recommendation

### TYPE-SYSTEM-2: String-based conditional logic instead of pattern matching - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: ProtocolEnforcer Processing
- **Issue**: Plan uses string field checks like builder_output.get('enum_root_data') instead of type-safe pattern matching. This creates fragile conditional chains that should use enums and pattern matching.
- **Reasoning**: This was a false positive. The plan IS consistent with using pattern matching on EnumContext throughout (lines 377, 559, 647). The string fields are only used for temporary data serialization between properly typed pattern-matching components. The plan correctly eliminates __variant_context magic fields and replaces them with structured EnumContext enum pattern matching.
- **Decision**: User confirmed the plan is already consistent with type-safe design

### DESIGN-5: Plan-introduced duplication of VariantSignature definition - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Internal MutationExample Enum for Enum Builder Only
- **Issue**: VariantSignature enum is defined identically in multiple files, creating duplication that violates DRY principle and creates maintenance burden.
- **Existing Implementation**: The plan already addresses this by moving VariantSignature to the shared types.rs module (see "Section: Update types.rs with shared types"). The old enum_builder.rs file will be completely deleted as part of the migration, eliminating the duplication automatically.
- **Plan Section**: Phase 1 step 2: "Move VariantSignature from new_enum_builder.rs to types.rs (needed for public API)"
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### DESIGN-6: JSON-based communication protocol creates fragile coupling between components - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: ProtocolEnforcer Processing
- **Issue**: Plan creates JSON-based protocol between enum builder and ProtocolEnforcer using magic field markers. This is fragile, type-unsafe, and requires string parsing at runtime.
- **Existing Implementation**: The plan already eliminates magic JSON fields and replaces them with proper typed communication via EnumContext enum and pattern matching (see "Section: EnumContext Addition" and pattern matching usage throughout lines 377, 559, 647). The plan explicitly calls out eliminating "__variant_context" magic fields as a core problem to solve.
- **Plan Section**: Section: Problem Statement lists "JSON encoding hacks: Using __variant_context and magic field names in JSON" as an issue being fixed
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

### QUALITY-1: Unnecessary complexity in ExampleGroup serialization - **Verdict**: CONFIRMED - REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Internal MutationExample Enum for Enum Builder Only
- **Issue**: Plan removes derive(Serialize, Deserialize) from ExampleGroup and implements custom serialization just to call Display on VariantSignature. This adds unnecessary complexity for minimal benefit.
- **Existing Implementation**: The plan already uses the cleaner serialize_with approach in "Section: Update types.rs with shared types". The obsolete custom Serialize implementation in "Section: Internal MutationExample Enum for Enum Builder Only" has been removed to eliminate duplication.
- **Plan Section**: Section: Update types.rs with shared types shows the proper serialize_with implementation
- **Critical Note**: This functionality/design already exists in the plan - future reviewers should check for existing coverage before suggesting

## Problem Statement

The current system has multiple issues:
1. **Inconsistent field naming**: `"variants"` internally vs `"applicable_variants"` in the API
2. **JSON encoding hacks**: Using `__variant_context` and magic field names in JSON
3. **Tight coupling**: Enum builder output is tightly coupled to conversion logic through JSON structure
4. **Bug in old implementation**: When an enum is a field in a struct, the root example incorrectly shows an array of examples instead of a concrete value
5. **Bug in new implementation**: Not returning the proper examples structure for enum root paths

## Current Flow

1. **Old enum_builder.rs**:
   - Builds JSON with `"variants"` field
   - Uses `__variant_context` wrapper for child paths
   - Returns array of signature groups for root path

2. **Conversion in types.rs**:
   - Parses JSON looking for `"variants"` field
   - Extracts `__variant_context` for applicable variants
   - Creates `ExampleGroup` with `applicable_variants`

3. **Output Structure**:
   - Root enum path: `examples` array with `applicable_variants`, `signature`, `example`
   - Child paths (.0, .1, etc.): Single `example` with `variants` field listing applicable variants
   - Embedded in struct: Should be single concrete value, but currently broken (shows array)

## Proposed Solution

### 1. Internal MutationExample Enum for Enum Builder Only

Keep the `example: Value` field in `MutationPathInternal` unchanged. Instead, use `MutationExample` as an internal implementation detail within the enum builder only:

```rust
// In new_enum_builder.rs ONLY - not exported to other modules
use std::fmt;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

/// Internal example data for enum mutation paths - used only within enum builder
/// Other builders continue using Value directly
#[derive(Debug, Clone)]
enum MutationExample {
    /// Simple example value (for non-enum types and embedded enum values)
    Simple(Value),

    /// Multiple examples with signatures (for enum root paths)
    /// Each group has applicable_variants, signature, and example
    EnumRoot(Vec<ExampleGroup>),

    /// Example with variant context (for enum child paths like .0, .1, .enabled)
    EnumChild {
        example: Value,
        applicable_variants: Vec<String>,
    },
}

/// Example group for enum variants (should be called VariantExampleGroup but keeping for compatibility)
#[derive(Debug, Clone)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<String>,

    /// The variant signature type (stored as enum for type safety)
    pub signature: VariantSignature,

    /// Example value for this group
    pub example: Value,
}

/// Variant signature types for enum variants
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariantSignature {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with ordered types
    Tuple(Vec<BrpTypeName>),
    /// Struct variant with named fields
    Struct(Vec<(String, BrpTypeName)>),
}

impl fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariantSignature::Unit => write!(f, "unit"),
            VariantSignature::Tuple(types) => {
                let type_names: Vec<String> = types
                    .iter()
                    .map(|t| shorten_type_name(t.as_str()))
                    .collect();
                write!(f, "tuple({})", type_names.join(", "))
            }
            VariantSignature::Struct(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, type_name)| {
                        format!("{}: {}", name, shorten_type_name(type_name.as_str()))
                    })
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
}
```

The `MutationExample` enum belongs in `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/new_enum_builder.rs` as an internal implementation detail. The `ExampleGroup` and `VariantSignature` structs need to be in `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` since they're used in the public API.

#### EnumContext Addition

Add to `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`:

```rust
/// Tracks enum-specific context during recursion
#[derive(Debug, Clone)]
pub enum EnumContext {
    /// This enum is establishing the root context
    Root,

    /// Building under enum variant(s)
    Child {
        /// Chain of variant constraints from parent to child
        /// e.g., [(TestEnumWithSerDe, ["Nested"]), (NestedConfigEnum, ["Conditional"])]
        variant_chain: Vec<(BrpTypeName, Vec<String>)>,
    },
}

pub struct RecursionContext {
    // ... existing fields ...
    /// Track enum context - None for non-enum types
    pub enum_context: Option<EnumContext>,
}
```

The enum builder determines what to return based on context:
- **Return `Simple`**: When `ctx.enum_context` is `None` (non-enum parent needs concrete value)
- **Return `EnumRoot`**: When `ctx.enum_context` is `Some(EnumContext::Root)` (building the enum's root path)
- **Return `EnumChild`**: When `ctx.enum_context` is `Some(EnumContext::Child { ... })` (building under enum variant)

#### RecursionContext Creation Changes

Need to update these methods to handle `enum_context`:
- `RecursionContext::root()` - starts with `enum_context: None`
- `RecursionContext::create_child_context()` - propagates parent's `enum_context` by default
- ProtocolEnforcer - sets `Some(EnumContext::Root)` when processing an enum type
- Enum builder's child creation - sets `Some(EnumContext::Child { ... })` for its children

### Example Enum Structures

*These examples are used throughout the case analysis:*

```rust
// Main enum with multiple variants including nested enum
enum TestEnumWithSerDe {
    Active,
    Inactive,
    Special(String, u32),
    AlsoSpecial(String, u32),  // Second variant with same signature
    Custom {
        enabled: bool,
        name: String,
        value: f32,
    },
    Nested {
        nested_config: NestedConfigEnum,
        other_field: String,
    }
}

// Nested enum used in the Nested variant
enum NestedConfigEnum {
    Always,
    Never,
    Conditional(u32),
}
```

This structure demonstrates:
- Unit variants (Active, Inactive)
- Multiple tuple variants with same signature (Special, AlsoSpecial)
- Struct variant (Custom)
- Nested enum (Nested contains NestedConfigEnum)

### Case Analysis for Enum Handling

*This section to be included in the module documentation for new_enum_builder.rs*

#### Case 1: Enum at root (e.g., TestEnumWithSerDe itself)

**Context:**
- Building mutation paths for an enum as the top-level type
- Path: `""`
- PathKind: `RootValue`
- Type: `TestEnumWithSerDe` (TypeKind::Enum)
- EnumContext: `Some(EnumContext::Root)` (set by ProtocolEnforcer when it sees TypeKind::Enum)

**Key Insight:** The enum builder knows it's TypeKind::Enum, which determines its behavior.

**Flow:**
1. `collect_children()` returns `Vec<PathKind>` with:
   ```rust
   PathKind::EnumVariant {
       signature: VariantSignature::Unit,
       parent_type: TestEnumWithSerDe,
       applicable_variants: vec!["Active", "Inactive"],
   },
   PathKind::EnumVariant {
       signature: VariantSignature::Tuple(vec![String, u32]),
       parent_type: TestEnumWithSerDe,
       applicable_variants: vec!["Special"],
   },
   PathKind::EnumVariant {
       signature: VariantSignature::Struct(vec![
           ("enabled", bool),
           ("name", String),
           ("value", f32),
       ]),
       parent_type: TestEnumWithSerDe,
       applicable_variants: vec!["Custom"],
   },
   ```

2. ProtocolEnforcer processes these `PathKind::EnumVariant` entries:
   - For Unit: No child paths created
   - For Tuple: Creates `.0`, `.1` paths with `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Special", "AlsoSpecial"])] })`
   - For Struct: Creates `.enabled`, `.name`, `.value` paths with `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Custom"])] })`
   - **This is where EnumContext changes from `Some(EnumContext::Root)` to `Some(EnumContext::Child { ... })`**

3. `assemble_from_children()` receives the built child examples and:
   - Checks: `EnumContext == Some(EnumContext::Root)`
   - Returns `MutationExample::EnumRoot(Vec<ExampleGroup>)`
   - Transforms child examples into the proper examples array structure

**Result:**
- Root path `""` gets `examples` array with all variant groups
- Child paths `.0`, `.1`, `.enabled`, etc. have `applicable_variants` from their EnumContext

#### Case 2: Enum as a field (e.g., .mode in TestComplexComponent)

**Context:**
- Building paths for TestComplexComponent's enum field
- Path: `.mode`
- PathKind: `StructField { field_name: "mode", type_name: TestEnumWithSerDe, parent_type: TestComplexComponent }`
- Type: `TestEnumWithSerDe` (TypeKind::Enum)
- EnumContext: `Some(EnumContext::Root)` (set by ProtocolEnforcer when it sees TypeKind::Enum)

**Key Rule:** Any enum type with `EnumContext == Some(EnumContext::Root)` returns `MutationExample::EnumRoot`.

**Flow:**
1. Struct builder delegates to enum builder for the `mode` field
2. ProtocolEnforcer sets `Some(EnumContext::Root)` for this enum field
3. Enum's `collect_children()` returns `PathKind::EnumVariant` entries
4. ProtocolEnforcer creates `.mode.0`, `.mode.1`, `.mode.enabled` with `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Special", "AlsoSpecial"])] })` or `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Custom"])] })`
5. Enum's `assemble_from_children()`:
   - Sees `EnumContext == Some(EnumContext::Root)`
   - Returns `MutationExample::EnumRoot(Vec<ExampleGroup>)`

**Result:** `.mode` gets full `examples` array just like a root enum.

#### Case 3: Enum child paths (e.g., .mode.0, .mode.enabled)

**Context:**
- Building paths for fields within enum variants
- Paths: `.mode.0`, `.mode.1` (tuple variant) or `.mode.enabled`, `.mode.name` (struct variant)
- PathKind: Created from `PathKind::EnumVariant` expansion (specific types TBD based on ProtocolEnforcer logic)
- Types: `String`, `u32`, `bool`, etc. (the field types)
- EnumContext: `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, [...])] })`

**Key Points:**
- These are NOT enum types - they're fields inside enum variants
- The `Some(EnumContext::Child { ... })` stays constant throughout this recursion branch because all paths under a variant apply to the same set of variants

**Flow:**
1. These paths were created by ProtocolEnforcer when it processed `PathKind::EnumVariant`
2. The appropriate builder for the field type is called (string, bool, etc.)
3. These builders see `Some(EnumContext::Child { ... })` and preserve it (it stays the same for the entire branch)
4. When assembling:
   - The `example` value changes as we recurse back up (field examples get assembled into parent structures)
   - But `applicable_variants` stays constant for this branch
   - Non-enum builders return `MutationExample::Simple(value)`
   - If the field itself was an enum, it would return `MutationExample::EnumChild { example, applicable_variants }`

**Result:**
- `.mode.0` gets example `"Hello, World!"` with `variants: ["Special"]`
- `.mode.enabled` gets example `true` with `variants: ["Custom"]`
- The `applicable_variants` information comes from the EnumContext, not the example

#### Case 4: Nested enum under another enum

**Context:**
- An enum field inside an enum variant (the `nested_config` field in the `Nested` variant)
- Path: `.mode.nested_config`
- PathKind: Created from parent enum's variant processing
- Type: `NestedConfigEnum` (TypeKind::Enum)
- EnumContext: `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Nested"])] })`

**Key Insight:** Nested enums track the full chain of variant constraints using dot notation in output.

**Flow:**
1. `.mode.nested_config` is processed:
   - ProtocolEnforcer sees TypeKind::Enum
   - Context has parent constraint in variant_chain
   - Sets `Some(EnumContext::Root)` for the nested enum itself (to get examples array)
   - But preserves parent constraints for its children

2. Nested enum's `collect_children()` returns its own `PathKind::EnumVariant` entries

3. For `.mode.nested_config.0` (the u32 in Conditional):
   - Context becomes `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Nested"]), (NestedConfigEnum, ["Conditional"])] })`

**Result:**
- `.mode.nested_config` gets its own `examples` array with `applicable_variants: ["Nested"]`
- `.mode.nested_config.0` gets `applicable_variants: ["Nested.Conditional"]` (flattened with dot notation)
- The conversion flattens the chain: `["Nested", "Conditional"]` → `"Nested.Conditional"`

#### Case 5: Providing value for parent assembly

**Context:**
- A parent struct/array/tuple needs a concrete enum value for its root path assembly
- Example: TestComplexComponent needs a value for its `mode` field when building root `""`
- EnumContext: `None` (no enum context established)

**Key Insight:** When enum's `assemble_from_children` sees `enum_context: None`, it knows a non-enum parent needs a concrete value.

**Flow:**
1. TestComplexComponent's struct builder assembles its root path
2. It calls enum's `assemble_from_children` for the `mode` field with `enum_context: None`
3. Enum's `assemble_from_children` logic:
   ```rust
   match ctx.enum_context {
       None => MutationExample::Simple(pick_concrete_value()),  // Parent needs a value
       Some(EnumContext::Root) => MutationExample::EnumRoot(examples),
       Some(EnumContext::Child { .. }) => MutationExample::EnumChild { .. },
   }
   ```

**Result:**
- TestComplexComponent's root gets `mode: "Active"` (concrete value)
- Clean separation: `None` means "give me something concrete to embed"
- The `.mode` path itself still gets its full `examples` array when processed as a field


### 2. Update types.rs with shared types

```rust
// In types.rs - move VariantSignature here for public API use
/// Variant signature types for enum variants - used for grouping similar structures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantSignature {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with ordered types
    Tuple(Vec<BrpTypeName>),
    /// Struct variant with named fields and types
    Struct(Vec<(String, BrpTypeName)>),
}

impl std::fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Tuple(types) => {
                let type_names: Vec<String> = types.iter().map(|t| shorten_type_name(t.as_str())).collect();
                write!(f, "tuple({})", type_names.join(", "))
            }
            Self::Struct(fields) => {
                let field_strs: Vec<String> = fields.iter()
                    .map(|(name, type_name)| format!("{}: {}", name, shorten_type_name(type_name.as_str())))
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<String>,

    /// The variant signature type (serialized as string using Display)
    #[serde(serialize_with = "serialize_signature")]
    pub signature: VariantSignature,

    /// Example value for this group
    pub example: Value,
}

fn serialize_signature<S>(sig: &VariantSignature, s: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    s.serialize_str(&sig.to_string())
}
```

### 3. Update MutationPathInternal

```rust
pub struct MutationPathInternal {
    /// Example data for this path - for enum children includes applicable_variants
    pub example: Value,

    /// For enum roots only: the examples array with all variant groups
    /// None for all other paths (including enum children and regular types)
    pub enum_root_examples: Option<Vec<ExampleGroup>>,

    // ... other fields remain the same
}
```

This explicit field eliminates JSON parsing in conversion - data transfers directly to `MutationPath`.



### 4. Builder Changes

#### Keep Original assemble_from_children Signature

All builders EXCEPT enum builder keep their current signature:
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,  // Stays as Value
) -> Result<Value>  // Stays as Result<Value>
```

Only the enum builder needs special handling internally.

#### new_enum_builder.rs Implementation

**Note:** This implementation extracts helper functions `build_variant_example()`, `concrete_example()`, and `flatten_variant_chain()` to keep the code clean and maintainable. The `format_signature()` function is replaced by the `Display` trait implementation on `VariantSignature`.

```rust
impl NewEnumMutationBuilder {
    /// Build a complete example for a variant with all its fields
    fn build_variant_example(
        &self,
        signature: &VariantSignature,
        variant_name: &str,
        children: &HashMap<MutationPathDescriptor, Value>,  // Takes Value, not MutationExample
    ) -> Value {
        match signature {
            VariantSignature::Unit => {
                json!(variant_name)
            }
            VariantSignature::Tuple(types) => {
                let mut tuple_values = Vec::new();
                for index in 0..types.len() {
                    let descriptor = MutationPathDescriptor::from(index.to_string());
                    let value = children.get(&descriptor)
                        .cloned()  // Just clone the Value directly
                        .unwrap_or(json!(null));
                    tuple_values.push(value);
                }
                json!({ variant_name: tuple_values })
            }
            VariantSignature::Struct(field_types) => {
                let mut field_values = serde_json::Map::new();
                for (field_name, _) in field_types {
                    let descriptor = MutationPathDescriptor::from(field_name.clone());
                    let value = children.get(&descriptor)
                        .cloned()  // Just clone the Value directly
                        .unwrap_or(json!(null));
                    field_values.insert(field_name.clone(), value);
                }
                json!({ variant_name: field_values })
            }
        }
    }

    /// Create a concrete example value for embedding in a parent structure
    fn concrete_example(
        &self,
        variant_groups: &[(VariantSignature, Vec<EnumVariant>)],
        children: &HashMap<MutationPathDescriptor, Value>,  // Takes Value, not MutationExample
    ) -> Value {
        // Pick first unit variant if available, otherwise first example
        let unit_variant = variant_groups.iter()
            .find(|(sig, _)| matches!(sig, VariantSignature::Unit))
            .and_then(|(_, variants)| variants.first());

        if let Some(variant) = unit_variant {
            return json!(variant.name());
        }

        // Fall back to first available example with full structure
        variant_groups.iter()
            .next()
            .map(|(sig, variants)| {
                let representative = variants.first().unwrap();
                self.build_variant_example(sig, representative.name(), children)
            })
            .unwrap_or(json!(null))
    }

    /// Separator used for flattening nested enum variant chains into dot notation
    /// e.g., "Nested.Conditional" for nested enum variants
    const VARIANT_PATH_SEPARATOR: &str = ".";

    /// Flatten variant chain into dot notation for nested enums
    fn flatten_variant_chain(variant_chain: &[(BrpTypeName, Vec<String>)]) -> Vec<String> {
        // e.g., [(TestEnum, ["Nested"]), (NestedEnum, ["Conditional"])] → ["Nested.Conditional"]
        if variant_chain.is_empty() {
            return vec![];
        }

        // Collect all the variant names from each level
        let mut result = Vec::new();
        for (_, variants) in variant_chain {
            for variant in variants {
                // Build the full path by collecting all parent variants
                let mut path_parts = Vec::new();
                for (parent_idx, (_, parent_variants)) in variant_chain.iter().enumerate() {
                    if parent_idx < variant_chain.len() - 1 {
                        // Add the first variant from each parent level
                        if let Some(parent_variant) = parent_variants.first() {
                            path_parts.push(parent_variant.clone());
                        }
                    } else {
                        // At the current level, add the actual variant
                        path_parts.push(variant.clone());
                    }
                }

                if !path_parts.is_empty() {
                    result.push(path_parts.join(VARIANT_PATH_SEPARATOR));
                }
            }
        }

        // Only return the variants from the last level in the chain
        if let Some((_, last_variants)) = variant_chain.last() {
            let prefix_parts: Vec<String> = variant_chain.iter()
                .take(variant_chain.len() - 1)
                .filter_map(|(_, v)| v.first().cloned())
                .collect();

            if prefix_parts.is_empty() {
                last_variants.clone()
            } else {
                last_variants.iter()
                    .map(|v| {
                        let mut full_path = prefix_parts.clone();
                        full_path.push(v.clone());
                        full_path.join(VARIANT_PATH_SEPARATOR)
                    })
                    .collect()
            }
        } else {
            result
        }
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,  // Takes Value like all other builders
    ) -> Result<Value> {  // Returns Value like all other builders
        let schema = ctx.require_registry_schema()?;
        let all_variants = extract_enum_variants(schema, &ctx.registry);
        let variant_groups = group_variants_by_signature(all_variants);

        // Build internal MutationExample to organize the enum logic
        let mutation_example = match ctx.enum_context {
            Some(EnumContext::Root) => {
                // Build examples array for enum root path
                let mut examples = Vec::new();

                for (signature, variants_in_group) in &variant_groups {
                    let representative = variants_in_group.first().ok_or_else(|| {
                        Report::new(Error::InvalidState("Empty variant group".to_string()))
                    })?;

                    let example = self.build_variant_example(
                        signature,
                        representative.name(),
                        &children
                    );

                    let applicable_variants: Vec<String> = variants_in_group
                        .iter()
                        .map(|v| v.name().to_string())
                        .collect();

                    examples.push(ExampleGroup {
                        applicable_variants,
                        signature: signature.clone(),
                        example,
                    });
                }

                MutationExample::EnumRoot(examples)
            }

            Some(EnumContext::Child { ref variant_chain }) => {
                // Building under another enum - return EnumChild
                let example = self.concrete_example(&variant_groups, &children);
                let applicable_variants = Self::flatten_variant_chain(variant_chain);

                MutationExample::EnumChild {
                    example,
                    applicable_variants,
                }
            }

            None => {
                // Parent is not an enum - return a concrete example
                let example = self.concrete_example(&variant_groups, &children);
                MutationExample::Simple(example)
            }
        };

        // Convert MutationExample to Value for ProtocolEnforcer to process
        match mutation_example {
            MutationExample::Simple(val) => Ok(val),
            MutationExample::EnumRoot(examples) => {
                // For enum roots, return both examples array and a default concrete value
                // ProtocolEnforcer will extract these to populate MutationPathInternal fields
                let default_example = examples.first()
                    .and_then(|g| g.get("example"))
                    .cloned()
                    .unwrap_or(json!(null));

                Ok(json!({
                    "enum_root_data": {
                        "examples": examples,
                        "default": default_example
                    }
                }))
            }
            MutationExample::EnumChild { example, applicable_variants } => {
                // For enum children, just return the example
                // ProtocolEnforcer will wrap it based on EnumContext
                Ok(example)
            }
        }
    }
}
```

#### ProtocolEnforcer Processing

The ProtocolEnforcer creates `MutationPathInternal` instances based on builder output and `EnumContext`:

```rust
impl ProtocolEnforcer {
    fn create_mutation_path_internal(
        &self,
        ctx: &RecursionContext,
        builder_output: Value,
    ) -> MutationPathInternal {
        let (example, enum_root_examples) = match ctx.enum_context {
            Some(EnumContext::Root) => {
                // For enum roots, extract both fields from structured output
                if let Some(enum_data) = builder_output.get("enum_root_data") {
                    let default_example = enum_data.get("default").cloned().unwrap_or(json!(null));
                    let examples_json = enum_data.get("examples").cloned().unwrap_or(json!([]));
                    let examples: Vec<ExampleGroup> = serde_json::from_value(examples_json).unwrap_or_default();
                    (default_example, Some(examples))
                } else {
                    // Fallback if structure is unexpected
                    (builder_output, None)
                }
            }
            Some(EnumContext::Child { ref variant_chain }) => {
                // For enum children, wrap the output with applicable_variants
                let applicable_variants = flatten_variant_chain(variant_chain);
                let wrapped = json!({
                    "example": builder_output,
                    "applicable_variants": applicable_variants
                });
                (wrapped, None)
            }
            None => {
                // Regular values pass through unchanged
                (builder_output, None)
            }
        };

        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            enum_root_examples,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutable,
            mutation_status_reason: None,
        }
    }
}
```

#### Other builders - No Changes Needed

All non-enum builders continue using their current signatures and implementations. They don't need to know about `MutationExample` at all:

##### 1. StructBuilder

```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,  // Already uses Value
) -> Result<Value> {  // Already returns Value
    // Current implementation stays exactly the same
    if children.is_empty() {
        return Ok(json!({}));
    }
    let mut result = serde_json::Map::new();
    for (field_name, value) in children {
        result.insert(field_name.into(), value);
    }
    Ok(json!(result))
}
```

##### 2. TupleBuilder, ArrayBuilder, ListBuilder, MapBuilder, OptionBuilder, PrimitiveBuilder, UnitBuilder

All these builders keep their existing implementations unchanged. They continue to:
- Take `HashMap<MutationPathDescriptor, Value>` as input
- Return `Result<Value>` as output
- Work directly with `Value` objects without needing to know about `MutationExample`

Example (TupleBuilder):
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value> {
    // Existing implementation unchanged
    let mut tuple_values = Vec::new();
    for index in 0..self.tuple_types.len() {
        let descriptor = MutationPathDescriptor::from(index.to_string());
        let value = children.get(&descriptor).cloned().unwrap_or(json!(null));
        tuple_values.push(value);
    }
    Ok(json!(tuple_values))
}
```

The same pattern applies to all other non-enum builders. They don't need any changes.

#### Internal MutationExample Helper Method (within new_enum_builder.rs only)

```rust
// This is INTERNAL to new_enum_builder.rs, not exposed outside
impl MutationExample {
    /// Extract a concrete value suitable for embedding in a parent
    fn concrete_value(&self) -> Value {
        match self {
            MutationExample::Simple(val) => val.clone(),
            MutationExample::EnumRoot(groups) => {
                // Type-safe: Pick first unit variant, or first example
                groups.iter()
                    .find(|g| matches!(g.signature, VariantSignature::Unit))
                    .or_else(|| groups.first())
                    .map(|g| g.example.clone())
                    .unwrap_or(json!(null))
            }
            MutationExample::EnumChild { example, .. } => example.clone(),
        }
    }
}



### 5. Simplified Conversion Logic

The conversion from `MutationPathInternal` to `MutationPath` is now trivial - just direct field transfer:

```rust
impl MutationPath {
    pub fn from_mutation_path_internal(
        path: &MutationPathInternal,
        registry: &HashMap<BrpTypeName, Value>
    ) -> Self {
        // Direct transfer - no JSON parsing needed!
        let (examples, example) = if let Some(ref enum_examples) = path.enum_root_examples {
            // Enum root: use the examples array
            (enum_examples.clone(), None)
        } else {
            // Everything else: use the example value
            // This includes enum children (with embedded applicable_variants) and regular values
            (vec![], Some(path.example.clone()))
        };

        Self {
            description: path.path_kind.description(&path.type_kind),
            path_info: PathInfo {
                path: path.path.clone(),
                type_name: path.type_name.clone(),
                type_kind: path.type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
            },
            // Note: variants field has been DEPRECATED - removed from MutationPath
            examples,
            example,
            note: None,
        }
    }
}
```

**Key improvements:**
- No JSON parsing or magic markers
- Direct field transfer from `MutationPathInternal` to `MutationPath`
- `variants` field removed entirely - information is in `examples` or embedded in `example`
- Clean separation of concerns

### 6. Migration Strategy

Clean migration with explicit data flow:

**Phase 1: Update data structures**
1. Add `enum_root_examples: Option<Vec<ExampleGroup>>` to `MutationPathInternal` in types.rs
2. Move `VariantSignature` from new_enum_builder.rs to types.rs (needed for public API)
3. Update `ExampleGroup` to use typed `VariantSignature` instead of string in types.rs
4. Add `EnumContext` to recursion_context.rs
5. Remove `variants` field from `MutationPath` (deprecated)

**Phase 2: Update enum builder**
1. Add internal `MutationExample` enum to new_enum_builder.rs (not exported)
2. Update `NewEnumMutationBuilder::assemble_from_children` to:
   - Build internal `MutationExample` for organization
   - Return structured data for ProtocolEnforcer to process
3. Keep all other builders unchanged

**Phase 3: Update ProtocolEnforcer**
1. Add `create_mutation_path_internal` method to handle:
   - Extracting enum root examples from structured output
   - Wrapping enum children with applicable_variants
   - Passing through regular values unchanged
2. Set `enum_root_examples` field when processing enum roots

**Phase 4: Update conversion logic**
1. Simplify `from_mutation_path_internal` in types.rs to:
   - Direct field transfer from `MutationPathInternal` to `MutationPath`
   - No JSON parsing or magic markers
   - Remove `variants` field from output

**Phase 5: Clean up and activate**
1. Remove `USE_NEW_ENUM_BUILDER` flag from type_kind.rs
2. Remove old `enum_builder` from builders/mod.rs
3. Test with existing test suite to ensure compatibility


### 7. Testing Strategy

- Use @get_guide.md to fetch baseline outputs for all test types
- Compare new output structure to ensure:
  - Root enum paths have proper `examples` array
  - Child paths have `variants` field (now `applicable_variants`)
  - Embedded enums show single concrete value (fix the bug)
- Run mutation tests to verify BRP operations still work

## Benefits

1. **Localized complexity**: Enum handling complexity is contained within the enum builder only
   - Other builders remain unchanged and simple
   - No API disruption for non-enum types
   - `MutationExample` is an internal implementation detail
2. **Type safety within enum builder**: Internal use of proper types instead of JSON hacks
   - `VariantSignature` enum for variant type classification
   - `ExampleGroup` for organizing variant examples
   - Clear separation of concerns
3. **Minimal migration risk**: Changes are isolated to enum-related code
   - No changes to other builders
   - Simple marker-based communication with conversion layer
   - Existing tests continue to work
4. **Correctness**: Fixes both bugs while maintaining simplicity
   - Embedded enum arrays fixed
   - Missing examples structure fixed
   - No over-engineering of the solution
5. **Maintainability**: Future enum enhancements stay within enum builder
   - Adding new enum features doesn't affect other builders
   - Clear boundary between enum and non-enum handling

## Risks and Mitigations

- **Risk**: Marker-based communication could be fragile
  - **Mitigation**: Use unique marker names like `__enum_root_marker` that won't collide
  - **Mitigation**: Keep markers as internal implementation details

- **Risk**: Missing some edge case in conversion
  - **Mitigation**: Comprehensive testing against baseline outputs
  - **Mitigation**: Conversion logic handles both old and new formats during transition

- **Risk**: Performance impact from internal enum construction
  - **Mitigation**: Minimal overhead - enum is built and immediately converted
  - **Mitigation**: Still more efficient than current JSON parsing approach

## Implementation Order

1. **Phase 1: Add internal types to enum builder**
   - Add `MutationExample` enum as private type in new_enum_builder.rs
   - Add helper methods for variant example building
   - Keep the public interface unchanged (still returns `Value`)

2. **Phase 2: Add public types to types.rs**
   - Add `ExampleGroup` struct (needed for API)
   - Add `VariantSignature` enum with `Display` trait
   - Add `EnumContext` to recursion_context.rs

3. **Phase 3: Update enum builder implementation**
   - Modify `assemble_from_children` to use internal `MutationExample`
   - Convert to `Value` with markers before returning
   - Test enum builder in isolation

4. **Phase 4: Update conversion logic**
   - Modify `from_mutation_path_internal` to recognize markers
   - Handle both old and new formats for compatibility
   - Verify with existing tests

5. **Phase 5: Activate and cleanup**
   - Remove `USE_NEW_ENUM_BUILDER` flag from type_kind.rs
   - Remove old enum_builder.rs file
   - Run full test suite to ensure no regressions
